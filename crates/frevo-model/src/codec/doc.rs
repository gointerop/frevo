//! The stored buffer and the lazily built index over it.
//!
//! `design.md` section 3: `Arc<[u8]>` plus, for each object that has been
//! entered, a small map from property name to the span of its value. Entering
//! an object is a single scan of *that* object's members, not of its children.
//!
//! The index here is built **incrementally**, not on first entry: a cursor
//! remembers where it stopped and resumes from there. That is what makes
//! reading `Patient.gender` from a document whose 50 KiB `text.div` comes
//! earlier cost one token for the narrative rather than a scan of it
//! (`model.lazy.006`), and what makes `Patient.birthDate` out of a 4 KiB
//! `Patient` never touch the `address` array that follows it
//! (`model.bytes.003`).

use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::scan::{
    DecodeError, Meter, NoRecord, Record, Span, is_null, scan_string, skip_value,
    skip_value_recording, skip_ws, unescape, unescape_borrowed,
};

/// The hasher of the cursor maps, whose keys are byte offsets into one
/// document.
///
/// The default `SipHash` is built to resist an adversary choosing the keys;
/// here the keys are offsets the codec itself produced, all distinct, so a
/// single multiply spreads them well enough, and a full decode, which consults
/// the maps once per object and once per array, stops paying for a keyed hash
/// each time.
#[derive(Debug, Default, Clone, Copy)]
struct OffsetHasher(u64);

impl Hasher for OffsetHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0.rotate_left(8) ^ u64::from(*byte)).wrapping_mul(FIBONACCI);
        }
    }

    fn write_usize(&mut self, value: usize) {
        // Fibonacci hashing: the high bits, which the table takes its control
        // byte from, depend on every bit of the offset.
        self.0 = (value as u64).wrapping_mul(FIBONACCI);
    }
}

/// 2^64 divided by the golden ratio, odd.
const FIBONACCI: u64 = 0x9E37_79B9_7F4A_7C15;

/// The member-list capacity a new object cursor starts with.
const MEMBERS_HINT: usize = 8;

type OffsetMap<V> = HashMap<usize, V, BuildHasherDefault<OffsetHasher>>;

/// An object's members, discovered as far as navigation has needed them.
#[derive(Debug)]
struct ObjectCursor {
    /// `(key text inside its quotes, value)` in document order.
    members: Vec<(Span, Span)>,
    /// Where the next member starts, once the preceding separator is consumed.
    resume: usize,
    done: bool,
}

/// An array's items, discovered as far as navigation has needed them.
#[derive(Debug)]
struct ArrayCursor {
    items: Vec<Span>,
    resume: usize,
    done: bool,
}

/// The structure of the containers one scan stepped over: each object's
/// members and each array's items, and what stepping over it cost.
///
/// A full decode enters every container of the document, and entering one
/// through the index is a scan of it, so a document nested `n` deep is scanned
/// `n` times over -- the root's member scan steps over the `address` array, the
/// array's item scan steps over each `Address` again, and so on. The tape is
/// what the first of those scans learns on the way past, so the decode can
/// enter each container from it instead of scanning it again.
///
/// It changes no observable: entering a container from the tape charges the
/// meter exactly the tokens and the allocations entering it through the index
/// would have, and a container entered from the tape answers later index
/// lookups as the cursor entering it would have left behind.
#[derive(Debug, Default)]
pub struct Tape {
    /// In opening order, which is increasing `start` order.
    nodes: Vec<TapeNode>,
    /// The members and items of every closed container, each container's
    /// contiguous: `(key inside its quotes, value)`, the key empty for an item.
    entries: Vec<(Span, Span)>,
    /// The entries of the containers still open, innermost last.
    pending: Vec<(Span, Span)>,
    /// Where each open container's entries begin in `pending`.
    marks: Vec<usize>,
}

#[derive(Debug, Clone, Copy)]
struct TapeNode {
    start: usize,
    end: usize,
    object: bool,
    first: usize,
    len: usize,
    tokens: u64,
    closed: bool,
    entered: bool,
}

impl Record for Tape {
    fn open(&mut self, start: usize, object: bool) -> usize {
        self.nodes.push(TapeNode {
            start,
            end: start,
            object,
            first: 0,
            len: 0,
            tokens: 0,
            closed: false,
            entered: false,
        });
        self.marks.push(self.pending.len());
        self.nodes.len() - 1
    }

    fn entry(&mut self, key: Span, value: Span) {
        self.pending.push((key, value));
    }

    fn close(&mut self, handle: usize, end: usize, tokens: u64) {
        let mark = self.marks.pop().unwrap_or(0);
        let first = self.entries.len();
        self.entries.extend(self.pending.drain(mark..));
        if let Some(node) = self.nodes.get_mut(handle) {
            node.end = end;
            node.first = first;
            node.len = self.entries.len() - first;
            node.tokens = tokens;
            node.closed = true;
        }
    }
}

impl Tape {
    /// A tape sized for a document of `len` bytes, so recording one does not
    /// grow its buffers by doubling.
    pub fn for_document(len: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(len / 48),
            entries: Vec::with_capacity(len / 12),
            pending: Vec::with_capacity(32),
            marks: Vec::with_capacity(16),
        }
    }

    /// The closed container starting at `start`, if this tape stepped over one.
    pub fn find(&self, start: usize) -> Option<usize> {
        let at = self
            .nodes
            .binary_search_by(|node| node.start.cmp(&start))
            .ok()?;
        self.nodes[at].closed.then_some(at)
    }

    /// Whether node `at` is an object, as opposed to an array.
    pub fn is_object(&self, at: usize) -> bool {
        self.nodes[at].object
    }

    /// Enter node `at` as the index would have: charge the tokens a scan of it
    /// costs and the allocations a cursor over it makes (its member or item
    /// list, plus, for an object, the copy [`Doc::member_spans`] hands out),
    /// and remember that it was entered. Returns its entries' range.
    pub fn enter(&mut self, at: usize, meter: &Meter) -> std::ops::Range<usize> {
        let node = &mut self.nodes[at];
        meter.tokens(node.tokens);
        meter.size_proportional_allocation();
        if node.object {
            meter.size_proportional_allocation();
        }
        node.entered = true;
        node.first..node.first + node.len
    }

    /// Entry `at`: `(key, value)`.
    pub fn entry_at(&self, at: usize) -> (Span, Span) {
        self.entries[at]
    }

    /// The entries of a container entered from this tape, if one starts at
    /// `start` and has the given shape.
    fn entered(&self, start: usize, object: bool) -> Option<(&[(Span, Span)], usize)> {
        let node = &self.nodes[self.find(start)?];
        (node.entered && node.object == object)
            .then(|| (&self.entries[node.first..node.first + node.len], node.end))
    }
}

/// One document's stored bytes and its index.
#[derive(Debug)]
pub struct Doc {
    bytes: Arc<[u8]>,
    meter: Arc<Meter>,
    objects: Mutex<OffsetMap<ObjectCursor>>,
    arrays: Mutex<OffsetMap<ArrayCursor>>,
    /// Tapes a full decode entered containers from, consulted when the index
    /// has no cursor for a container (see [`Tape`]).
    tapes: Mutex<Vec<Tape>>,
    has_tapes: AtomicBool,
}

impl Doc {
    /// Take ownership of an already-owned buffer. **No scan happens here**:
    /// this is the whole of "load from stored bytes", and it is why
    /// `model.bytes.002` can require a zero token count.
    pub fn new(bytes: Arc<[u8]>, meter: Arc<Meter>) -> Self {
        Self {
            bytes,
            meter,
            objects: Mutex::new(OffsetMap::default()),
            arrays: Mutex::new(OffsetMap::default()),
            tapes: Mutex::new(Vec::new()),
            has_tapes: AtomicBool::new(false),
        }
    }

    pub fn bytes(&self) -> &Arc<[u8]> {
        &self.bytes
    }

    pub fn meter(&self) -> &Arc<Meter> {
        &self.meter
    }

    /// The span of the whole root value.
    pub fn root(&self) -> Span {
        let start = skip_ws(&self.bytes, 0);
        Span::new(start, self.bytes.len())
    }

    fn key_matches(&self, key: Span, want: &[u8]) -> bool {
        let raw = key.of(&self.bytes);
        if raw.contains(&b'\\') {
            unescape(raw)
                .map(|text| text.as_bytes() == want)
                .unwrap_or(false)
        } else {
            raw == want
        }
    }

    /// The span of `key`'s value in the object starting at `object_start`, or
    /// `None` when the object has no such member. Scans only as far as the
    /// member, and remembers how far it got.
    pub fn member(&self, object_start: usize, key: &str) -> Result<Option<Span>, DecodeError> {
        self.member_bytes(object_start, key.as_bytes())
    }

    /// [`Doc::member`] over the raw property-name bytes.
    pub fn member_bytes(
        &self,
        object_start: usize,
        key: &[u8],
    ) -> Result<Option<Span>, DecodeError> {
        let mut index = self
            .objects
            .lock()
            .map_err(|_| DecodeError::new("", object_start, "the index lock was poisoned"))?;
        let cursor = self.cursor_for(&mut index, object_start)?;
        // Already discovered?
        for (found_key, value) in &cursor.members {
            if self.key_matches(*found_key, key) {
                return Ok(Some(*value));
            }
        }
        while !cursor.done {
            let discovered = self.advance_object(cursor)?;
            match discovered {
                None => break,
                Some((found_key, value)) => {
                    if self.key_matches(found_key, key) {
                        return Ok(Some(value));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Every member of the object starting at `object_start`, in document
    /// order, as `(property name, value span)`.
    pub fn members(&self, object_start: usize) -> Result<Vec<(String, Span)>, DecodeError> {
        let spans = self.member_spans(object_start)?;
        let mut out = Vec::with_capacity(spans.len());
        for (key, value) in spans {
            out.push((unescape(key.of(&self.bytes))?, value));
        }
        Ok(out)
    }

    /// Every member of the object, with the property names left as spans. A
    /// decode looks property names up by bytes, so it never pays for a
    /// `String` per property.
    pub fn member_spans(&self, object_start: usize) -> Result<Vec<(Span, Span)>, DecodeError> {
        let mut index = self
            .objects
            .lock()
            .map_err(|_| DecodeError::new("", object_start, "the index lock was poisoned"))?;
        let cursor = self.cursor_for(&mut index, object_start)?;
        while !cursor.done {
            if self.advance_object(cursor)?.is_none() {
                break;
            }
        }
        self.meter.size_proportional_allocation();
        Ok(cursor.members.clone())
    }

    /// [`Doc::member_spans`], recording into `tape` the structure of every
    /// container the scan steps over. The same tokens, allocations, errors and
    /// index state as [`Doc::member_spans`].
    pub fn member_spans_recording(
        &self,
        object_start: usize,
        tape: &mut Tape,
    ) -> Result<Vec<(Span, Span)>, DecodeError> {
        let mut index = self
            .objects
            .lock()
            .map_err(|_| DecodeError::new("", object_start, "the index lock was poisoned"))?;
        let cursor = self.cursor_for(&mut index, object_start)?;
        while !cursor.done {
            if self.advance_object_recording(cursor, tape)?.is_none() {
                break;
            }
        }
        self.meter.size_proportional_allocation();
        Ok(cursor.members.clone())
    }

    /// The text of a property name span.
    pub fn property_name(&self, key: Span) -> Result<String, DecodeError> {
        unescape(key.of(&self.bytes))
    }

    /// The members of a container a full decode entered from a tape: what
    /// the cursor entering it would have left in the index.
    fn entered_on_tape(&self, start: usize, object: bool) -> Option<(Vec<(Span, Span)>, usize)> {
        if !self.has_tapes.load(Ordering::Acquire) {
            return None;
        }
        let tapes = self.tapes.lock().ok()?;
        tapes
            .iter()
            .find_map(|tape| tape.entered(start, object))
            .map(|(entries, end)| (entries.to_vec(), end))
    }

    /// Keep `tape`, so the containers entered from it answer later lookups.
    pub fn install_tape(&self, tape: Tape) {
        if !tape.nodes.iter().any(|node| node.entered) {
            return;
        }
        if let Ok(mut tapes) = self.tapes.lock() {
            tapes.push(tape);
            self.has_tapes.store(true, Ordering::Release);
        }
    }

    fn cursor_for<'i>(
        &self,
        index: &'i mut OffsetMap<ObjectCursor>,
        object_start: usize,
    ) -> Result<&'i mut ObjectCursor, DecodeError> {
        let (bytes, meter) = (&self.bytes[..], &*self.meter);
        match index.entry(object_start) {
            Entry::Occupied(occupied) => Ok(occupied.into_mut()),
            Entry::Vacant(vacant) => {
                if let Some((members, end)) = self.entered_on_tape(object_start, true) {
                    return Ok(vacant.insert(ObjectCursor {
                        members,
                        resume: end,
                        done: true,
                    }));
                }
                let start = skip_ws(bytes, object_start);
                if bytes.get(start) != Some(&b'{') {
                    return Err(DecodeError::new("", start, "expected a JSON object"));
                }
                meter.tokens(1);
                // The member list of this object: one allocation whose size
                // scales with the object, which is why entering an object is
                // counted. Sized for a typical FHIR object up front, so
                // discovering its members does not reallocate the list as it
                // goes.
                meter.size_proportional_allocation();
                Ok(vacant.insert(ObjectCursor {
                    members: Vec::with_capacity(MEMBERS_HINT),
                    resume: start + 1,
                    done: false,
                }))
            }
        }
    }

    /// Discover one more member, or mark the object finished.
    fn advance_object(
        &self,
        cursor: &mut ObjectCursor,
    ) -> Result<Option<(Span, Span)>, DecodeError> {
        self.advance_object_recording(cursor, &mut NoRecord)
    }

    fn advance_object_recording<R: Record>(
        &self,
        cursor: &mut ObjectCursor,
        record: &mut R,
    ) -> Result<Option<(Span, Span)>, DecodeError> {
        let bytes = &self.bytes;
        let mut at = skip_ws(bytes, cursor.resume);
        if bytes.get(at) == Some(&b'}') {
            self.meter.tokens(1);
            cursor.done = true;
            return Ok(None);
        }
        let (key, next) = scan_string(bytes, at, &self.meter)?;
        at = skip_ws(bytes, next);
        if bytes.get(at) != Some(&b':') {
            return Err(DecodeError::new(
                "",
                at,
                "expected ':' after a property name",
            ));
        }
        let (value, next) = skip_value_recording(bytes, at + 1, &self.meter, record)?;
        at = skip_ws(bytes, next);
        match bytes.get(at) {
            Some(b',') => cursor.resume = at + 1,
            Some(b'}') => {
                self.meter.tokens(1);
                cursor.resume = at + 1;
                cursor.done = true;
            }
            _ => {
                return Err(DecodeError::new(
                    "",
                    at,
                    "expected ',' or '}' after a property value",
                ));
            }
        }
        cursor.members.push((key, value));
        Ok(Some((key, value)))
    }

    fn array_cursor_for<'i>(
        &self,
        index: &'i mut OffsetMap<ArrayCursor>,
        array_start: usize,
    ) -> Result<&'i mut ArrayCursor, DecodeError> {
        let (bytes, meter) = (&self.bytes[..], &*self.meter);
        match index.entry(array_start) {
            Entry::Occupied(occupied) => Ok(occupied.into_mut()),
            Entry::Vacant(vacant) => {
                if let Some((entries, end)) = self.entered_on_tape(array_start, false) {
                    return Ok(vacant.insert(ArrayCursor {
                        items: entries.into_iter().map(|(_, item)| item).collect(),
                        resume: end,
                        done: true,
                    }));
                }
                let start = skip_ws(bytes, array_start);
                if bytes.get(start) != Some(&b'[') {
                    return Err(DecodeError::new("", start, "expected a JSON array"));
                }
                meter.tokens(1);
                meter.size_proportional_allocation();
                Ok(vacant.insert(ArrayCursor {
                    items: Vec::new(),
                    resume: start + 1,
                    done: false,
                }))
            }
        }
    }

    fn advance_array(&self, cursor: &mut ArrayCursor) -> Result<Option<Span>, DecodeError> {
        let bytes = &self.bytes;
        let mut at = skip_ws(bytes, cursor.resume);
        if bytes.get(at) == Some(&b']') {
            self.meter.tokens(1);
            cursor.done = true;
            return Ok(None);
        }
        let (item, next) = skip_value(bytes, at, &self.meter)?;
        at = skip_ws(bytes, next);
        match bytes.get(at) {
            Some(b',') => cursor.resume = at + 1,
            Some(b']') => {
                self.meter.tokens(1);
                cursor.resume = at + 1;
                cursor.done = true;
            }
            _ => {
                return Err(DecodeError::new(
                    "",
                    at,
                    "expected ',' or ']' after an array item",
                ));
            }
        }
        cursor.items.push(item);
        Ok(Some(item))
    }

    /// Item `index` of the array starting at `array_start`. Scans only as far
    /// as that item.
    pub fn item(&self, array_start: usize, index: usize) -> Result<Option<Span>, DecodeError> {
        let mut arrays = self
            .arrays
            .lock()
            .map_err(|_| DecodeError::new("", array_start, "the index lock was poisoned"))?;
        let cursor = self.array_cursor_for(&mut arrays, array_start)?;
        while cursor.items.len() <= index && !cursor.done {
            if self.advance_array(cursor)?.is_none() {
                break;
            }
        }
        Ok(cursor.items.get(index).copied())
    }

    /// How many items the array starting at `array_start` holds. Scans the
    /// array's separators and steps over its items; it materialises nothing,
    /// which is what `model.lazy.003` grades.
    pub fn item_count(&self, array_start: usize) -> Result<usize, DecodeError> {
        let mut arrays = self
            .arrays
            .lock()
            .map_err(|_| DecodeError::new("", array_start, "the index lock was poisoned"))?;
        let cursor = self.array_cursor_for(&mut arrays, array_start)?;
        while !cursor.done {
            if self.advance_array(cursor)?.is_none() {
                break;
            }
        }
        Ok(cursor.items.len())
    }

    /// Every item of the array starting at `array_start`, in order. Scans
    /// exactly what [`Doc::item_count`] scans, and takes the index lock once
    /// for the whole array rather than once per item.
    pub fn items(&self, array_start: usize) -> Result<Vec<Span>, DecodeError> {
        let mut arrays = self
            .arrays
            .lock()
            .map_err(|_| DecodeError::new("", array_start, "the index lock was poisoned"))?;
        let cursor = self.array_cursor_for(&mut arrays, array_start)?;
        while !cursor.done {
            if self.advance_array(cursor)?.is_none() {
                break;
            }
        }
        Ok(cursor.items.clone())
    }

    /// Whether the value at `span` is the JSON literal `null`.
    pub fn is_null(&self, span: Span) -> bool {
        is_null(&self.bytes, span)
    }

    /// Whether the value at `span` is a JSON array.
    pub fn is_array(&self, span: Span) -> bool {
        self.bytes.get(span.start) == Some(&b'[')
    }

    /// Whether the value at `span` is a JSON object.
    pub fn is_object(&self, span: Span) -> bool {
        self.bytes.get(span.start) == Some(&b'{')
    }

    /// Materialise the lexical form of the primitive at `span`. A quoted value
    /// is unescaped; a bare value is handed back verbatim so a decimal keeps
    /// the precision it was written with (`model.canonical.003`).
    pub fn lexical(&self, span: Span) -> Result<String, DecodeError> {
        self.lexical_borrowed(span).map(Cow::into_owned)
    }

    /// [`Doc::lexical`], borrowing from the stored buffer whenever the value
    /// needs no unescaping, which is nearly always.
    pub fn lexical_borrowed(&self, span: Span) -> Result<Cow<'_, str>, DecodeError> {
        if self.bytes.get(span.start) == Some(&b'"') {
            let inner = Span::new(span.start + 1, span.end.saturating_sub(1));
            unescape_borrowed(inner.of(&self.bytes))
        } else {
            std::str::from_utf8(span.of(&self.bytes))
                .map(Cow::Borrowed)
                .map_err(|e| DecodeError::new("", span.start, format!("invalid UTF-8: {e}")))
        }
    }
}
