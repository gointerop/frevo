//! What a parse rejection carries.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRLexer.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: upstream's `FHIRLexerException` carries a
//! `SourceLocation` line/column pair; this carries a byte offset into the
//! expression instead, because `path.parse.007` asks for the offset and
//! upstream's column bookkeeping does not count the characters
//! `skipWhitespaceAndComments` consumed (`FHIRLexer.java:285`).

use std::error::Error;
use std::fmt;

/// A syntax error, with the byte offset in the expression at which it was
/// found (`path.parse.007`).
///
/// The offset is a byte offset into the expression text as the caller wrote
/// it, and always falls on a `char` boundary. It points at the first byte of
/// the offending token, which for an end-of-input failure is the length of the
/// expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
    offset: usize,
}

impl ParseError {
    pub(crate) fn new(message: impl Into<String>, offset: usize) -> Self {
        Self {
            message: message.into(),
            offset,
        }
    }

    /// The diagnostic text. It reproduces upstream's wording, without
    /// upstream's `Error @<line>, <column>: ` prefix: the position travels in
    /// [`ParseError::offset`] rather than inside the sentence.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The byte offset in the expression at which the error was found.
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at offset {})", self.message, self.offset)
    }
}

impl Error for ParseError {}
