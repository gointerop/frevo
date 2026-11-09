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

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Arc, Mutex};

use super::scan::{DecodeError, Meter, Span, is_null, scan_string, skip_value, skip_ws, unescape};

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

/// One document's stored bytes and its index.
#[derive(Debug)]
pub struct Doc {
    bytes: Arc<[u8]>,
    meter: Arc<Meter>,
    objects: Mutex<HashMap<usize, ObjectCursor>>,
    arrays: Mutex<HashMap<usize, ArrayCursor>>,
}

impl Doc {
    /// Take ownership of an already-owned buffer. **No scan happens here**:
    /// this is the whole of "load from stored bytes", and it is why
    /// `model.bytes.002` can require a zero token count.
    pub fn new(bytes: Arc<[u8]>, meter: Arc<Meter>) -> Self {
        Self {
            bytes,
            meter,
            objects: Mutex::new(HashMap::new()),
            arrays: Mutex::new(HashMap::new()),
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
        let cursor = Self::cursor_for(&mut index, object_start, &self.bytes, &self.meter)?;
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
        let cursor = Self::cursor_for(&mut index, object_start, &self.bytes, &self.meter)?;
        while !cursor.done {
            if self.advance_object(cursor)?.is_none() {
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

    fn cursor_for<'i>(
        index: &'i mut HashMap<usize, ObjectCursor>,
        object_start: usize,
        bytes: &[u8],
        meter: &Meter,
    ) -> Result<&'i mut ObjectCursor, DecodeError> {
        match index.entry(object_start) {
            Entry::Occupied(occupied) => Ok(occupied.into_mut()),
            Entry::Vacant(vacant) => {
                let start = skip_ws(bytes, object_start);
                if bytes.get(start) != Some(&b'{') {
                    return Err(DecodeError::new("", start, "expected a JSON object"));
                }
                meter.tokens(1);
                // The member list of this object: one allocation whose size
                // scales with the object, which is why entering an object is
                // counted.
                meter.size_proportional_allocation();
                Ok(vacant.insert(ObjectCursor {
                    members: Vec::new(),
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
        let (value, next) = skip_value(bytes, at + 1, &self.meter)?;
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
        index: &'i mut HashMap<usize, ArrayCursor>,
        array_start: usize,
        bytes: &[u8],
        meter: &Meter,
    ) -> Result<&'i mut ArrayCursor, DecodeError> {
        match index.entry(array_start) {
            Entry::Occupied(occupied) => Ok(occupied.into_mut()),
            Entry::Vacant(vacant) => {
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
        let cursor = Self::array_cursor_for(&mut arrays, array_start, &self.bytes, &self.meter)?;
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
        let cursor = Self::array_cursor_for(&mut arrays, array_start, &self.bytes, &self.meter)?;
        while !cursor.done {
            if self.advance_array(cursor)?.is_none() {
                break;
            }
        }
        Ok(cursor.items.len())
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
        if self.bytes.get(span.start) == Some(&b'"') {
            let inner = Span::new(span.start + 1, span.end.saturating_sub(1));
            unescape(inner.of(&self.bytes))
        } else {
            std::str::from_utf8(span.of(&self.bytes))
                .map(str::to_owned)
                .map_err(|e| DecodeError::new("", span.start, format!("invalid UTF-8: {e}")))
        }
    }
}
