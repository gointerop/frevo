//! The expression text the lexer scans, and the offsets reported back.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/Utilities.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 (`stripBOM`).
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: upstream's `FHIRLexer` constructor removes
//! **every** byte-order mark from the source, wherever it occurs, which shifts
//! the positions of everything after it. Frevo reports byte offsets into the
//! text the caller passed, so the removal is paired with a mapping back. The
//! mapping is only built when the text actually contains a mark, which no
//! expression in the pinned suite does.

/// The scanned text, plus the mapping from its offsets back to the caller's.
pub(crate) struct Source {
    text: String,
    /// `map[i]` is the offset, in the original text, of byte `i` of `text`.
    /// `None` when nothing was stripped and the two agree.
    map: Option<Vec<u32>>,
    original_len: usize,
}

impl Source {
    pub(crate) fn new(input: &str) -> Self {
        if !input.contains('\u{feff}') {
            return Source {
                text: input.to_string(),
                map: None,
                original_len: input.len(),
            };
        }
        let mut text = String::with_capacity(input.len());
        let mut map: Vec<u32> = Vec::with_capacity(input.len() + 1);
        for (at, ch) in input.char_indices() {
            if ch == '\u{feff}' {
                continue;
            }
            for k in 0..ch.len_utf8() {
                map.push(u32::try_from(at + k).unwrap_or(u32::MAX));
            }
            text.push(ch);
        }
        map.push(u32::try_from(input.len()).unwrap_or(u32::MAX));
        Source {
            text,
            map: Some(map),
            original_len: input.len(),
        }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// Whether the scanned text differs from the caller's, in which case
    /// offsets have to be translated.
    pub(crate) fn is_remapped(&self) -> bool {
        self.map.is_some()
    }

    /// Translate an offset in [`Source::text`] into one in the text the caller
    /// passed, so a diagnostic names a position the caller can index with.
    pub(crate) fn to_original(&self, offset: usize) -> usize {
        match &self.map {
            None => offset.min(self.original_len),
            Some(map) => match map.get(offset) {
                Some(o) => *o as usize,
                None => self.original_len,
            },
        }
    }
}
