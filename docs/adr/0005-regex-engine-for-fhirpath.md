# 5. The regular-expression engine for FHIRPath

- **Status:** PROPOSED. Needs owner approval. Constitution section 5 requires
  an ADR and the owner's approval for any new runtime dependency, so no crate
  is added to any `Cargo.toml` until this ADR reads `accepted`. Task 41a of
  `add-fhirpath-and-codegen-r4` stays blocked until then.
- **Date:** 2026-11-17
- **Context phase:** Phase 1, `add-fhirpath-and-codegen-r4` task 41a

## Context

`frevo-path` has no dependencies today. ADR 0004 (on the change branch, not
yet on `main`) decided to write the engine ourselves. The string functions
`matches`, `matchesFull` and `replaceMatches` need a regular-expression
engine. Task 41 was split so that those three became task 41a, blocked on
this ADR.

### What the reference does

All line numbers below refer to `org.hl7.fhir.core` at
`73c432b5f43ddbce979978a760ec4aa81e929094`, the pin in `reference/PINS.md`.
`FHIRPathEngine.java` means
`org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`.
`RegexTimeout.java` and `RegexConstants.java` are in
`org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/regex/`.

Only these three functions accept a pattern from the expression. Each one
goes through `RegexTimeout`, which is a wrapper around `java.util.regex`.

| Function | Call | Pattern actually compiled | Timeout |
|---|---|---|---|
| `matches` (`funcMatches`, `:5683-5706`) | `RegexTimeout.find` = `Pattern.compile(p).matcher(s).find()` (`:5698`) | `"(?s)" + p` | `regexTimeoutMillis`, default 500 ms (`:186`) |
| `matchesFull` (`funcMatchesFull`, `:5708-5735`) | `RegexTimeout.matches` = `Pattern.matches(p, s)`, a whole-input match (`:5725`) | `"(?s)" + p` | `regexTimeoutMillis` |
| `replaceMatches` (`funcReplaceMatches`, `:4865-4889`) | `RegexTimeout.replaceAll` = `Pattern.compile(p).matcher(s).replaceAll(r)` (`:4879`) | `p` as given, **no** `(?s)` | the 3-argument overload, so always `DEFAULT_TIMEOUT` = 500 ms. The `regexTimeoutMillis` setter has no effect here |

The rest of the behaviour depends on the function wrappers, not on the
engine:

- `matches` returns empty if the focus or the pattern is empty. It returns
  `false` for an empty string focus, an empty-string pattern, or a focus with
  more than one item.
- `matchesFull` has no empty-propagation branch. `{}` as focus or as pattern
  gives `false`, not empty.
- `replaceMatches` with an empty-string pattern returns the input unchanged,
  because of the `Utilities.noString(regex)` guard. That is why
  `'abc'.replaceMatches('', 'x')` gives `abc` (`testReplaceMatches2`). Java's
  `replaceAll` alone would give `xaxbxcx`.
- A pattern Java cannot compile throws `PatternSyntaxException`.
  `executeWithTimeout` rewraps it as `RuntimeException`, so it ends as an
  evaluation error.
- A timeout raises `FHIRException("Timeout evaluating regex: " + p)`.

**ReDoS in HAPI.** The caller's premise was that HAPI is unbounded. That is
not quite right. HAPI bounds each call by wall-clock time, not by steps:

- `executeWithTimeout` creates a new single-thread `ExecutorService` for every
  call, waits up to the timeout, then calls `future.cancel(true)`.
- The cancellation works because `InterruptibleCharSequence.charAt` checks
  `Thread.interrupted()`.
- So a catastrophic pattern costs a thread and up to about 500 ms of CPU for
  each evaluation, and then fails with an error.
- Whether a borderline pattern finishes or times out depends on machine load,
  so the outcome is not deterministic.
- This matters because both sides can come from outside:
  - Patterns come from FHIRPath expressions, which can arrive in client-posted
    `SearchParameter` resources and in profile invariants.
  - Haystacks are resource content.

**Fixed patterns.** There are three more regular expressions, all fixed:

- `DATE_TIME_REGEX` (`RegexConstants.java:27-30`), used by `funcIsDateTime`
  and `funcIsDate`.
- The inline time pattern in `funcIsTime` (`:5993`).

They use `String.matches` with no timeout. They contain only ASCII ranges,
bounded alternation and optional groups. On the change branch,
`crates/frevo-path/src/eval/convert.rs` already implements them as
hand-written matchers over the same language, with no crate. They are outside
this decision.

### What the pinned suite exercises

`oracles/fhirpath/tests-fhir-r4.xml` has 23 regex cases. There are 16 in
`testMatches` (`matches` and `matchesFull`) and 7 in `testReplaceMatches`.
`hapi-8.12.0-results.json` records all 23 as `passed`. One more case,
`testMatchesUnicodeCharacters` (`'🔥🔥🔥'.matches('🔥+')`), is commented out
of the suite.

The patterns use very little syntax:

- literals: `FHIR`, `fhir`, `Library`, `234`, `2`
- `.*`
- `^…$`
- the class `[0-9]`
- the empty pattern
- `testMatchesSingleLineMode1`, which needs `.` to match a newline (the
  `(?s)` prefix)

None uses a backreference, lookaround, a possessive quantifier, `\d`, `\w`,
`(?i)` or a replacement group reference. Every engine considered below passes
all 23. The choice therefore rests on behaviour outside the suite: the
differential against live HAPI, and safety.

### What the change's spec requires

`openspec/changes/add-fhirpath-and-codegen-r4/specs/path/spec.md` on the
change branch states no regex-specific step or time bound. These requirements
apply:

- `path.errors.003`: evaluation never panics.
- `path.errors.004`: evaluation is bounded for an adversarial expression,
  through a depth or step limit.
- `path.perf.001` and `path.perf.002`: compiling an expression is at most
  20 µs, and a compiled expression is not parsed again.
- `path.perf.003`: extracting all search parameters is at most 300 µs per
  resource.

A wall-clock timeout on a spawned thread fits none of these. A regex
evaluation must be bounded by construction or by a step budget.

### The lockfile, checked

The task-42 implementer claimed `fancy-regex` is already in `Cargo.lock`.
**That is wrong.** Neither `regex` nor `fancy-regex` appears in `Cargo.lock`
on `main` (`48c7214`) or on `change/add-fhirpath-and-codegen-r4` (`c2bfc3f`).
`Cargo.lock` does already contain these crates, all pulled in by
`tracing-subscriber` 0.3.23 with `env-filter`, through `matchers` 0.2.0:

- `regex-automata` 0.4.18
- `regex-syntax` 0.8.11
- `aho-corasick` 1.1.5
- `memchr` 2.8.3

### Constraints

From `deny.toml`:

- Licence allow-list: Apache-2.0, MIT, BSD, ISC, Zlib, Unicode-3.0, CC0,
  Unlicense, BSL-1.0.
- `wildcards = "deny"`.
- `multiple-versions = "warn"`.
- Registry sources only.

From the workspace:

- `rust-version = "1.85"`.
- Crates use `#![forbid(unsafe_code)]`.

Two more constraints:

- A licence condition on the in-crate option. OpenJDK's `java.util.regex` is
  GPL-2.0 with the Classpath Exception, which this Apache-2.0 repository
  cannot take. A faithful Java engine can only be a clean-room
  reimplementation, never a port.
- The decision will be reused. `frevo-validator` will need a regex engine for
  the same Java semantics: primitive value regexes, `ElementDefinition`
  patterns, and profile invariants that call `matches()`. What is chosen here
  sets the precedent there.

## Options considered

Crate facts are from crates.io and docs.rs on 2026-11-17.

### (a) `regex` 1.13.1, recommended

- **Licence:** MIT OR Apache-2.0.
- **MSRV:** 1.65.
- **Repository:** rust-lang/regex.
- **Dependencies:** `regex-automata` ^0.4.16, `regex-syntax` ^0.8.11, and
  optionally `aho-corasick` and `memchr`. All four are already in the graph
  at satisfying versions, so the lockfile gains exactly one package, `regex`
  itself, from the same maintainers.
- **Complexity:** search is worst case O(m·n), so there is no catastrophic
  backtracking. Pattern size and nesting are capped at compile time
  (`RegexBuilder::size_limit`, `nest_limit`), and those caps fail as
  compile errors.
- **Caveat:** iterating over all matches, as `replace_all` does, can degrade
  to O(m·n²) on adversarial patterns.

What it lacks compared with Java:

- No backreferences.
- No lookahead or lookbehind.
- No atomic groups.
- No `\Q…\E`, `\R`, `\X`, `\G`, `\Z` or `\h`.
- No `(?d)` flag.
- All of these are compile errors, so the failure is loud. Each was checked
  against `regex` 1.13.1 with a probe binary on 2026-11-17.

The probe also showed that possessive quantifiers are **not** rejected: `a*+`
compiles, as `(?:a*)+`. In Java, `a*+` never gives back what it matched, so
`a*+a` can never match. In Rust it can. The difference is silent.

Differences that are silent unless handled:

- `\d`, `\w`, `\s` and `\b` are Unicode-aware. Java's defaults are ASCII.
- `(?i)` uses Unicode simple case folding. Java's default is ASCII only.
- `$` without `m` matches only at the end of input. Java also matches before
  a final line terminator.
- Without `s`, `.` excludes only `\n`. Java also excludes `\r`, `\u0085`,
  ` ` and ` `. This matters for `replaceMatches`, which gets no
  `(?s)`.
- The inline flag letter `U` means "swap greed" in Rust and
  `UNICODE_CHARACTER_CLASS` in Java. `u` turns on Unicode mode in Rust, which
  is already on by default, and turns on Unicode case folding in Java. Both
  compile in Rust.
- `\v` is only a vertical tab in Rust (the probe showed `\v` does not match
  `\n`). In Java it is any vertical whitespace.
- Possessive quantifiers (`*+`, `++`, `?+`, `{n,m}+`), described above.
- `\p{Alpha}` and the other POSIX-named properties are Unicode in Rust and
  ASCII in Java.
- `--` and `~~` inside a class are set operators in Rust and literal
  characters in Java.

Replacement strings:

| | Rust | Java |
|---|---|---|
| Literal `$` | `$$` | `\$` |
| `$1a` | the group named `1a` | group 1, then `a` |
| Missing group | replaced with the empty string | throws `IndexOutOfBoundsException` |

### (b) `fancy-regex` 0.20.2

- **Licence:** MIT.
- **MSRV:** 1.66.
- **What it is:** a backtracking VM for backreferences, lookaround, atomic
  groups, `\G` and `\K`. A pattern without those features is delegated to
  `regex-automata` and stays linear.
- **Bound:** fancy patterns are bounded by `backtrack_limit`, default
  1,000,000. When the limit is exceeded, the call returns
  `Error::BacktrackLimitExceeded`. This is a deterministic step limit, the
  exact analogue of HAPI's timeout error, and it would satisfy
  `path.errors.004`.
- **New dependencies:** `fancy-regex` and `bit-set` 0.8, plus `bit-vec`.
  Both are Apache-2.0 OR MIT. `fancy-regex` depends on `regex-automata`
  directly, not on `regex`.
- **Against:**
  - It is 0.x with fast churn: 0.19.0 on 2026-07-28, then 0.20.0, 0.20.1 and
    0.20.2 (2026-11-11). 0.20.2 is six days old today.
  - Its syntax follows Oniguruma and Perl, not Java. It still carries every
    silent difference listed in (a), and adds its own.
  - It buys syntax that no suite case uses and that no consumer of Frevo is
    known to need.
- **Possessive quantifiers:** the docs do not mention them.

### (c) An in-crate backtracking engine

This is the only route to Java-exact semantics: UTF-16 code-unit iteration,
Java's line terminators, ASCII classes, Java's flag letters and replacement
syntax, with a step counter that turns HAPI's timeout into a deterministic
error.

Against:

- It is several thousand lines of new code on the write path, against a
  23-case suite.
- It must be clean-room (see Constraints).
- Every Java corner case it gets wrong is a silent bug in exactly the
  component ADR 0004 warned about.
- A backtracking engine is exponential without its budget, so the budget
  itself becomes a correctness-critical knob.

This is not justified while the observed syntax is literals, `.*`, anchors
and a range class.

### (d) Others, each worth a line

- **`regex-lite` 0.1.9** (MIT OR Apache-2.0, MSRV 1.65, no dependencies,
  rust-lang): its ASCII-only `\d`, `\w`, `\s`, `\b` and ASCII-only `(?i)` are
  closer to Java's defaults than `regex`'s. But it has no `\p{…}` at all
  (Java does), no class set operations, and is documented as substantially
  slower. Its O(m·n) guarantee is the same as `regex`'s. Rejected: Java's
  `\p{L}` and the like would become compile errors, and `regex` gives the
  ASCII classes by rewriting them to explicit classes (see Decision).
- **`regress` 0.12.0** (MIT OR Apache-2.0, no declared MSRV): ECMAScript 2018
  syntax, further from Java than either Rust option. Flags are passed as a
  separate argument rather than inline, it has no `\p{…}` property escapes,
  and it is classical backtracking. Its docs state no step or time limit.
  Rejected.
- **`pcre2` 0.2.11** (Unlicense OR MIT): FFI to the C PCRE2 library. It has
  match limits and is closest to Java in syntax, but it is a C build
  dependency for the static musl binary and is `unsafe` at the boundary.
  Rejected.

### (e) Match Java semantics exactly, including ReDoS exposure

Reproducing HAPI's 500 ms wall-clock timeout would need a thread or task per
evaluation. Its outcome would depend on load, it would put a cancellation
point inside a write transaction, and it would breach the `path.perf.*`
budgets. Rejected as a mechanism. The observable behaviour it produces,
"an over-expensive pattern is an evaluation error", is approximated only by
options (b) and (c).

## Decision

Adopt **`regex` 1.13** in `frevo-path`, with a thin Java-compatibility layer
inside the crate. Task 41a implements the layer and tests every item. No
backtracking engine is used.

The layer:

1. **`matches`**: compile `(?s)` + the pattern, then `is_match`. This is
   `find()` semantics.
2. **`matchesFull`**: compile `(?s)\A(?:` + the pattern + `)\z`, which gives
   `Matcher.matches()` semantics exactly.
3. **`replaceMatches`**: compile the pattern with no prefix, then replace all
   matches with an **in-crate Java-syntax expander** over `Captures`. The
   expander handles `$n` with Java's greedy-digit rule, `${name}`, and `\`
   escapes, and turns a reference to a missing group into an evaluation
   error. It never uses `regex`'s own `$` template syntax.
4. **Exact rewrites.** Before compiling, rewrite `\d`, `\D`, `\w`, `\W`, `\s`
   and `\S` (outside and inside classes) and `\b` and `\B` to their ASCII
   meanings, so they mean what they mean in Java's default mode.
5. **Rejections.** Reject, as an evaluation error naming the construct, every
   construct whose `regex` meaning would silently differ from Java's:
   - the inline flags `U` and `u`
   - `\v`
   - possessive quantifiers
   - POSIX-named `\p{…}` properties
   - `--` and `~~` inside classes

   Constructs that `regex` already rejects stay rejected.
6. **Literal patterns are compiled once.** When the pattern argument is a
   string literal, compile it when the expression is compiled
   (`path.perf.002`). A computed pattern is compiled per evaluation. Whether
   it needs a cache is decided by measurement, not now.
7. **Limits.** Set `size_limit` and `nest_limit` explicitly through
   `RegexBuilder`, and record the values in the change's design. Exceeding
   either is an evaluation error. `regex` does not panic on any pattern or
   haystack (`path.errors.003`).

### Cargo changes this implies

These changes are not made by this ADR. They are made only after approval, on
the change branch, in task 41a.

Root `Cargo.toml`, `[workspace.dependencies]`, under a new `# Text` heading:

```toml
regex = { version = "1.13", default-features = false, features = ["std", "perf", "unicode"] }
```

This is the default feature set spelt out, so it reads like the other
workspace entries. `unicode` is needed for Java's `\p{L}` and similar
classes. `perf` keeps the literal and DFA optimisations that the 300 µs
extraction budget relies on.

`crates/frevo-path/Cargo.toml`:

```toml
[dependencies]
regex.workspace = true
```

**`deny.toml`: no change.** MIT OR Apache-2.0 is on the allow-list. The
version is not a wildcard. No crate becomes duplicated, because
`regex-automata` 0.4.18 already satisfies `regex`'s `^0.4.16`.

**`Cargo.lock`:** one new package, `regex` 1.13.1. Feature unification turns
on more `regex-automata` features for the crates already in the graph.

`NOTICE` and `docs/provenance.md`: nothing to record. This is a dependency,
not ported code, and `regex` ships no NOTICE file.

## Consequences

Good:

- Bounded by construction: there is no catastrophic backtracking, so there is
  no regex timeout to design and no budget to tune. Where HAPI burns a thread
  and up to 500 ms, Frevo answers in linear time.
- One new crate in the lockfile, from rust-lang. Its dependencies are already
  audited through `tracing-subscriber`. It is 1.x stable, its MSRV of 1.65 is
  well below the workspace's 1.85, and it contains no `unsafe` that Frevo
  owns.
- Every construct Frevo cannot match is a compile error at evaluation, never
  a silently different answer. Silent differences are either rewritten
  exactly or rejected.
- `frevo-validator` can reuse the same layer for Java-semantics patterns.

Bad, and accepted (these become entries when task 41a lands):

**Deviation candidates.** Here HAPI returns an answer and Frevo returns an
error, or the reverse.

- DEV candidate: backreferences, lookaround, atomic groups, `\Q…\E`, `\R`,
  `\X`, `\G`, `\Z`, `\h` and `(?d)` are evaluation errors in Frevo, where HAPI
  evaluates them.
- DEV candidate: a pattern on which HAPI times out (an error after about
  500 ms) gets an answer in Frevo. The reverse never happens.
- DEV candidate: the constructs rejected in Decision item 5 (`(?U)`, `(?u)`,
  `\v`, possessive quantifiers, POSIX `\p{…}`, and `--` and `~~` in classes)
  are errors in Frevo and valid in Java.
- DEV candidate: in `replaceMatches`, which runs without `(?s)`, `.` matches
  `\r`, `\u0085`, ` ` and ` ` in Frevo but not in Java.
- DEV candidate: `$` without `(?m)` does not match before a final line
  terminator, in all three functions. For the two boolean functions this
  could be rewritten exactly as `(?:\r\n|[\n\r\u0085  ])?\z`.
  Whether to do so is left to task 41a's spec review.
- DEV candidate: `(?i)` folds Unicode case where Java's default folds ASCII
  only. For example, `K` (U+212A, Kelvin) matches `k` in Frevo. An exact
  rewrite exists only for ASCII literals.
- DEV candidate, to confirm in the differential: empty matches next to a
  non-empty match. The probe showed `regex` gives
  `Regex::new("b*").replace_all("abc", "-")` = `-a-c-`. Java's `Matcher`,
  which allows an empty match right after a non-empty one, is expected to
  give `-a--c-` for `"abc".replaceAll("b*", "-")`. The suite does not
  exercise this. If the differential confirms the difference, the expander in
  Decision item 3 can drive `find_at` itself and reproduce Java's iteration
  exactly.

**Debt candidates.**

- DEBT candidate: there is no compiled-pattern cache for patterns computed at
  evaluation time. The trigger is a `bench/` workload or a corpus
  search-parameter where per-evaluation compilation shows up.
- DEBT candidate: the O(m·n²) worst case of `replace_all` iteration is not
  separately bounded. The trigger is a profiling or fuzzing finding.
- DEBT candidate: HAPI's `regexTimeoutMillis` setter is not ported, because
  there is nothing to time out. Per section 1, no flag is added. The trigger
  is a consumer that sets it.

## What would change this

- **A consumer needs Java-only syntax.** If a corpus `SearchParameter`, an IG
  invariant, or a differential diff needs a backreference or lookaround, add
  `fancy-regex` behind the same compatibility layer. It sits on the same
  `regex-automata`, so non-fancy patterns keep their linear-time path. Map
  `BacktrackLimitExceeded` to HAPI's "Timeout evaluating regex" error, and
  write that as an ADR amendment with the owner's approval.
- **Silent-difference diffs accumulate.** If the differential keeps finding
  silent differences that cannot be rewritten exactly, reconsider option (c),
  a clean-room in-crate engine with a step budget.
- **`frevo-validator` finds a different trade-off.** If profile patterns turn
  out to need something different, decide it there, and amend this ADR rather
  than run two regex engines in one binary.
