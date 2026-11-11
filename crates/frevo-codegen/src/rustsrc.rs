//! Emitting Rust source that `rustfmt` already agrees with.
//!
//! `codegen.output.004` requires every generated file to be rustfmt-clean, and
//! [`crate::emit`] cannot shell out to `rustfmt` to get there: the output would
//! then depend on which toolchain happened to be installed, which
//! `codegen.output.001` and `codegen.output.002` forbid. So the generator
//! formats its own output, and this module is the part of `rustfmt`'s layout
//! that the generated tables actually exercise.
//!
//! Four rules cover it, all measured against the pinned toolchain's `rustfmt`
//! with the repository's (default) configuration:
//!
//! | Construct | Rule |
//! |---|---|
//! | an assignment whose right-hand side does not fit | the right-hand side moves to the next line, indented once, **before** anything inside it is broken |
//! | a call | one line while its argument list is at most [`FN_CALL_WIDTH`] wide **and** the line fits in [`MAX_WIDTH`]; otherwise one argument per line |
//! | an array | one line while the whole statement fits in [`MAX_WIDTH`]; otherwise filled — several items per line — while **every** item is at most [`SHORT_ITEM_WIDTH`] wide; otherwise one item per line |
//! | either, broken | a trailing comma after the last item |
//!
//! The two width thresholds are `rustfmt`'s `fn_call_width` and
//! `short_array_element_width_threshold` defaults, and they are not
//! derivable from `max_width`: a 40-column call argument list breaks even
//! though the line would fit, and an array of two-character strings fills
//! even though a longer item in the same array would not. Both are visible
//! in `crates/frevo-model/src/codec/provisional_r4.rs`, which was formatted
//! by `cargo fmt` rather than by this module.
//!
//! A string literal longer than the line is left alone, because `rustfmt`
//! leaves it alone: `format_strings` is off by default. The longest R4
//! snapshot path is 106 characters, so this is load-bearing rather than
//! theoretical.

/// `rustfmt`'s `max_width`.
pub const MAX_WIDTH: usize = 100;
/// `rustfmt`'s `fn_call_width`: the widest argument list that stays on one
/// line.
pub const FN_CALL_WIDTH: usize = 60;
/// `rustfmt`'s `array_width`: the widest array body that stays on one line,
/// however much room the line has left.
pub const ARRAY_WIDTH: usize = 60;
/// `rustfmt`'s `short_array_element_width_threshold`: an array whose every
/// item is at most this wide is filled rather than listed one per line.
pub const SHORT_ITEM_WIDTH: usize = 10;
/// One level of indentation.
pub const INDENT: usize = 4;

/// One item of an emitted array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// Anything that is already one token-ish run: a string literal, a path,
    /// a number.
    Atom(String),
    /// `name(arg, arg, ...)`, which breaks one argument per line when its
    /// argument list is too wide.
    Call(String, Vec<String>),
}

impl Item {
    /// A string literal item.
    pub fn string(value: &str) -> Item {
        Item::Atom(quote(value))
    }

    /// A call item.
    pub fn call(name: &str, args: Vec<String>) -> Item {
        Item::Call(name.to_string(), args)
    }

    /// The item on one line, however wide that is.
    pub fn one_line(&self) -> String {
        match self {
            Item::Atom(text) => text.clone(),
            Item::Call(name, args) => format!("{name}({})", args.join(", ")),
        }
    }

    /// The item at `indent`, broken across lines if `rustfmt` would break it.
    /// The result carries its own leading indentation and no trailing newline.
    fn render(&self, indent: usize) -> String {
        let one_line = self.one_line();
        match self {
            Item::Atom(_) => format!("{}{one_line}", pad(indent)),
            Item::Call(name, args) => {
                let arguments = args.join(", ");
                // Strictly less than the margin, because an item inside an
                // array is followed by a comma.
                if arguments.len() <= FN_CALL_WIDTH
                    && indent + one_line.len() < MAX_WIDTH
                    && !one_line.contains('\n')
                {
                    return format!("{}{one_line}", pad(indent));
                }
                let mut out = format!("{}{name}(\n", pad(indent));
                for argument in args {
                    out.push_str(&format!("{}{argument},\n", pad(indent + INDENT)));
                }
                out.push_str(&format!("{})", pad(indent)));
                out
            }
        }
    }
}

/// `n` spaces.
pub fn pad(n: usize) -> String {
    " ".repeat(n)
}

/// A Rust string literal for `value`.
///
/// Only the two escapes FHIR content can contain are needed — a backslash and
/// a double quote, both of which occur in the primitive value regexes — plus
/// the control characters a regex spells literally.
pub fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// A whole statement whose right-hand side is one value:
/// `{prefix}{value}{suffix}`, laid out the way `rustfmt` would.
///
/// `prefix` carries everything up to the value, ending in `= `.
pub fn scalar_statement(indent: usize, prefix: &str, value: &str, suffix: &str) -> String {
    let single = format!("{}{prefix}{value}{suffix}", pad(indent));
    if single.len() <= MAX_WIDTH {
        return format!("{single}\n");
    }
    // `rustfmt` moves the right-hand side down a line before it gives up; a
    // string literal it still cannot fit is left over the margin, because
    // `format_strings` is off.
    format!(
        "{}{}\n{}{value}{suffix}\n",
        pad(indent),
        prefix.trim_end(),
        pad(indent + INDENT)
    )
}

/// A whole statement whose value is an array literal:
/// `{prefix}&[{items}]{suffix}`, laid out the way `rustfmt` would.
///
/// `prefix` carries everything up to the `&[`, for instance
/// `const T_DATE: &[&str] = `, and `suffix` everything after the `]`, usually
/// `;`.
pub fn array_statement(indent: usize, prefix: &str, items: &[Item], suffix: &str) -> String {
    let inner_indent = indent + INDENT;
    let one_line_items: Vec<String> = items.iter().map(Item::one_line).collect();
    let body = one_line_items.join(", ");
    // An array body wider than `array_width` is broken however much room the
    // line has left, and an array body narrower than it is never broken —
    // only moved down a line, which is [`scalar_statement`]'s rule.
    //
    // A one-item array is the exception: `rustfmt` keeps it horizontal
    // whatever `array_width` says, because breaking it out gains nothing, and
    // only `max_width` moves it.
    if body.len() <= ARRAY_WIDTH || items.len() <= 1 {
        return scalar_statement(indent, prefix, &format!("&[{body}]"), suffix);
    }

    let mut out = format!("{}{prefix}&[\n", pad(indent));
    if !one_line_items.is_empty()
        && one_line_items
            .iter()
            .all(|item| item.len() <= SHORT_ITEM_WIDTH)
    {
        // Filled: as many items per line as fit.
        let mut line = pad(inner_indent);
        let mut empty = true;
        for item in &one_line_items {
            let addition = if empty {
                item.clone()
            } else {
                format!(" {item}")
            };
            if !empty && line.len() + addition.len() + 1 > MAX_WIDTH {
                out.push_str(&line);
                out.push('\n');
                line = pad(inner_indent);
                line.push_str(item);
                line.push(',');
            } else {
                line.push_str(&addition);
                line.push(',');
                empty = false;
            }
        }
        if !empty || !line.trim().is_empty() {
            out.push_str(&line);
            out.push('\n');
        }
    } else {
        for item in items {
            out.push_str(&item.render(inner_indent));
            out.push_str(",\n");
        }
    }
    out.push_str(&format!("{}]{suffix}\n", pad(indent)));
    out
}
