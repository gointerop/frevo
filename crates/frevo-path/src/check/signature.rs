//! The function signature table the check pass reads.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094 — `evaluateFunctionType` at
//! `:3274`, `evaluateParameters` at `:3261`, `isExpressionParameter` at
//! `:3804` and the `checkContext*` family at `:3839`-`:3900`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: upstream's 90-arm `switch` is data here — one
//! [`Signature`] per function — so a function is checked by registering what
//! upstream checks for it, and the pass that walks the expression does not
//! change. Each arm of upstream's switch is three questions, in this order:
//! how the parameters are typed ([`Params`]), what the focus must be
//! ([`Focus`]), what each parameter must be ([`Param`]), and then what the
//! function yields ([`Yields`]).
//!
//! A function with no entry in [`signature`] is checked as
//! `CheckError::NotImplemented`. That is the only answer for a function this
//! table does not describe; there is no default signature.

use super::types::{
    CollectionStatus, SYSTEM_BOOLEAN, SYSTEM_DATE_TIME, SYSTEM_DECIMAL, SYSTEM_INTEGER,
    SYSTEM_QUANTITY, SYSTEM_STRING, SYSTEM_TIME,
};
use crate::syntax::ast::Function;

/// How the parameters are typed before the function is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Params {
    /// `evaluateParameters(context, focus, ...)`.
    Standard,
    /// `evaluateParameters(context, focus.toSingleton(), ...)`: the
    /// criteria-taking functions, whose criteria see one item at a time.
    SingletonFocus,
    /// The parameter is a type name, typed as a string and never evaluated:
    /// `is`, `as`, `ofType`.
    TypeName,
    /// `repeat`'s fixed point over its projection.
    Repeat,
}

/// A precondition on the focus, each one of upstream's `checkContext*` and
/// `checkOrdered`/`checkSingleton` calls. The text is the name upstream puts
/// in the message, which is not always the function's own code: `substring`
/// reports itself as `subString`, `matchesFull` as `matches`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    /// `checkOrdered`: an unordered focus fails.
    Ordered(&'static str),
    /// `checkSingleton`: a non-singleton focus warns.
    Singleton(&'static str),
    /// `checkContextString`.
    String(&'static str),
    /// `checkContextPrimitive`, with or without `Quantity` allowed.
    Primitive(&'static str, bool),
    /// `checkContextNumerical`.
    Numerical(&'static str),
    /// `checkContextDecimal`.
    Decimal(&'static str),
    /// `checkContextContinuous`, with or without `integer` allowed.
    Continuous(&'static str, bool),
    /// `checkContextReference`.
    Reference(&'static str),
    /// `checkContextCoded`.
    Coded(&'static str),
}

/// What one parameter must be (`checkParamTypes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Param {
    /// One of these types, with this collection status.
    Types(CollectionStatus, &'static [&'static str]),
    /// The focus's own types, with the focus's status or the one given.
    FocusTypes(Option<CollectionStatus>),
}

/// What the function yields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Yields {
    /// These types, with this status.
    Types(CollectionStatus, &'static [&'static str]),
    /// The focus, unchanged.
    Focus,
    /// The focus's types, with this status.
    FocusAs(CollectionStatus),
    /// These types, with the focus's status (`resolve()`).
    TypesWithFocusStatus(&'static [&'static str]),
    /// The type of parameter `n`.
    Param(usize),
    /// The focus's types united with parameter 0's.
    UnionWithParam,
    /// The focus's types intersected with parameter 0's.
    IntersectWithParam,
    /// A rule that is code rather than data; see [`Special`].
    Special(Special),
}

/// The arms of upstream's switch that compute their result from more than
/// the focus and the parameter types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Special {
    /// `as()` and `ofType()`: the named type, from the unevaluated parameter.
    TypeNamed,
    /// `type()`: `SimpleTypeInfo`, `ClassInfo` or both.
    TypeInfo,
    /// `iif()`: the union of the two branches.
    Iif,
    /// `aggregate()` and `$total`: any type at all.
    Anything,
    /// `children()`.
    Children,
    /// `descendants()`.
    Descendants,
    /// `defineVariable()`: the focus, with the variable added to the context.
    DefineVariable,
    /// `extension(url)`.
    Extension,
    /// `lowBoundary()` and `highBoundary()`.
    Boundary,
}

/// One function's check.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Signature {
    pub params: Params,
    /// Whether a singleton focus warns that the function expects a
    /// collection (`first`, `last`, `tail`, `skip`, `take`).
    pub warns_not_a_collection: bool,
    pub focus: &'static [Focus],
    pub param_types: &'static [Param],
    pub yields: Yields,
}

impl Signature {
    /// Whether the function's check reads the focus at all. Upstream
    /// dereferences a null focus wherever this is true.
    pub(crate) fn reads_focus(&self) -> bool {
        self.params == Params::SingletonFocus
            || self.params == Params::Repeat
            || self.warns_not_a_collection
            || !self.focus.is_empty()
            || self
                .param_types
                .iter()
                .any(|p| matches!(p, Param::FocusTypes(_)))
            || !matches!(self.yields, Yields::Types(..) | Yields::Param(_))
    }
}

use CollectionStatus::{Ordered, Singleton, Unordered};

const BOOLEAN: Yields = Yields::Types(Singleton, &[SYSTEM_BOOLEAN]);
const STRING: Yields = Yields::Types(Singleton, &[SYSTEM_STRING]);
const INTEGER: Yields = Yields::Types(Singleton, &[SYSTEM_INTEGER]);
const DECIMAL: Yields = Yields::Types(Singleton, &[SYSTEM_DECIMAL]);

const P_BOOLEAN: Param = Param::Types(Singleton, &[SYSTEM_BOOLEAN]);
const P_STRING: Param = Param::Types(Singleton, &[SYSTEM_STRING]);
const P_INTEGER: Param = Param::Types(Singleton, &[SYSTEM_INTEGER]);
const P_NUMBERS: Param = Param::Types(Singleton, &[SYSTEM_INTEGER, SYSTEM_DECIMAL]);

const fn sig(focus: &'static [Focus], param_types: &'static [Param], yields: Yields) -> Signature {
    Signature {
        params: Params::Standard,
        warns_not_a_collection: false,
        focus,
        param_types,
        yields,
    }
}

const fn criteria(param_types: &'static [Param], yields: Yields) -> Signature {
    Signature {
        params: Params::SingletonFocus,
        warns_not_a_collection: false,
        focus: &[],
        param_types,
        yields,
    }
}

const fn subset(
    focus: &'static [Focus],
    param_types: &'static [Param],
    yields: Yields,
) -> Signature {
    Signature {
        params: Params::Standard,
        warns_not_a_collection: true,
        focus,
        param_types,
        yields,
    }
}

const fn type_name(yields: Yields) -> Signature {
    Signature {
        params: Params::TypeName,
        warns_not_a_collection: false,
        focus: &[],
        param_types: &[P_STRING],
        yields,
    }
}

/// The check for `function`, or `None` when this table does not describe it
/// — which the pass reports as not implemented.
///
/// `Function::Custom` has no entry: a host-supplied function is checked by
/// the host service's `check_function`, which is task 49's.
pub(crate) fn signature(function: Function) -> Option<Signature> {
    use Focus as F;
    Some(match function {
        Function::Empty | Function::Not => sig(&[], &[], BOOLEAN),
        Function::Exists => criteria(&[P_BOOLEAN], BOOLEAN),
        Function::SubsetOf => sig(&[], &[Param::FocusTypes(Some(Unordered))], BOOLEAN),
        Function::SupersetOf => sig(&[], &[Param::FocusTypes(None)], BOOLEAN),
        Function::IsDistinct => sig(&[], &[], BOOLEAN),
        Function::Distinct => sig(&[], &[], Yields::Focus),
        Function::Count => sig(&[], &[], INTEGER),
        // Upstream's `where` also narrows a `Reference` focus's targets when
        // the criteria is `resolve() is T`; targets are not carried, and
        // decide no verdict (`types.rs`).
        Function::Where => criteria(&[P_BOOLEAN], Yields::Focus),
        Function::Select => criteria(&[], Yields::Param(0)),
        Function::All => criteria(&[P_BOOLEAN], BOOLEAN),
        Function::AllTrue | Function::AnyTrue | Function::AllFalse | Function::AnyFalse => {
            criteria(&[], BOOLEAN)
        }
        Function::Repeat => Signature {
            params: Params::Repeat,
            warns_not_a_collection: false,
            focus: &[],
            param_types: &[],
            yields: Yields::Param(0),
        },
        Function::Aggregate => sig(&[], &[], Yields::Special(Special::Anything)),
        Function::Item => sig(
            &[F::Ordered("item")],
            &[P_INTEGER],
            Yields::FocusAs(Singleton),
        ),
        Function::As | Function::OfType => type_name(Yields::Special(Special::TypeNamed)),
        Function::Is => type_name(BOOLEAN),
        Function::Type => sig(&[], &[], Yields::Special(Special::TypeInfo)),
        Function::Single => sig(&[], &[], Yields::FocusAs(Singleton)),
        Function::First => subset(&[F::Ordered("first")], &[], Yields::FocusAs(Singleton)),
        Function::Last => subset(&[F::Ordered("last")], &[], Yields::FocusAs(Singleton)),
        Function::Tail => subset(&[F::Ordered("tail")], &[], Yields::Focus),
        Function::Skip => subset(&[F::Ordered("skip")], &[P_INTEGER], Yields::Focus),
        Function::Take => subset(&[F::Ordered("take")], &[P_INTEGER], Yields::Focus),
        Function::Union | Function::Combine => sig(&[], &[], Yields::UnionWithParam),
        Function::Intersect => sig(&[], &[], Yields::IntersectWithParam),
        Function::Exclude => sig(&[], &[], Yields::Focus),
        Function::Iif => sig(
            &[F::Singleton("iif")],
            &[P_BOOLEAN],
            Yields::Special(Special::Iif),
        ),
        Function::Lower => sig(&[F::String("lower")], &[], STRING),
        Function::Upper => sig(&[F::String("upper")], &[], STRING),
        Function::ToChars => sig(
            &[F::String("toChars")],
            &[],
            Yields::Types(Ordered, &[SYSTEM_STRING]),
        ),
        Function::IndexOf => sig(&[F::String("indexOf")], &[P_STRING], INTEGER),
        Function::Substring => sig(&[F::String("subString")], &[P_INTEGER, P_INTEGER], STRING),
        Function::StartsWith => sig(&[F::String("startsWith")], &[P_STRING], BOOLEAN),
        Function::EndsWith => sig(&[F::String("endsWith")], &[P_STRING], BOOLEAN),
        Function::Matches => sig(&[F::String("matches")], &[P_STRING], BOOLEAN),
        Function::MatchesFull => sig(&[F::String("matches")], &[P_STRING], BOOLEAN),
        Function::ReplaceMatches => sig(
            &[F::String("replaceMatches")],
            &[P_STRING, P_STRING],
            STRING,
        ),
        Function::Contains => sig(&[F::String("contains")], &[P_STRING], BOOLEAN),
        Function::Replace => sig(&[F::String("replace")], &[P_STRING, P_STRING], STRING),
        Function::Length => sig(&[F::Primitive("length", false)], &[], INTEGER),
        Function::Children => sig(&[], &[], Yields::Special(Special::Children)),
        Function::Descendants => sig(&[], &[], Yields::Special(Special::Descendants)),
        Function::MemberOf => sig(&[F::Coded("memberOf")], &[P_STRING], BOOLEAN),
        Function::Trace => sig(&[], &[P_STRING], Yields::Focus),
        Function::DefineVariable => sig(
            &[],
            &[Param::Types(Unordered, &[SYSTEM_STRING])],
            Yields::Special(Special::DefineVariable),
        ),
        Function::Check => sig(&[], &[P_STRING], Yields::Focus),
        Function::Today | Function::Now => {
            sig(&[], &[], Yields::Types(Singleton, &[SYSTEM_DATE_TIME]))
        }
        Function::Resolve => sig(
            &[F::Reference("resolve")],
            &[],
            Yields::TypesWithFocusStatus(&["Resource"]),
        ),
        Function::Extension => sig(&[], &[P_STRING], Yields::Special(Special::Extension)),
        Function::HasValue
        | Function::HtmlChecks1
        | Function::HtmlChecks2
        | Function::Comparable => sig(&[], &[], BOOLEAN),
        Function::Encode | Function::Decode | Function::Escape | Function::Unescape => {
            sig(&[], &[P_STRING], STRING)
        }
        Function::Trim => sig(&[], &[], STRING),
        Function::Split | Function::Join => sig(&[], &[P_STRING], STRING),
        Function::ToInteger => sig(&[F::Primitive("toInteger", true)], &[], INTEGER),
        Function::ToDecimal => sig(&[F::Primitive("toDecimal", true)], &[], DECIMAL),
        Function::ToString => sig(&[F::Primitive("toString", true)], &[], STRING),
        Function::ToQuantity => sig(
            &[F::Primitive("toQuantity", true)],
            &[],
            Yields::Types(Singleton, &[SYSTEM_QUANTITY]),
        ),
        Function::ToBoolean => sig(&[F::Primitive("toBoolean", false)], &[], BOOLEAN),
        Function::ToDateTime => sig(
            &[F::Primitive("ToDateTime", false)],
            &[],
            Yields::Types(Singleton, &[SYSTEM_DATE_TIME]),
        ),
        Function::ToTime => sig(
            &[F::Primitive("ToTime", false)],
            &[],
            Yields::Types(Singleton, &[SYSTEM_TIME]),
        ),
        Function::ConvertsToString => sig(&[F::Primitive("convertsToString", true)], &[], BOOLEAN),
        Function::ConvertsToQuantity => {
            sig(&[F::Primitive("convertsToQuantity", true)], &[], BOOLEAN)
        }
        Function::ConvertsToInteger => {
            sig(&[F::Primitive("convertsToInteger", false)], &[], BOOLEAN)
        }
        Function::ConvertsToDecimal => {
            sig(&[F::Primitive("convertsToDecimal", false)], &[], BOOLEAN)
        }
        Function::ConvertsToDateTime => {
            sig(&[F::Primitive("convertsToDateTime", false)], &[], BOOLEAN)
        }
        Function::ConvertsToDate => sig(&[F::Primitive("convertsToDate", false)], &[], BOOLEAN),
        // Upstream names this one by `toCode()`, which for `ConvertsToTime`
        // is `isTime`.
        Function::ConvertsToTime => sig(&[F::Primitive("isTime", false)], &[], BOOLEAN),
        Function::ConvertsToBoolean => {
            sig(&[F::Primitive("convertsToBoolean", false)], &[], BOOLEAN)
        }
        Function::ConformsTo => sig(&[], &[P_STRING], BOOLEAN),
        Function::Abs => sig(&[F::Numerical("abs")], &[], Yields::FocusAs(Singleton)),
        Function::Sort => sig(&[], &[], Yields::FocusAs(Ordered)),
        Function::Truncate => sig(&[F::Decimal("truncate")], &[], INTEGER),
        Function::Floor => sig(&[F::Decimal("floor")], &[], INTEGER),
        Function::Ceiling => sig(&[F::Decimal("ceiling")], &[], INTEGER),
        Function::Round => sig(&[F::Decimal("round")], &[P_INTEGER], DECIMAL),
        Function::Exp => sig(&[F::Numerical("exp")], &[], DECIMAL),
        Function::Ln => sig(&[F::Numerical("ln")], &[], DECIMAL),
        Function::Sqrt => sig(&[F::Numerical("sqrt")], &[], DECIMAL),
        Function::Log => sig(&[F::Numerical("log")], &[P_NUMBERS], DECIMAL),
        Function::Power => sig(
            &[F::Numerical("power")],
            &[P_NUMBERS],
            Yields::FocusAs(Singleton),
        ),
        Function::LowBoundary => sig(
            &[F::Continuous("lowBoundary", true)],
            &[P_INTEGER],
            Yields::Special(Special::Boundary),
        ),
        Function::HighBoundary => sig(
            &[F::Continuous("highBoundary", true)],
            &[P_INTEGER],
            Yields::Special(Special::Boundary),
        ),
        Function::Precision => sig(&[F::Continuous("precision", false)], &[], INTEGER),
        Function::HasTemplateIdOf => sig(&[], &[P_STRING], BOOLEAN),
        Function::Custom => return None,
    })
}
