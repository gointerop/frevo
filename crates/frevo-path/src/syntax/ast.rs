//! The parsed form: the node kinds, the function table and the operators.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/ExpressionNode.java,
//! org.hl7.fhir.core/org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/SourceLocation.java
//! and (`rep`) org.hl7.fhir.core/org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/exceptions/PathEngineException.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the node graph is an arena of [`NodeId`]s
//! rather than a graph of mutable objects, because the precedence pass rewires
//! it (`organisePrecedence`) and an arena makes that safe in Rust and keeps a
//! parsed expression to one allocation block for reuse (`path.parse.001`,
//! `path.perf.002`). A node keeps its byte offsets for its own spans and,
//! beside them, the line and column HAPI's lexer records for it
//! ([`SourceLocation`]), which is what an execution error's position suffix
//! names (`path.errors.002`); a node HAPI builds without a start has none.

use std::fmt;

use crate::seam::value::Value;

/// `ExpressionNode.Kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Name,
    Function,
    Constant,
    Group,
    Unary,
}

/// A literal, as the parser resolved it.
///
/// `processConstant` (`FHIRPathEngine.java:1323`) resolves a literal to a
/// value at **parse** time for every form but two: `{}` is a null constant,
/// and a `%name` or `@...` run stays as its source text because resolving it
/// needs the evaluation context (`FHIRConstant`).
#[derive(Debug, Clone)]
pub enum Constant {
    /// `{}` — upstream's null constant, which `resolveConstant` turns into an
    /// empty collection (`path.literal.014`).
    Empty,
    /// A boolean, string, integer, decimal or quantity literal.
    Value(Value),
    /// A `%name` environment variable or an `@...` temporal literal, verbatim
    /// including its leading character. Upstream's `FHIRConstant`.
    Deferred(String),
}

/// `org.hl7.fhir.utilities.SourceLocation`: HAPI's one-based line and
/// column, advanced by the lexer's own rule rather than derived from a byte
/// offset (`path.errors.002`). An execution error's ` (@char C)` or
/// ` (@line L char C)` suffix is rendered from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
}

impl SourceLocation {
    /// `new SourceLocation(1, 1)`, where every lexer starts
    /// (`FHIRLexer.java:56`).
    pub(crate) const START: SourceLocation = SourceLocation { line: 1, column: 1 };

    /// `incColumn(i)`.
    pub(crate) fn inc_column(&mut self, by: u32) {
        self.column = self.column.saturating_add(by);
    }

    /// `newLine()`.
    fn new_line(&mut self) {
        self.line = self.line.saturating_add(1);
        self.column = 1;
    }

    /// `checkChar(ch, last13)`: a CR, or an LF not preceded by a CR, starts a
    /// new line; any other character advances the column by one. Answers
    /// whether `ch` was a CR.
    pub(crate) fn check_char(&mut self, ch: char, last13: bool) -> bool {
        match ch {
            '\r' => {
                self.new_line();
                true
            }
            '\n' => {
                if !last13 {
                    self.new_line();
                }
                false
            }
            _ => {
                self.column = self.column.saturating_add(1);
                false
            }
        }
    }

    /// `PathEngineException.rep(loc, expr)` for a non-null `loc`
    /// (`PathEngineException.java:81-93`).
    pub(crate) fn suffix(&self) -> String {
        if self.line == 1 {
            format!(" (@char {})", self.column)
        } else {
            format!(" (@line {} char {})", self.line, self.column)
        }
    }
}

/// Where a node, or its operator, starts: the zero-based byte offset of its
/// first token, and HAPI's line and column for the same token. A node HAPI
/// builds without a start -- a `newGroup` precedence group, the indexer node
/// -- has none, and an error it holds carries no position (`path.errors.002`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub offset: usize,
    pub location: SourceLocation,
}

/// Where an error raised about a node is attributed: the byte offset the
/// node's text starts at, which [`EvalError::Unsupported`] always names, and
/// the start HAPI records for it, which only some nodes have
/// (`path.errors.002`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct At {
    pub offset: usize,
    pub position: Option<Position>,
}

impl At {
    /// `holder.getStart()`.
    pub(crate) fn node(node: &ExpressionNode) -> At {
        At {
            offset: node.start(),
            position: node.position(),
        }
    }

    /// `holder.getOpStart()`.
    pub(crate) fn op(node: &ExpressionNode) -> At {
        At {
            offset: node.op_start(),
            position: node.op_position(),
        }
    }
}

/// An index into an [`Expression`]'s arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

impl NodeId {
    fn index(self) -> usize {
        self.0 as usize
    }
}

/// One node of the parsed form.
#[derive(Debug, Clone)]
pub struct ExpressionNode {
    kind: Kind,
    name: Option<String>,
    constant: Option<Constant>,
    function: Option<Function>,
    parameters: Vec<NodeId>,
    inner: Option<NodeId>,
    group: Option<NodeId>,
    operation: Option<Operation>,
    proximal: bool,
    op_next: Option<NodeId>,
    start: usize,
    end: usize,
    op_start: usize,
    op_end: usize,
    /// `getStart()`: `None` for a node HAPI builds without one.
    start_location: Option<SourceLocation>,
    /// `getOpStart()`: `None` until an operator is parsed onto the node, and
    /// for a precedence group that took an operator over.
    op_location: Option<SourceLocation>,
}

impl ExpressionNode {
    pub(crate) fn new(kind: Kind, start: usize) -> Self {
        ExpressionNode {
            kind,
            name: None,
            constant: None,
            function: None,
            parameters: Vec::new(),
            inner: None,
            group: None,
            operation: None,
            proximal: false,
            op_next: None,
            start,
            end: start,
            op_start: start,
            op_end: start,
            start_location: None,
            op_location: None,
        }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn constant(&self) -> Option<&Constant> {
        self.constant.as_ref()
    }

    pub fn function(&self) -> Option<Function> {
        self.function
    }

    pub fn parameters(&self) -> &[NodeId] {
        &self.parameters
    }

    pub fn inner(&self) -> Option<NodeId> {
        self.inner
    }

    pub fn group(&self) -> Option<NodeId> {
        self.group
    }

    pub fn operation(&self) -> Option<Operation> {
        self.operation
    }

    pub fn op_next(&self) -> Option<NodeId> {
        self.op_next
    }

    /// A proximal node is the first in a sequence of operations, which is what
    /// decides where the precedence pass regroups.
    pub fn is_proximal(&self) -> bool {
        self.proximal
    }

    /// The byte offset in the expression at which this sub-expression starts
    /// (`path.errors.002`).
    pub fn start(&self) -> usize {
        self.start
    }

    /// `getStart()`: where HAPI records this node as starting, or `None` for
    /// a node built without a start (`path.errors.002`).
    pub fn position(&self) -> Option<Position> {
        self.start_location.map(|location| Position {
            offset: self.start,
            location,
        })
    }

    /// `getOpStart()`: where HAPI records this node's operator token as
    /// starting, or `None` where it records none (`path.errors.002`).
    pub fn op_position(&self) -> Option<Position> {
        self.op_location.map(|location| Position {
            offset: self.op_start,
            location,
        })
    }

    /// One past the last byte of this sub-expression.
    pub fn end(&self) -> usize {
        self.end
    }

    /// The byte offset of this node's operator token.
    pub fn op_start(&self) -> usize {
        self.op_start
    }

    /// One past the last byte of this node's operator token.
    pub fn op_end(&self) -> usize {
        self.op_end
    }

    /// `ExpressionNode.isNullSet`: the `{}` literal.
    pub fn is_null_set(&self) -> bool {
        self.kind == Kind::Constant && matches!(self.constant, Some(Constant::Empty))
    }

    // -- mutators, used by the parser only --------------------------------

    pub(crate) fn set_kind(&mut self, kind: Kind) {
        self.kind = kind;
    }

    pub(crate) fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub(crate) fn set_constant(&mut self, constant: Constant) {
        self.constant = Some(constant);
    }

    pub(crate) fn set_function(&mut self, function: Function) {
        self.function = Some(function);
    }

    pub(crate) fn push_parameter(&mut self, id: NodeId) {
        self.parameters.push(id);
    }

    pub(crate) fn set_inner(&mut self, id: Option<NodeId>) {
        self.inner = id;
    }

    pub(crate) fn set_group(&mut self, id: NodeId) {
        self.group = Some(id);
    }

    pub(crate) fn set_operation(&mut self, op: Option<Operation>) {
        self.operation = op;
    }

    pub(crate) fn set_proximal(&mut self, proximal: bool) {
        self.proximal = proximal;
    }

    pub(crate) fn set_op_next(&mut self, id: Option<NodeId>) {
        self.op_next = id;
    }

    pub(crate) fn set_end(&mut self, at: usize) {
        self.end = at;
    }

    pub(crate) fn set_op_span(&mut self, start: usize, end: usize) {
        self.op_start = start;
        self.op_end = end;
    }

    pub(crate) fn set_start_location(&mut self, location: SourceLocation) {
        self.start_location = Some(location);
    }

    pub(crate) fn set_op_location(&mut self, location: SourceLocation) {
        self.op_location = Some(location);
    }

    /// Translate this node's four offsets, for the one case where the scanned
    /// text and the caller's text do not agree: an expression that carried a
    /// byte-order mark.
    pub(crate) fn remap(&mut self, map: impl Fn(usize) -> usize) {
        self.start = map(self.start);
        self.end = map(self.end);
        self.op_start = map(self.op_start);
        self.op_end = map(self.op_end);
    }
}

/// A parsed expression: the reusable form a caller parses once and evaluates
/// many times (`path.parse.001`, `path.api.002`, `path.perf.002`).
///
/// It is immutable after parsing, owns its text and its nodes, and holds no
/// evaluation state, so one parsed expression can be shared and evaluated
/// against any number of resources.
#[derive(Debug, Clone)]
pub struct Expression {
    text: String,
    nodes: Vec<ExpressionNode>,
    root: NodeId,
}

impl Expression {
    pub(crate) fn new(text: String, nodes: Vec<ExpressionNode>, root: NodeId) -> Self {
        Expression { text, nodes, root }
    }

    /// The expression as the caller wrote it.
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> &ExpressionNode {
        &self.nodes[id.index()]
    }

    /// How many nodes the parsed form holds. The evaluator's step limit is
    /// stated in these terms (`path.errors.004`).
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// `ExpressionNode.toString()`: the sub-expression rooted at `id`,
    /// rendered back to FHIRPath. An execution error names the sub-expression
    /// it failed on, and this is what it names it with (`path.errors.002`).
    pub fn render(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.write(&mut out, id);
        out
    }

    fn write(&self, out: &mut String, id: NodeId) {
        let node = self.node(id);
        match node.kind {
            Kind::Name => out.push_str(node.name.as_deref().unwrap_or("")),
            Kind::Function => {
                let is_item = node.function == Some(Function::Item);
                if is_item {
                    out.push('[');
                } else {
                    out.push_str(node.name.as_deref().unwrap_or(""));
                    out.push('(');
                }
                for (at, param) in node.parameters.iter().enumerate() {
                    if at > 0 {
                        out.push_str(", ");
                    }
                    self.write(out, *param);
                }
                out.push(if is_item { ']' } else { ')' });
            }
            Kind::Constant => match &node.constant {
                None | Some(Constant::Empty) => out.push_str("{}"),
                Some(Constant::Deferred(text)) => out.push_str(text),
                Some(Constant::Value(value)) => render_value(out, value),
            },
            Kind::Group => {
                out.push('(');
                if let Some(group) = node.group {
                    self.write(out, group);
                }
                out.push(')');
            }
            Kind::Unary => {}
        }
        if let Some(inner) = node.inner {
            if !(self.node(inner).kind == Kind::Function
                && self.node(inner).function == Some(Function::Item))
            {
                out.push('.');
            }
            self.write(out, inner);
        }
        if let Some(op) = node.operation {
            out.push(' ');
            out.push_str(op.to_code());
            out.push(' ');
            if let Some(next) = node.op_next {
                self.write(out, next);
            }
        }
    }
}

/// How `ExpressionNode.toString` renders a literal
/// (`ExpressionNode.java:423-445`): a string in single quotes, a quantity as
/// its value and unit, everything else as its primitive value.
fn render_value(out: &mut String, value: &Value) {
    match value {
        Value::String(s) => {
            out.push('\'');
            out.push_str(s);
            out.push('\'');
        }
        Value::Quantity(q) => {
            out.push_str(q.value.lexical());
            let unit = q.unit.as_deref().or(q.code.as_deref());
            if let Some(unit) = unit {
                out.push_str(" '");
                out.push_str(unit);
                out.push('\'');
            }
        }
        Value::Boolean(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Integer(i) => out.push_str(&i.to_string()),
        Value::Decimal(d) => out.push_str(d.lexical()),
        Value::Date(d) => out.push_str(&d.lexical),
        Value::DateTime(d) => out.push_str(&d.lexical),
        Value::Time(t) => out.push_str(&t.lexical),
        Value::Node(n) => out.push_str(n.type_name()),
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render(self.root))
    }
}

/// How many parameters a function takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arity {
    Exact(usize),
    Between(usize, usize),
    /// A host-supplied function: the bounds come from the host's
    /// `FunctionDetails` (`FHIRPathEngine.java:1462`).
    Host,
}

/// `ExpressionNode.Function`: every function name the pinned R4 engine knows.
///
/// The list is exactly upstream's, which is what makes a name it does not have
/// — `toDate`, or any of the seven document functions — a **parse** failure
/// rather than an evaluation one (`path.parse.005`, `DEV-0009`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Function {
    Custom,
    Empty,
    Not,
    Exists,
    SubsetOf,
    SupersetOf,
    IsDistinct,
    Distinct,
    Count,
    Where,
    Select,
    All,
    Repeat,
    Aggregate,
    /// Implicit, from `name[...]`.
    Item,
    As,
    Is,
    Single,
    First,
    Last,
    Tail,
    Skip,
    Take,
    Union,
    Combine,
    Intersect,
    Exclude,
    Iif,
    Upper,
    Lower,
    ToChars,
    IndexOf,
    Substring,
    StartsWith,
    EndsWith,
    Matches,
    MatchesFull,
    ReplaceMatches,
    Contains,
    Replace,
    Length,
    Children,
    Descendants,
    MemberOf,
    Trace,
    DefineVariable,
    Check,
    Today,
    Now,
    Resolve,
    Extension,
    AllFalse,
    AnyFalse,
    AllTrue,
    AnyTrue,
    HasValue,
    OfType,
    Type,
    ConvertsToBoolean,
    ConvertsToInteger,
    ConvertsToString,
    ConvertsToDecimal,
    ConvertsToQuantity,
    ConvertsToDateTime,
    ConvertsToDate,
    ConvertsToTime,
    ToBoolean,
    ToInteger,
    ToString,
    ToDecimal,
    ToQuantity,
    ToDateTime,
    ToTime,
    ConformsTo,
    Round,
    Sqrt,
    Abs,
    Ceiling,
    Exp,
    Floor,
    Ln,
    Log,
    Power,
    Truncate,
    Sort,
    Encode,
    Decode,
    Escape,
    Unescape,
    Trim,
    Split,
    Join,
    LowBoundary,
    HighBoundary,
    Precision,
    HtmlChecks1,
    HtmlChecks2,
    Comparable,
    HasTemplateIdOf,
}

impl Function {
    /// `Function.fromCode` (`ExpressionNode.java:62`). `htmlchecks` is an
    /// accepted alias of `htmlChecks`, as upstream records, "support change of
    /// care from R3".
    pub fn from_code(name: &str) -> Option<Function> {
        Some(match name {
            "empty" => Function::Empty,
            "not" => Function::Not,
            "exists" => Function::Exists,
            "subsetOf" => Function::SubsetOf,
            "supersetOf" => Function::SupersetOf,
            "isDistinct" => Function::IsDistinct,
            "distinct" => Function::Distinct,
            "count" => Function::Count,
            "where" => Function::Where,
            "select" => Function::Select,
            "all" => Function::All,
            "repeat" => Function::Repeat,
            "aggregate" => Function::Aggregate,
            "item" => Function::Item,
            "as" => Function::As,
            "is" => Function::Is,
            "single" => Function::Single,
            "first" => Function::First,
            "last" => Function::Last,
            "tail" => Function::Tail,
            "skip" => Function::Skip,
            "take" => Function::Take,
            "union" => Function::Union,
            "combine" => Function::Combine,
            "intersect" => Function::Intersect,
            "exclude" => Function::Exclude,
            "iif" => Function::Iif,
            "lower" => Function::Lower,
            "upper" => Function::Upper,
            "toChars" => Function::ToChars,
            "indexOf" => Function::IndexOf,
            "substring" => Function::Substring,
            "startsWith" => Function::StartsWith,
            "endsWith" => Function::EndsWith,
            "matches" => Function::Matches,
            "matchesFull" => Function::MatchesFull,
            "replaceMatches" => Function::ReplaceMatches,
            "contains" => Function::Contains,
            "replace" => Function::Replace,
            "length" => Function::Length,
            "children" => Function::Children,
            "descendants" => Function::Descendants,
            "memberOf" => Function::MemberOf,
            "trace" => Function::Trace,
            "defineVariable" => Function::DefineVariable,
            "check" => Function::Check,
            "today" => Function::Today,
            "now" => Function::Now,
            "resolve" => Function::Resolve,
            "extension" => Function::Extension,
            "allFalse" => Function::AllFalse,
            "anyFalse" => Function::AnyFalse,
            "allTrue" => Function::AllTrue,
            "anyTrue" => Function::AnyTrue,
            "hasValue" => Function::HasValue,
            "htmlChecks" | "htmlchecks" => Function::HtmlChecks1,
            "htmlChecks2" => Function::HtmlChecks2,
            "comparable" => Function::Comparable,
            "encode" => Function::Encode,
            "decode" => Function::Decode,
            "escape" => Function::Escape,
            "unescape" => Function::Unescape,
            "trim" => Function::Trim,
            "split" => Function::Split,
            "join" => Function::Join,
            "ofType" => Function::OfType,
            "type" => Function::Type,
            "toInteger" => Function::ToInteger,
            "toDecimal" => Function::ToDecimal,
            "toString" => Function::ToString,
            "toQuantity" => Function::ToQuantity,
            "toBoolean" => Function::ToBoolean,
            "toDateTime" => Function::ToDateTime,
            "toTime" => Function::ToTime,
            "convertsToInteger" => Function::ConvertsToInteger,
            "convertsToDecimal" => Function::ConvertsToDecimal,
            "convertsToString" => Function::ConvertsToString,
            "convertsToQuantity" => Function::ConvertsToQuantity,
            "convertsToBoolean" => Function::ConvertsToBoolean,
            "convertsToDateTime" => Function::ConvertsToDateTime,
            "convertsToDate" => Function::ConvertsToDate,
            "convertsToTime" => Function::ConvertsToTime,
            "conformsTo" => Function::ConformsTo,
            "round" => Function::Round,
            "sqrt" => Function::Sqrt,
            "abs" => Function::Abs,
            "ceiling" => Function::Ceiling,
            "exp" => Function::Exp,
            "floor" => Function::Floor,
            "ln" => Function::Ln,
            "log" => Function::Log,
            "power" => Function::Power,
            "truncate" => Function::Truncate,
            "sort" => Function::Sort,
            "lowBoundary" => Function::LowBoundary,
            "highBoundary" => Function::HighBoundary,
            "precision" => Function::Precision,
            "hasTemplateIdOf" => Function::HasTemplateIdOf,
            _ => return None,
        })
    }

    /// `Function.toCode` (`ExpressionNode.java:166`), upstream's spellings
    /// kept as they are — including `ConvertsToTime`, which upstream renders
    /// as `isTime`, and `Custom`, which renders as `?custom?`.
    pub fn to_code(self) -> &'static str {
        match self {
            Function::Custom => "?custom?",
            Function::Empty => "empty",
            Function::Not => "not",
            Function::Exists => "exists",
            Function::SubsetOf => "subsetOf",
            Function::SupersetOf => "supersetOf",
            Function::IsDistinct => "isDistinct",
            Function::Distinct => "distinct",
            Function::Count => "count",
            Function::Where => "where",
            Function::Select => "select",
            Function::All => "all",
            Function::Repeat => "repeat",
            Function::Aggregate => "aggregate",
            Function::Item => "item",
            Function::As => "as",
            Function::Is => "is",
            Function::Single => "single",
            Function::First => "first",
            Function::Last => "last",
            Function::Tail => "tail",
            Function::Skip => "skip",
            Function::Take => "take",
            Function::Union => "union",
            Function::Combine => "combine",
            Function::Intersect => "intersect",
            Function::Exclude => "exclude",
            Function::Iif => "iif",
            Function::ToChars => "toChars",
            Function::Lower => "lower",
            Function::Upper => "upper",
            Function::IndexOf => "indexOf",
            Function::Substring => "substring",
            Function::StartsWith => "startsWith",
            Function::EndsWith => "endsWith",
            Function::Matches => "matches",
            Function::MatchesFull => "matchesFull",
            Function::ReplaceMatches => "replaceMatches",
            Function::Contains => "contains",
            Function::Replace => "replace",
            Function::Length => "length",
            Function::Children => "children",
            Function::Descendants => "descendants",
            Function::MemberOf => "memberOf",
            Function::Trace => "trace",
            Function::DefineVariable => "defineVariable",
            Function::Check => "check",
            Function::Today => "today",
            Function::Now => "now",
            Function::Resolve => "resolve",
            Function::Extension => "extension",
            Function::AllFalse => "allFalse",
            Function::AnyFalse => "anyFalse",
            Function::AllTrue => "allTrue",
            Function::AnyTrue => "anyTrue",
            Function::HasValue => "hasValue",
            Function::Encode => "encode",
            Function::Decode => "decode",
            Function::Escape => "escape",
            Function::Unescape => "unescape",
            Function::Trim => "trim",
            Function::Split => "split",
            Function::Join => "join",
            Function::HtmlChecks1 => "htmlChecks",
            Function::HtmlChecks2 => "htmlChecks2",
            Function::Comparable => "comparable",
            Function::OfType => "ofType",
            Function::Type => "type",
            Function::ToInteger => "toInteger",
            Function::ToDecimal => "toDecimal",
            Function::ToString => "toString",
            Function::ToBoolean => "toBoolean",
            Function::ToQuantity => "toQuantity",
            Function::ToDateTime => "toDateTime",
            Function::ToTime => "toTime",
            Function::ConvertsToInteger => "convertsToInteger",
            Function::ConvertsToDecimal => "convertsToDecimal",
            Function::ConvertsToString => "convertsToString",
            Function::ConvertsToBoolean => "convertsToBoolean",
            Function::ConvertsToQuantity => "convertsToQuantity",
            Function::ConvertsToDateTime => "convertsToDateTime",
            Function::ConvertsToDate => "convertsToDate",
            Function::ConvertsToTime => "isTime",
            Function::ConformsTo => "conformsTo",
            Function::Round => "round",
            Function::Sqrt => "sqrt",
            Function::Abs => "abs",
            Function::Ceiling => "ceiling",
            Function::Exp => "exp",
            Function::Floor => "floor",
            Function::Ln => "ln",
            Function::Log => "log",
            Function::Power => "power",
            Function::Truncate => "truncate",
            Function::Sort => "sort",
            Function::LowBoundary => "lowBoundary",
            Function::HighBoundary => "highBoundary",
            Function::Precision => "precision",
            Function::HasTemplateIdOf => "hasTemplateIdOf",
        }
    }

    /// `checkParameters` (`FHIRPathEngine.java:1362`), which is the arity
    /// table: every function's parameter count, checked at parse time
    /// (`path.parse.006`).
    pub fn arity(self) -> Arity {
        use Arity::{Between, Exact, Host};
        match self {
            Function::Custom => Host,
            Function::Empty => Exact(0),
            Function::Not => Exact(0),
            Function::Exists => Between(0, 1),
            Function::SubsetOf => Exact(1),
            Function::SupersetOf => Exact(1),
            Function::IsDistinct => Exact(0),
            Function::Distinct => Exact(0),
            Function::Count => Exact(0),
            Function::Where => Exact(1),
            Function::Select => Exact(1),
            Function::All => Between(0, 1),
            Function::Repeat => Exact(1),
            Function::Aggregate => Between(1, 2),
            Function::Item => Exact(1),
            Function::As => Exact(1),
            Function::OfType => Exact(1),
            Function::Type => Exact(0),
            Function::Is => Exact(1),
            Function::Single => Exact(0),
            Function::First => Exact(0),
            Function::Last => Exact(0),
            Function::Tail => Exact(0),
            Function::Skip => Exact(1),
            Function::Take => Exact(1),
            Function::Union => Exact(1),
            Function::Combine => Exact(1),
            Function::Intersect => Exact(1),
            Function::Exclude => Exact(1),
            Function::Iif => Between(2, 3),
            Function::Lower => Exact(0),
            Function::Upper => Exact(0),
            Function::ToChars => Exact(0),
            Function::IndexOf => Exact(1),
            Function::Substring => Between(1, 2),
            Function::StartsWith => Exact(1),
            Function::EndsWith => Exact(1),
            Function::Matches => Exact(1),
            Function::MatchesFull => Exact(1),
            Function::ReplaceMatches => Exact(2),
            Function::Contains => Exact(1),
            Function::Replace => Exact(2),
            Function::Length => Exact(0),
            Function::Children => Exact(0),
            Function::Descendants => Exact(0),
            Function::MemberOf => Exact(1),
            Function::Trace => Between(1, 2),
            Function::DefineVariable => Between(1, 2),
            Function::Check => Exact(2),
            Function::Today => Exact(0),
            Function::Now => Exact(0),
            Function::Resolve => Exact(0),
            Function::Extension => Exact(1),
            Function::AllFalse => Exact(0),
            Function::AnyFalse => Exact(0),
            Function::AllTrue => Exact(0),
            Function::AnyTrue => Exact(0),
            Function::HasValue => Exact(0),
            Function::Encode => Exact(1),
            Function::Decode => Exact(1),
            Function::Escape => Exact(1),
            Function::Unescape => Exact(1),
            Function::Trim => Exact(0),
            Function::Split => Exact(1),
            Function::Join => Between(0, 1),
            Function::HtmlChecks1 => Exact(0),
            Function::HtmlChecks2 => Exact(0),
            Function::Comparable => Exact(1),
            Function::ToInteger => Exact(0),
            Function::ToDecimal => Exact(0),
            Function::ToString => Exact(0),
            Function::ToQuantity => Exact(0),
            Function::ToBoolean => Exact(0),
            Function::ToDateTime => Exact(0),
            Function::ToTime => Exact(0),
            Function::ConvertsToInteger => Exact(0),
            Function::ConvertsToDecimal => Exact(0),
            Function::ConvertsToString => Exact(0),
            Function::ConvertsToQuantity => Exact(0),
            Function::ConvertsToBoolean => Exact(0),
            Function::ConvertsToDateTime => Exact(0),
            Function::ConvertsToDate => Exact(0),
            Function::ConvertsToTime => Exact(0),
            Function::ConformsTo => Exact(1),
            Function::Round => Between(0, 1),
            Function::Sqrt => Exact(0),
            Function::Abs => Exact(0),
            Function::Ceiling => Exact(0),
            Function::Exp => Exact(0),
            Function::Floor => Exact(0),
            Function::Ln => Exact(0),
            Function::Log => Exact(1),
            Function::Power => Exact(1),
            Function::Truncate => Exact(0),
            Function::Sort => Between(0, 10),
            Function::LowBoundary => Between(0, 1),
            Function::HighBoundary => Between(0, 1),
            Function::Precision => Exact(0),
            Function::HasTemplateIdOf => Exact(1),
        }
    }
}

/// `ExpressionNode.Operation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    Equals,
    Equivalent,
    NotEquals,
    NotEquivalent,
    LessThan,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
    Is,
    As,
    Union,
    Or,
    And,
    Xor,
    Implies,
    Times,
    DivideBy,
    Plus,
    Minus,
    Concatenate,
    Div,
    Mod,
    In,
    Contains,
    MemberOf,
}

impl Operation {
    /// `Operation.fromCode` (`ExpressionNode.java:271`).
    pub fn from_code(name: &str) -> Option<Operation> {
        Some(match name {
            "=" => Operation::Equals,
            "~" => Operation::Equivalent,
            "!=" => Operation::NotEquals,
            "!~" => Operation::NotEquivalent,
            ">" => Operation::Greater,
            "<" => Operation::LessThan,
            ">=" => Operation::GreaterOrEqual,
            "<=" => Operation::LessOrEqual,
            "|" => Operation::Union,
            "or" => Operation::Or,
            "and" => Operation::And,
            "xor" => Operation::Xor,
            "is" => Operation::Is,
            "as" => Operation::As,
            "*" => Operation::Times,
            "/" => Operation::DivideBy,
            "+" => Operation::Plus,
            "-" => Operation::Minus,
            "&" => Operation::Concatenate,
            "implies" => Operation::Implies,
            "div" => Operation::Div,
            "mod" => Operation::Mod,
            "in" => Operation::In,
            "contains" => Operation::Contains,
            "memberOf" => Operation::MemberOf,
            _ => return None,
        })
    }

    /// `Operation.toCode`.
    pub fn to_code(self) -> &'static str {
        match self {
            Operation::Equals => "=",
            Operation::Equivalent => "~",
            Operation::NotEquals => "!=",
            Operation::NotEquivalent => "!~",
            Operation::Greater => ">",
            Operation::LessThan => "<",
            Operation::GreaterOrEqual => ">=",
            Operation::LessOrEqual => "<=",
            Operation::Union => "|",
            Operation::Or => "or",
            Operation::And => "and",
            Operation::Xor => "xor",
            Operation::Times => "*",
            Operation::DivideBy => "/",
            Operation::Plus => "+",
            Operation::Minus => "-",
            Operation::Concatenate => "&",
            Operation::Implies => "implies",
            Operation::Is => "is",
            Operation::As => "as",
            Operation::Div => "div",
            Operation::Mod => "mod",
            Operation::In => "in",
            Operation::Contains => "contains",
            Operation::MemberOf => "memberOf",
        }
    }
}

/// `organisePrecedence` (`FHIRPathEngine.java:1225`): the precedence table, in
/// the order the engine applies it, tightest binding first.
///
/// `implies` is absent on purpose — upstream's comment is "last: implies" and
/// it gathers no group for it, so it binds loosest by being left alone
/// (`path.parse.002`).
pub(crate) const PRECEDENCE: &[&[Operation]] = &[
    &[
        Operation::Times,
        Operation::DivideBy,
        Operation::Div,
        Operation::Mod,
    ],
    &[Operation::Plus, Operation::Minus, Operation::Concatenate],
    &[Operation::Is, Operation::As],
    &[Operation::Union],
    &[
        Operation::LessThan,
        Operation::Greater,
        Operation::LessOrEqual,
        Operation::GreaterOrEqual,
    ],
    &[
        Operation::Equals,
        Operation::Equivalent,
        Operation::NotEquals,
        Operation::NotEquivalent,
    ],
    &[Operation::In, Operation::Contains, Operation::MemberOf],
    &[Operation::And],
    &[Operation::Xor, Operation::Or],
];
