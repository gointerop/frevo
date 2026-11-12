//! The recursive-descent parser and the precedence pass.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 (`parse`, `parseExpression`,
//! `organisePrecedence`, `gatherPrecedence`, `newGroup`, `processConstant`,
//! `checkParamCount`, `checkParameters`).
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: nodes live in an arena, positions are byte
//! offsets, and the recursion carries a depth limit — upstream's `Group` arm
//! recurses with no counter, so 10,000 nested parentheses end the JVM with a
//! StackOverflowError, which `path.errors.003` forbids and
//! `path.errors.004` replaces with a returned error.

use super::ast::{
    Arity, Constant, Expression, ExpressionNode, Function, Kind, NodeId, Operation, PRECEDENCE,
};
use super::error::ParseError;
use super::lexer::{self, Lexer};
use super::literal::{self, CALENDAR_UNITS, UCUM_SYSTEM};
use super::source::Source;
use crate::seam::host::{FunctionDetails, HostServices};
use crate::seam::value::{Decimal, Quantity, Value};

/// How deeply the parser will recurse before it refuses
/// (`path.errors.004`).
///
/// Upstream has no limit at all, so 10,000 nested parentheses end the JVM
/// with a StackOverflowError. One level here is one frame of
/// `parse_expression`, and a level is spent per written group, per `.` step
/// and per operator in a chain — so the limit has to be well above any real
/// expression and well below what a 2 MiB thread stack holds in an
/// unoptimised build, where a frame is at its largest. 200 is roughly five
/// times the deepest expression in the pinned suite and a quarter of what the
/// smallest stack this runs on can take.
pub const MAX_PARSE_DEPTH: usize = 200;

/// What the parser was told about the lexer's one variable rule.
///
/// `FHIRPathEngine.allowDoubleQuotes` is a plain boolean field with no
/// initialiser (`FHIRPathEngine.java:182`), so it is false on every engine
/// nothing sets it on — which is both HAPI wrappers (`path.lexer.015`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParseOptions {
    /// `setAllowDoubleQuotes` (`FHIRPathEngine.java:6996`), threaded into the
    /// lexer at construction and read at `FHIRLexer.java:209`.
    pub allow_double_quotes: bool,
}

impl Expression {
    /// Parse an expression with the default options and no host services.
    pub fn parse(text: &str) -> Result<Expression, ParseError> {
        Expression::parse_with(text, ParseOptions::default(), None)
    }

    /// `FHIRPathEngine.parse(String, String)` (`FHIRPathEngine.java:333`).
    ///
    /// `host` is consulted for a function name the R4 `Function` table does
    /// not have, and only for that: upstream asks `resolveFunction` **after**
    /// `Function.fromCode` has returned null (`FHIRPathEngine.java:1158-1166`),
    /// so an unknown name is a parse failure unless a host service claims it.
    pub fn parse_with(
        text: &str,
        options: ParseOptions,
        host: Option<&dyn HostServices>,
    ) -> Result<Expression, ParseError> {
        let source = Source::new(text);
        match parse_inner(&source, options, host) {
            Ok((nodes, root)) => Ok(Expression::new(text.to_string(), nodes, root)),
            Err(e) => Err(e.remapped(|offset| source.to_original(offset))),
        }
    }
}

impl ParseError {
    fn remapped(self, map: impl Fn(usize) -> usize) -> ParseError {
        let offset = map(self.offset());
        ParseError::new(self.message().to_string(), offset)
    }
}

fn parse_inner(
    source: &Source,
    options: ParseOptions,
    host: Option<&dyn HostServices>,
) -> Result<(Vec<ExpressionNode>, NodeId), ParseError> {
    let lexer = Lexer::new(source.text(), options.allow_double_quotes)?;
    let mut parser = Parser {
        lexer,
        host,
        nodes: Vec::new(),
        depth: 0,
    };
    if parser.lexer.done() {
        return Err(ParseError::new("Path cannot be empty", 0));
    }
    let root = parser.parse_expression(true)?;
    if !parser.lexer.done() {
        return Err(parser.lexer.error(format!(
            "Premature ExpressionNode termination at unexpected token \"{}\"",
            parser.lexer.current_or_empty()
        )));
    }
    // Upstream calls `result.check()` here and **discards** what it returns
    // (`FHIRPathEngine.java:342`), so a structurally impossible tree would not
    // be reported. There is nothing to discard here: every node this parser
    // builds gets its kind and its operands in the same step.
    let mut nodes = parser.nodes;
    if source.is_remapped() {
        for node in &mut nodes {
            node.remap(|offset| source.to_original(offset));
        }
    }
    Ok((nodes, root))
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    host: Option<&'a dyn HostServices>,
    nodes: Vec<ExpressionNode>,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn add(&mut self, kind: Kind, start: usize) -> NodeId {
        let id = NodeId(u32::try_from(self.nodes.len()).unwrap_or(u32::MAX));
        self.nodes.push(ExpressionNode::new(kind, start));
        id
    }

    fn node(&self, id: NodeId) -> &ExpressionNode {
        &self.nodes[id.0 as usize]
    }

    fn node_mut(&mut self, id: NodeId) -> &mut ExpressionNode {
        &mut self.nodes[id.0 as usize]
    }

    fn current(&self) -> Option<&'a str> {
        self.lexer.current()
    }

    /// `parseExpression(lexer, proximal)` (`FHIRPathEngine.java:1072`).
    fn parse_expression(&mut self, proximal: bool) -> Result<NodeId, ParseError> {
        self.depth += 1;
        if self.depth > MAX_PARSE_DEPTH {
            self.depth -= 1;
            return Err(self.lexer.error(format!(
                "This expression nests more than {MAX_PARSE_DEPTH} levels deep, \
                 which is the parser's depth limit"
            )));
        }
        let parsed = self.parse_expression_at_depth(proximal);
        self.depth -= 1;
        parsed
    }

    fn parse_expression_at_depth(&mut self, proximal: bool) -> Result<NodeId, ParseError> {
        // `c`: where this sub-expression starts. An arity diagnostic reports
        // it, and so does an execution error naming the sub-expression.
        let c = self.lexer.current_start();
        let mut result = self.add(Kind::Name, c);
        let mut wrapper: Option<NodeId> = None;

        // A leading `+` or `-` is a unary operation. The lexer cannot decide
        // that, because the same character is binary elsewhere, so the parser
        // back-corrects here and again for a signed constant below.
        if self.lexer.has_token_any(&["-", "+"]) {
            let at = self.lexer.current_start();
            let w = self.add(Kind::Unary, at);
            let op = Operation::from_code(self.lexer.take()?);
            let node = self.node_mut(w);
            node.set_operation(op);
            node.set_proximal(proximal);
            wrapper = Some(w);
        }

        if self.current().is_none() {
            return Err(self.lexer.error("Expression terminated unexpectedly"));
        } else if self.lexer.is_constant() {
            let is_string = self.lexer.is_string_constant();
            if !is_string
                && self
                    .current()
                    .is_some_and(|c| c.starts_with('-') || c.starts_with('+'))
            {
                // The grammar says this is a unary operation, and it affects
                // the order the inner operations are processed in.
                let at = self.lexer.current_start();
                let w = self.add(Kind::Unary, at);
                let sign = &self.lexer.current_or_empty()[..1];
                let op = Operation::from_code(sign);
                let node = self.node_mut(w);
                node.set_operation(op);
                node.set_proximal(proximal);
                wrapper = Some(w);
                self.lexer.drop_first_char_of_current();
            }
            // The numeric token's own text, kept because a quantity literal
            // keeps the value exactly as written (`path.literal.006`).
            let literal_text = self.lexer.current_or_empty().to_string();
            let constant = self.process_constant()?;
            let numeric = matches!(
                &constant,
                Constant::Value(Value::Integer(_)) | Constant::Value(Value::Decimal(_))
            );
            {
                let node = self.node_mut(result);
                node.set_kind(Kind::Constant);
                node.set_constant(constant);
            }

            // The quantity-literal arm (`FHIRPathEngine.java:1105-1133`): a
            // number followed by a quoted unit or by a calendar-duration
            // keyword is one `Quantity` literal.
            if !is_string
                && !self.lexer.done()
                && numeric
                && (self.lexer.is_string_constant() || self.lexer.has_token_any(CALENDAR_UNITS))
            {
                let quantity = self.parse_quantity_unit(&literal_text)?;
                self.node_mut(result).set_constant(quantity);
            }
            let end = self.lexer.previous_end();
            self.node_mut(result).set_end(end);
        } else if self.current() == Some("(") {
            self.lexer.next()?;
            self.node_mut(result).set_kind(Kind::Group);
            let group = self.parse_expression(true)?;
            self.node_mut(result).set_group(group);
            if self.current() != Some(")") {
                return Err(self.lexer.error(format!(
                    "Found {} expecting a \")\"",
                    self.lexer.current_or_empty()
                )));
            }
            let end = self.lexer.current_end();
            self.node_mut(result).set_end(end);
            self.lexer.next()?;
        } else {
            if !self.lexer.is_token() && !self.lexer.current_or_empty().starts_with('`') {
                return Err(self.lexer.error(format!(
                    "Found {} expecting a token name",
                    self.lexer.current_or_empty()
                )));
            }
            let name = if self.lexer.is_fixed_name() {
                self.lexer.read_fixed_name("Path Name")?
            } else {
                self.lexer.take()?.to_string()
            };
            let end = self.lexer.current_start();
            {
                let node = self.node_mut(result);
                node.set_name(name.clone());
                node.set_end(end);
            }
            if !check_name(&name) {
                return Err(self
                    .lexer
                    .error(format!("Found {name} expecting a valid token name")));
            }
            if self.current() == Some("(") {
                let mut details: Option<FunctionDetails> = None;
                let function = match Function::from_code(&name) {
                    Some(f) => f,
                    None => {
                        if let Some(host) = self.host {
                            details = host.resolve_function(&name);
                        }
                        if details.is_none() {
                            // Upstream reports the position of the current
                            // token, which is the `(`; the offset names the
                            // function instead, because that is what the
                            // message is about (`path.parse.007`).
                            return Err(ParseError::new(
                                format!("The name {name} is not a valid function name"),
                                c,
                            ));
                        }
                        Function::Custom
                    }
                };
                {
                    let node = self.node_mut(result);
                    node.set_kind(Kind::Function);
                    node.set_function(function);
                }
                self.lexer.next()?;
                while self.current() != Some(")") {
                    let parameter = self.parse_expression(true)?;
                    self.node_mut(result).push_parameter(parameter);
                    if self.current() == Some(",") {
                        self.lexer.next()?;
                    } else if self.current() != Some(")") {
                        return Err(self.lexer.error(format!(
                            "The token {} is not expected here - either a \",\" or a \")\" expected",
                            self.lexer.current_or_empty()
                        )));
                    }
                }
                let end = self.lexer.current_end();
                self.node_mut(result).set_end(end);
                self.lexer.next()?;
                self.check_parameters(c, result, details.as_ref())?;
            } else {
                self.node_mut(result).set_kind(Kind::Name);
            }
        }

        // `name[...]`: an implicit `item()` call, which the following `.`
        // hangs off rather than off the name.
        let mut focus = result;
        if self.current() == Some("[") {
            let at = self.lexer.current_start();
            self.lexer.next()?;
            let item = self.add(Kind::Function, at);
            {
                let node = self.node_mut(item);
                node.set_function(Function::Item);
                node.set_name("item".to_string());
            }
            let parameter = self.parse_expression(true)?;
            self.node_mut(item).push_parameter(parameter);
            if self.current() != Some("]") {
                return Err(self.lexer.error(format!(
                    "The token {} is not expected here - a \"]\" expected",
                    self.lexer.current_or_empty()
                )));
            }
            let end = self.lexer.current_end();
            self.node_mut(item).set_end(end);
            self.lexer.next()?;
            self.node_mut(result).set_inner(Some(item));
            focus = item;
        }
        if self.current() == Some(".") {
            self.lexer.next()?;
            let inner = self.parse_expression(false)?;
            self.node_mut(focus).set_inner(Some(inner));
        }
        self.node_mut(result).set_proximal(proximal);

        if proximal {
            while self.lexer.is_op() {
                let op = self.current().and_then(Operation::from_code);
                let (op_start, op_end) = (self.lexer.current_start(), self.lexer.current_end());
                {
                    let node = self.node_mut(focus);
                    node.set_operation(op);
                    node.set_op_span(op_start, op_end);
                }
                self.lexer.next()?;
                let next = self.parse_expression(false)?;
                self.node_mut(focus).set_op_next(Some(next));
                focus = next;
            }
            result = self.organise_precedence(result);
        }
        if let Some(w) = wrapper {
            self.node_mut(w).set_op_next(Some(result));
            self.node_mut(result).set_proximal(false);
            result = w;
        }
        Ok(result)
    }

    /// `processConstant(lexer)` (`FHIRPathEngine.java:1323`): which literal
    /// form the current token is, in upstream's order — string, integer,
    /// decimal, boolean, `{}`, then a `%` or `@` run left for evaluation.
    fn process_constant(&mut self) -> Result<Constant, ParseError> {
        if self.lexer.is_string_constant() {
            let at = self.lexer.current_start();
            let text = self.lexer.take()?;
            let decoded = lexer::process_constant_string(text, at)?;
            return Ok(Constant::Value(Value::String(decoded)));
        }
        let current = self.lexer.current_or_empty();
        if literal::is_integer(current) {
            let text = self.lexer.take()?;
            let value = text.parse::<i64>().map_err(|_| {
                self.lexer
                    .error(format!("Invalid Constant {text}: it is not an integer"))
            })?;
            return Ok(Constant::Value(Value::Integer(value)));
        }
        if literal::is_decimal(current, false, false) {
            let at = self.lexer.current_start();
            let text = self.lexer.take()?;
            let value = Decimal::parse(text).ok_or_else(|| {
                ParseError::new(
                    format!(
                        "Invalid Constant {text}: it has more significant digits than \
                         this engine's decimal representation carries"
                    ),
                    at,
                )
            })?;
            return Ok(Constant::Value(Value::Decimal(value)));
        }
        if current == "true" || current == "false" {
            let text = self.lexer.take()?;
            return Ok(Constant::Value(Value::Boolean(text == "true")));
        }
        if current == "{}" {
            self.lexer.take()?;
            return Ok(Constant::Empty);
        }
        if current.starts_with('%') || current.starts_with('@') {
            let text = self.lexer.take()?;
            return Ok(Constant::Deferred(text.to_string()));
        }
        Err(self.lexer.error(format!(
            "Invalid Constant {}",
            self.lexer.current_or_empty()
        )))
    }

    /// The unit half of a quantity literal: either a quoted UCUM code or one
    /// of the sixteen calendar-duration keywords
    /// (`FHIRPathEngine.java:1105-1132`).
    ///
    /// Upstream builds
    /// `new Quantity().setValue(..).setUnit(unit).setSystem(..).setCode(ucum)`,
    /// where `unit` stays null for a quoted code and `system` is set only when
    /// there is a code at all — which is why `1 'cm'` has a code and no unit
    /// text, and `1 year` has unit text and neither code nor system
    /// (`path.literal.012`, `path.literal.013`).
    fn parse_quantity_unit(&mut self, literal_text: &str) -> Result<Constant, ParseError> {
        let at = self.lexer.current_start();
        let (unit, code) = if self.lexer.has_token_any(CALENDAR_UNITS) {
            let word = self.lexer.take()?.to_string();
            let code = literal::calendar_unit_code(&word);
            (Some(word), code.map(str::to_string))
        } else {
            (None, Some(self.lexer.read_constant("units")?))
        };
        let value = Decimal::parse(literal_text).ok_or_else(|| {
            ParseError::new(
                format!("Invalid Constant {literal_text}: it is not a quantity value"),
                at,
            )
        })?;
        Ok(Constant::Value(Value::Quantity(Quantity {
            value,
            unit,
            system: code.as_ref().map(|_| UCUM_SYSTEM.to_string()),
            code,
        })))
    }

    /// `checkParameters` plus `checkParamCount`
    /// (`FHIRPathEngine.java:1348-1465`): the arity check, at parse time,
    /// with the function named and its declared arity in the message
    /// (`path.parse.006`).
    fn check_parameters(
        &self,
        location: usize,
        id: NodeId,
        details: Option<&FunctionDetails>,
    ) -> Result<(), ParseError> {
        let node = self.node(id);
        let name = node.name().unwrap_or("");
        let count = node.parameters().len();
        let arity = match node.function() {
            None => return Ok(()),
            Some(function) => function.arity(),
        };
        let (min, max) = match arity {
            Arity::Exact(n) => {
                if count != n {
                    return Err(ParseError::new(
                        format!("The function \"{name}\" requires {n} parameters"),
                        location,
                    ));
                }
                return Ok(());
            }
            Arity::Between(min, max) => (min, max),
            Arity::Host => match details {
                // `Function.Custom` is only reachable through a host service,
                // and the parser takes the bounds from its `FunctionDetails`
                // (`FHIRPathEngine.java:1462`).
                Some(d) => (d.min_params, d.max_params),
                None => return Ok(()),
            },
        };
        if count < min || count > max {
            return Err(ParseError::new(
                format!("The function \"{name}\" requires between {min} and {max} parameters"),
                location,
            ));
        }
        Ok(())
    }

    // -- precedence -------------------------------------------------------

    /// `organisePrecedence` (`FHIRPathEngine.java:1225`): regroup the flat
    /// operator chain the parse produced, one precedence level at a time.
    fn organise_precedence(&mut self, mut node: NodeId) -> NodeId {
        for ops in PRECEDENCE {
            node = self.gather_precedence(node, ops);
        }
        node
    }

    fn contains(&self, id: Option<NodeId>, ops: &[Operation]) -> bool {
        match id {
            None => false,
            Some(id) => self
                .node(id)
                .operation()
                .is_some_and(|op| ops.contains(&op)),
        }
    }

    fn op_next(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).op_next()
    }

    fn has_op(&self, id: Option<NodeId>) -> bool {
        id.is_some_and(|id| self.node(id).operation().is_some())
    }

    /// `gatherPrecedence` (`FHIRPathEngine.java:1239`), transliterated.
    ///
    /// The sequence of operations hanging off a proximal node is a linked
    /// list; this walks it and wraps each run of operators at this precedence
    /// level in a `Group`, which is what makes `1 + 2 * 3` multiply first.
    fn gather_precedence(&mut self, start: NodeId, ops: &[Operation]) -> NodeId {
        debug_assert!(self.node(start).is_proximal());
        let mut start = start;

        // Is there anything to do? Only if this level's operators and some
        // other level's operators are mixed in the same chain.
        let mut work = false;
        let mut cursor = self.op_next(start);
        if self.contains(Some(start), ops) {
            while self.has_op(cursor) {
                work = work || !self.contains(cursor, ops);
                cursor = cursor.and_then(|id| self.op_next(id));
            }
        } else {
            while self.has_op(cursor) {
                work = work || self.contains(cursor, ops);
                cursor = cursor.and_then(|id| self.op_next(id));
            }
        }
        if !work {
            return start;
        }

        // Entry point: tricky, as upstream says.
        let mut group;
        let mut focus: Option<NodeId>;
        if self.contains(Some(start), ops) {
            group = self.new_group(start);
            self.node_mut(group).set_proximal(true);
            focus = Some(start);
            start = group;
        } else {
            let mut node = start;
            let Some(mut f) = self.op_next(node) else {
                return start;
            };
            while !self.contains(Some(f), ops) {
                node = f;
                match self.op_next(f) {
                    // `work` guarantees a node at this level exists further
                    // along the chain, so this is unreachable; returning
                    // rather than panicking is the cheap way to keep that a
                    // fact about the input and not a trust in it.
                    None => return start,
                    Some(next) => f = next,
                }
            }
            group = self.new_group(f);
            self.node_mut(node).set_op_next(Some(group));
            focus = Some(f);
        }

        // At this point `group` is the group being added to and `focus` points
        // at `group.group`. Upstream's do-while, with the null checks its
        // `focus.getOperation()` calls leave implicit.
        loop {
            // Run to the end of this run of same-level operators.
            while let Some(f) = focus {
                if self.contains(Some(f), ops) {
                    focus = self.op_next(f);
                } else {
                    break;
                }
            }
            let Some(f) = focus else { break };
            if let Some(op) = self.node(f).operation() {
                // The group takes over the operator that ends the run, so the
                // run itself becomes one operand of it.
                let next = self.op_next(f);
                {
                    let g = self.node_mut(group);
                    g.set_operation(Some(op));
                    g.set_op_next(next);
                }
                {
                    let node = self.node_mut(f);
                    node.set_operation(None);
                    node.set_op_next(None);
                }
                // Now look for another run, and start it.
                let mut node = group;
                focus = self.op_next(group);
                while let Some(ff) = focus {
                    if self.contains(Some(ff), ops) {
                        break;
                    }
                    node = ff;
                    focus = self.op_next(ff);
                }
                if let Some(ff) = focus {
                    group = self.new_group(ff);
                    self.node_mut(node).set_op_next(Some(group));
                }
            }
            if !self.has_op(focus) {
                break;
            }
        }
        start
    }

    /// `newGroup` (`FHIRPathEngine.java:1315`).
    fn new_group(&mut self, next: NodeId) -> NodeId {
        let (start, end) = {
            let node = self.node(next);
            (node.start(), node.end())
        };
        let group = self.add(Kind::Group, start);
        {
            let node = self.node_mut(group);
            node.set_kind(Kind::Group);
            node.set_group(next);
            node.set_end(end);
        }
        self.node_mut(next).set_proximal(true);
        group
    }
}

/// `ExpressionNode.checkName` (`ExpressionNode.java:516`): a name beginning
/// with `$` is only ever one of the three the engine defines.
fn check_name(name: &str) -> bool {
    !name.starts_with('$') || matches!(name, "$this" | "$total" | "$index")
}
