//! The regular-expression string functions: `matches`, `matchesFull` and
//! `replaceMatches`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094: `funcReplaceMatches` at
//! `:4865` (the call at `:4879`), `funcMatches` at `:5683` (the call at
//! `:5698`) and `funcMatchesFull` at `:5711` (the call at `:5725`).
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the pinned engine calls `java.util.regex`
//! through `RegexTimeout`; this file uses the `regex` crate (ADR 0005)
//! behind a Java-compatibility layer, and no wall-clock timeout, because the
//! engine is linear in the haystack.
//!
//! What the pinned engine does here and is kept:
//!
//! - `matches` prefixes `(?s)` and finds anywhere; `matchesFull` prefixes
//!   `(?s)` and matches the whole string (`\A(?:...)\z` here); `replaceMatches`
//!   gets **no** `(?s)`, so `.` does not match a line terminator in it;
//! - `matches` answers empty for an empty focus or an empty pattern
//!   collection; `matchesFull` has no such branch and answers `false`;
//! - an empty-string pattern is `false` for the two predicates and returns the
//!   input unchanged from `replaceMatches` (`Utilities.noString(regex)`);
//! - an empty-string focus is `false` for the predicates;
//! - a focus that is not string-like answers empty (`DEV-0006`), except that
//!   `replaceMatches` with an empty-string pattern or a multi-item focus
//!   returns the first item's string with no guard;
//! - a pattern Java cannot compile is an evaluation error.
//!
//! The Java-compatibility layer ([`translate`]): `\d \D \w \W \s \S \b \B` are
//! rewritten to their ASCII meanings, `\<` and `\>` to literals, and `.` in
//! `replaceMatches` to the class that excludes Java's five line terminators;
//! constructs whose `regex` meaning silently differs from Java's (the inline
//! flags `U`, `u` and `x`, `\v`, possessive quantifiers, POSIX-named `\p{..}`
//! properties, `--` and `~~` in a class) and an unbalanced parenthesis are
//! errors. Replacement strings use Java's `$n`, `${name}` and `\` syntax, and
//! empty matches are found as Java's `Matcher.find` finds them.
//!
//! After an empty match the search steps one UTF-16 unit, so it can split a
//! supplementary character: only then is the text copied and the character
//! replaced by its two lone halves (the plane-16 encoding of `strfn.rs`), the
//! ordinary path allocating nothing. The pattern still sees a lone half as a
//! private-use character, and `.` matches a whole character, not a unit.
//!
//! Known remaining differences are `DEBT-0015`.

use std::cell::RefCell;

use regex::{Regex, RegexBuilder};

use crate::seam::value::Value;
use crate::syntax::ast::{Function, NodeId};

use super::collection::{convert_list_to_string, convert_to_string};
use super::operand::{STRING_TYPES, has_type};
use super::strfn::{LONE_BASE, from_utf16, lone_unit, utf16};
use super::{Context, EvalError, Run};

/// The most compiled-program memory a pattern may use, in bytes.
const SIZE_LIMIT: usize = 2 << 20;
/// The deepest group nesting a pattern may have.
const NEST_LIMIT: u32 = 100;
/// The compiled patterns kept per thread.
const CACHE_SIZE: usize = 64;

/// How the pattern is wrapped, which is what distinguishes the three functions.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// `(?s)` + pattern, found anywhere (`matches`).
    Find,
    /// `(?s)` + pattern, matching the whole input (`matchesFull`).
    Full,
    /// The pattern as given (`replaceMatches`).
    Replace,
}

type Compiled = Result<Regex, String>;

thread_local! {
    static CACHE: RefCell<Vec<(Mode, String, Compiled)>> = const { RefCell::new(Vec::new()) };
}

fn compile(mode: Mode, pattern: &str) -> Compiled {
    let hit = CACHE.with(|c| {
        c.borrow()
            .iter()
            .find(|(m, p, _)| *m == mode && p == pattern)
            .map(|(_, _, r)| r.clone())
    });
    if let Some(r) = hit {
        return r;
    }
    let built = build(mode, pattern);
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= CACHE_SIZE {
            drop(c.remove(0));
        }
        c.push((mode, pattern.to_owned(), built.clone()));
    });
    built
}

fn build(mode: Mode, pattern: &str) -> Compiled {
    let body = translate(pattern, mode == Mode::Replace)?;
    let text = match mode {
        Mode::Find => format!("(?s){body}"),
        Mode::Full => format!(r"(?s)\A(?:{body})\z"),
        Mode::Replace => body,
    };
    RegexBuilder::new(&text)
        .size_limit(SIZE_LIMIT)
        .nest_limit(NEST_LIMIT)
        .build()
        .map_err(|e| e.to_string())
}

/// The class `.` stands for when Java's default (no `s` flag) is in force.
const JAVA_DOT: &str = r"[^\n\r\x{85}\x{2028}\x{2029}]";
const SPACE: &str = r" \t\n\x0B\x0C\r";
/// Java's POSIX-named properties, which are ASCII in Java and Unicode here.
const POSIX: [&str; 13] = [
    "Lower", "Upper", "ASCII", "Alpha", "Digit", "Alnum", "Punct", "Graph", "Print", "Blank",
    "Cntrl", "XDigit", "Space",
];

fn unsupported(what: &str) -> String {
    format!("unsupported regular expression construct: {what}")
}

/// Rewrites a Java pattern into one whose `regex` meaning is Java's, or says
/// why it cannot.
fn translate(pattern: &str, java_dot: bool) -> Result<String, String> {
    let chars: Vec<char> = pattern.chars().collect();
    // A pattern with an inline flag group may switch `s` on or off, so `.` is
    // left alone.
    let dot_class = java_dot && !has_flag_group(&chars);
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut i = 0;
    let mut class_depth = 0usize;
    let mut group_depth = 0usize;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        if class_depth > 0 {
            match c {
                '\\' => {
                    let e = *chars.get(i).ok_or("trailing backslash")?;
                    i += 1;
                    escape(e, &chars, &mut i, true, &mut out)?;
                }
                '[' => {
                    class_depth += 1;
                    out.push(c);
                }
                ']' => {
                    class_depth -= 1;
                    out.push(c);
                }
                '-' | '~' if chars.get(i) == Some(&c) => {
                    return Err(unsupported(&format!("`{c}{c}` inside a character class")));
                }
                _ => out.push(c),
            }
            continue;
        }
        match c {
            '\\' => {
                let e = *chars.get(i).ok_or("trailing backslash")?;
                i += 1;
                escape(e, &chars, &mut i, false, &mut out)?;
            }
            '[' => {
                class_depth = 1;
                out.push(c);
                if chars.get(i) == Some(&'^') {
                    out.push('^');
                    i += 1;
                }
            }
            '(' => {
                out.push('(');
                if chars.get(i) == Some(&'?') && chars.get(i + 1).is_some_and(|f| flag_char(*f)) {
                    let mut j = i + 1;
                    let mut flags = String::new();
                    while let Some(f) = chars.get(j).filter(|f| flag_char(**f)) {
                        flags.push(*f);
                        j += 1;
                    }
                    for bad in ['U', 'u', 'x'] {
                        if flags.contains(bad) {
                            return Err(unsupported(&format!("the inline flag `{bad}`")));
                        }
                    }
                    out.push('?');
                    out.push_str(&flags);
                    i = j;
                    if chars.get(i) == Some(&':') {
                        group_depth += 1;
                    } else if chars.get(i) != Some(&')') {
                        return Err("malformed inline flag group".to_owned());
                    }
                } else {
                    group_depth += 1;
                }
            }
            ')' => {
                group_depth = group_depth.checked_sub(1).ok_or("unmatched `)`")?;
                out.push(')');
            }
            '*' | '+' | '?' => {
                out.push(c);
                quantifier_tail(&chars, &mut i, &mut out)?;
            }
            '{' => {
                out.push('{');
                if let Some(end) = quantifier_end(&chars, i) {
                    out.extend(&chars[i..=end]);
                    i = end + 1;
                    quantifier_tail(&chars, &mut i, &mut out)?;
                }
            }
            '.' if dot_class => out.push_str(JAVA_DOT),
            _ => out.push(c),
        }
    }
    if class_depth > 0 {
        return Err("unclosed character class".to_owned());
    }
    if group_depth > 0 {
        return Err("unclosed group".to_owned());
    }
    Ok(out)
}

fn flag_char(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '-'
}

/// Whether the pattern has an inline flag group such as `(?i)` or `(?s:`.
fn has_flag_group(chars: &[char]) -> bool {
    chars
        .windows(3)
        .any(|w| w[0] == '(' && w[1] == '?' && flag_char(w[2]))
}

/// The index of the `}` closing a `{n}`, `{n,}` or `{n,m}` quantifier whose
/// body starts at `from`, or `None` when the brace is not one.
fn quantifier_end(chars: &[char], from: usize) -> Option<usize> {
    let mut j = from;
    let digits = |j: &mut usize| {
        let start = *j;
        while chars.get(*j).is_some_and(char::is_ascii_digit) {
            *j += 1;
        }
        *j > start
    };
    if !digits(&mut j) {
        return None;
    }
    if chars.get(j) == Some(&',') {
        j += 1;
        digits(&mut j);
    }
    (chars.get(j) == Some(&'}')).then_some(j)
}

/// After a quantifier: a `?` makes it lazy, a `+` would make it possessive.
fn quantifier_tail(chars: &[char], i: &mut usize, out: &mut String) -> Result<(), String> {
    match chars.get(*i) {
        Some('?') => {
            out.push('?');
            *i += 1;
        }
        Some('+') => return Err(unsupported("a possessive quantifier")),
        _ => {}
    }
    Ok(())
}

/// One backslash escape; `e` is the character after the backslash.
fn escape(
    e: char,
    chars: &[char],
    i: &mut usize,
    in_class: bool,
    out: &mut String,
) -> Result<(), String> {
    let set = |positive: &str, negated: bool, out: &mut String| {
        if negated {
            out.push_str("[^");
            out.push_str(positive);
            out.push(']');
        } else if in_class {
            out.push_str(positive);
        } else {
            out.push('[');
            out.push_str(positive);
            out.push(']');
        }
    };
    match e {
        'd' | 'D' => set("0-9", e == 'D', out),
        'w' | 'W' => set("0-9A-Za-z_", e == 'W', out),
        's' | 'S' => set(SPACE, e == 'S', out),
        'b' if !in_class => out.push_str(r"(?-u:\b)"),
        'B' if !in_class => out.push_str(r"(?-u:\B)"),
        'v' => return Err(unsupported(r"`\v`")),
        '<' | '>' => out.push(e),
        'p' | 'P' => {
            out.push('\\');
            out.push(e);
            if chars.get(*i) == Some(&'{') {
                let close = chars[*i..]
                    .iter()
                    .position(|c| *c == '}')
                    .ok_or("unclosed property name")?;
                let name: String = chars[*i + 1..*i + close].iter().collect();
                if POSIX.contains(&name.as_str()) {
                    return Err(unsupported(&format!(
                        r"the POSIX property `\{e}{{{name}}}`"
                    )));
                }
                out.extend(&chars[*i..=*i + close]);
                *i += close + 1;
            }
        }
        _ => {
            out.push('\\');
            out.push(e);
        }
    }
    Ok(())
}

/// `Matcher.replaceAll`: every match, in Java's order, with Java's
/// replacement syntax.
fn replace_all(re: &Regex, text: &str, replacement: &str) -> Result<String, String> {
    let groups = re.captures_len() - 1;
    let mut out = String::with_capacity(text.len());
    let mut work = std::borrow::Cow::Borrowed(text);
    let mut split = false;
    let mut copied = 0;
    let mut from = 0;
    while let Some(caps) = re.captures_at(&work, from) {
        let Some(whole) = caps.get(0) else { break };
        let (start, end) = (whole.start(), whole.end());
        out.push_str(&work[copied..start]);
        expand(re, groups, &caps, replacement, &mut out)?;
        copied = end;
        from = end;
        if start == end {
            // `Matcher.find` steps one UTF-16 unit past an empty match. A
            // supplementary character is two units, so stepping into it
            // splits it: the character becomes its two lone halves (each the
            // same four bytes, so offsets hold) and the search resumes
            // between them, with the surrounding text still in view.
            let Some(c) = work[from..].chars().next() else {
                break;
            };
            if u32::from(c) >= 0x10000 && lone_unit(c).is_none() {
                let mut units = [0u16; 2];
                c.encode_utf16(&mut units);
                let half = |u: u16| {
                    char::from_u32(LONE_BASE + u32::from(u) - 0xD800)
                        .unwrap_or(char::REPLACEMENT_CHARACTER)
                };
                let mut halves = String::with_capacity(8);
                halves.push(half(units[0]));
                halves.push(half(units[1]));
                work.to_mut().replace_range(from..from + 4, &halves);
                split = true;
                from += 4;
            } else {
                from += c.len_utf8();
            }
        }
    }
    out.push_str(&work[copied..]);
    if split {
        // Halves left side by side are the pair they were.
        out = from_utf16(&utf16(&out));
    }
    Ok(out)
}

/// `Matcher.appendReplacement` for one match.
fn expand(
    re: &Regex,
    groups: usize,
    caps: &regex::Captures<'_>,
    replacement: &str,
    out: &mut String,
) -> Result<(), String> {
    let chars: Vec<char> = replacement.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        match c {
            '\\' => {
                let next = chars.get(i).ok_or("character to be escaped is missing")?;
                out.push(*next);
                i += 1;
            }
            '$' => {
                let next = *chars
                    .get(i)
                    .ok_or("Illegal group reference: group index is missing")?;
                let index = if next == '{' {
                    i += 1;
                    let start = i;
                    while chars.get(i).is_some_and(|c| c.is_ascii_alphanumeric()) {
                        i += 1;
                    }
                    if i == start || !chars[start].is_ascii_alphabetic() {
                        return Err("named capturing group is missing trailing '}'".to_owned());
                    }
                    let name: String = chars[start..i].iter().collect();
                    if chars.get(i) != Some(&'}') {
                        return Err("named capturing group is missing trailing '}'".to_owned());
                    }
                    i += 1;
                    re.capture_names()
                        .position(|n| n == Some(name.as_str()))
                        .ok_or_else(|| format!("No group with name {{{name}}}"))?
                } else {
                    let Some(first) = next.to_digit(10) else {
                        return Err("Illegal group reference".to_owned());
                    };
                    i += 1;
                    let mut n = first as usize;
                    if n > groups {
                        return Err(format!("No group {n}"));
                    }
                    // Further digits are part of the number while it still
                    // names a group.
                    while let Some(d) = chars.get(i).and_then(|c| c.to_digit(10)) {
                        let wider = n * 10 + d as usize;
                        if wider > groups {
                            break;
                        }
                        n = wider;
                        i += 1;
                    }
                    n
                };
                if let Some(m) = caps.get(index) {
                    out.push_str(m.as_str());
                }
            }
            _ => out.push(c),
        }
    }
    Ok(())
}

impl Run<'_> {
    /// `evaluateFunction` for the regular-expression functions.
    pub(super) fn func_regex(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
        function: Function,
    ) -> Result<Vec<Value>, EvalError> {
        // The regex failures are `PatternSyntaxException`s, which carry no
        // position (`path.errors.002`).
        let fail = |message: String| EvalError::plain(message);
        let p0 = self.param(id, 0)?;
        let pattern_items = self.execute(context, focus, p0, true)?;
        let pattern = convert_list_to_string(&pattern_items);
        let guard = |one: &Value| self.implicit_string_conversion || has_type(one, &STRING_TYPES);

        if function == Function::ReplaceMatches {
            let p1 = self.param(id, 1)?;
            let replacement_items = self.execute(context, focus, p1, true)?;
            let replacement = convert_list_to_string(&replacement_items);
            let Some(first) = focus.first() else {
                return Ok(Vec::new());
            };
            if pattern_items.is_empty() || replacement_items.is_empty() {
                return Ok(Vec::new());
            }
            if focus.len() == 1 && !pattern.is_empty() {
                if !guard(first) {
                    return Ok(Vec::new());
                }
                let re = compile(Mode::Replace, &pattern)
                    .map_err(|e| fail(format!("Invalid regular expression {pattern}: {e}")))?;
                let done = replace_all(&re, &convert_to_string(first), &replacement)
                    .map_err(|e| fail(format!("Error replacing with {replacement}: {e}")))?;
                return Ok(vec![Value::String(done)]);
            }
            return Ok(vec![Value::String(convert_to_string(first))]);
        }

        let full = function == Function::MatchesFull;
        if !full && (focus.is_empty() || pattern_items.is_empty()) {
            return Ok(Vec::new());
        }
        let [one] = focus else {
            return Ok(vec![Value::Boolean(false)]);
        };
        if pattern.is_empty() {
            return Ok(vec![Value::Boolean(false)]);
        }
        if !guard(one) {
            return Ok(Vec::new());
        }
        let text = convert_to_string(one);
        if text.is_empty() {
            return Ok(vec![Value::Boolean(false)]);
        }
        let mode = if full { Mode::Full } else { Mode::Find };
        let re = compile(mode, &pattern)
            .map_err(|e| fail(format!("Invalid regular expression {pattern}: {e}")))?;
        Ok(vec![Value::Boolean(re.is_match(&text))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replaced(pattern: &str, text: &str, with: &str) -> Result<String, String> {
        replace_all(&compile(Mode::Replace, pattern)?, text, with)
    }

    fn found(mode: Mode, pattern: &str, text: &str) -> Result<bool, String> {
        Ok(compile(mode, pattern)?.is_match(text))
    }

    // covers: path.string.010
    #[test]
    fn empty_matches_are_found_as_java_finds_them() {
        assert_eq!(replaced("b*", "abc", "-").unwrap(), "-a--c-");
        assert_eq!(replaced("x*", "", "-").unwrap(), "-");
    }

    // covers: path.string.010
    #[test]
    fn java_replacement_syntax() {
        assert_eq!(replaced("(a)(b)", "ab", "$2$1").unwrap(), "ba");
        assert_eq!(replaced("(?<x>a)", "a", "${x}${x}").unwrap(), "aa");
        assert_eq!(replaced("a", "a", r"\$1").unwrap(), "$1");
        assert_eq!(replaced("(a)", "a", "$10").unwrap(), "a0");
        assert!(replaced("a", "a", "$1").is_err());
        assert!(replaced("a", "a", "$").is_err());
        assert!(replaced("a", "a", r"\").is_err());
        // No match: the replacement is never read.
        assert_eq!(replaced("z", "a", "$9").unwrap(), "a");
    }

    // covers: path.string.008
    #[test]
    fn classes_are_ascii_as_in_java() {
        assert!(!found(Mode::Find, r"\d", "\u{0661}").unwrap());
        assert!(found(Mode::Find, r"\d", "7").unwrap());
        assert!(!found(Mode::Find, r"\w", "\u{e9}").unwrap());
        assert!(!found(Mode::Find, r"[\w]", "\u{e9}").unwrap());
        assert!(found(Mode::Find, r"[^\D]", "5").unwrap());
        assert!(found(Mode::Find, r"\s", "\u{b}").unwrap());
        assert!(!found(Mode::Find, r"\s", "\u{a0}").unwrap());
        assert!(found(Mode::Find, r"a\b", "a b").unwrap());
        assert!(found(Mode::Find, r"\bb", "\u{e9}b").unwrap());
        assert!(found(Mode::Find, r"\<", "<").unwrap());
    }

    // covers: path.string.008, path.string.010
    #[test]
    fn dot_excludes_line_terminators_only_without_dotall() {
        assert_eq!(replaced(".", "a\rb", "x").unwrap(), "x\rx");
        assert_eq!(replaced(".", "a\u{2028}", "x").unwrap(), "x\u{2028}");
        assert!(found(Mode::Find, "a.b", "a\rb").unwrap());
        assert!(found(Mode::Full, "a.b", "a\nb").unwrap());
    }

    // covers: path.string.009
    #[test]
    fn full_match_cannot_be_escaped_by_the_pattern() {
        assert!(compile(Mode::Full, "a)|(b").is_err());
        assert!(compile(Mode::Full, "(a").is_err());
        assert!(!found(Mode::Full, "a|ab", "abc").unwrap());
        assert!(found(Mode::Full, "a|ab", "ab").unwrap());
    }

    // covers: path.string.008
    #[test]
    fn constructs_that_would_silently_differ_are_rejected() {
        for p in [
            "a*+",
            "a++",
            "a?+",
            "a{1,2}+",
            r"\v",
            r"\p{Alpha}",
            r"\P{Lower}",
            "(?u)a",
            "(?U)a",
            "(?x)a",
            "[a--b]",
            "[a~~b]",
        ] {
            assert!(compile(Mode::Find, p).is_err(), "{p}");
        }
        // The lazy form is a different, accepted, construct.
        assert!(found(Mode::Find, "a+?", "a").unwrap());
        assert!(found(Mode::Find, r"\p{L}", "\u{e9}").unwrap());
        assert!(found(Mode::Find, "a{2}", "aa").unwrap());
    }

    // covers: path.string.008
    #[test]
    fn constructs_the_engine_lacks_are_errors() {
        for p in [
            r"(a)\1", "(?=a)a", "(?<!a)b", "(?>a)", r"\Qa\E", r"\R", r"\G", "(?d)a",
        ] {
            assert!(compile(Mode::Find, p).is_err(), "{p}");
        }
    }

    // covers: path.string.008
    #[test]
    fn a_pattern_over_the_limits_is_an_error_not_a_panic() {
        assert!(compile(Mode::Find, "(a{1000}){1000}").is_err());
        let deep = format!("{}a{}", "(".repeat(200), ")".repeat(200));
        assert!(compile(Mode::Find, &deep).is_err());
    }
}
