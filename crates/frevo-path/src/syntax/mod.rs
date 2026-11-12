//! The front end: lexer, literals, parser and the parsed form.
//!
//! Tasks 28, 29 and 30 of `add-fhirpath-and-codegen-r4`. The split mirrors the
//! upstream engine's own: `FHIRLexer` is the token layer, `processConstant`
//! and `processDateConstant` decide what a written literal denotes, and
//! `parseExpression` plus `organisePrecedence` build the tree. Keeping them
//! apart is what lets the parse path be measured on its own
//! (`path.perf.001`): nothing here touches a model, a node or an evaluation
//! context.
//!
//! The parsed form, [`ast::Expression`], is immutable, owns its text and its
//! nodes, and carries no evaluation state, so one parse serves any number of
//! evaluations (`path.parse.001`).

pub mod ast;
pub mod error;
mod lexer;
pub mod literal;
pub mod parser;
mod source;

#[cfg(test)]
mod tests;
