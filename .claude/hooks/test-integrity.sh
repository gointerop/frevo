#!/usr/bin/env bash
# H4 -- PreToolUse on Edit|Write|MultiEdit
#
# Tests and oracles are the only thing standing between this project and a
# server that looks finished. The cheapest way to make a gate green is to
# weaken the test, so that route is closed:
#
#   - no #[ignore]
#   - no #[should_panic] without an expected = "..."
#   - no todo!() / unimplemented!() in a test path
#   - no deleting a #[test]
#   - every test carries a // covers: <scenario-id>
#   - every covers: id exists in the active change's spec delta or in
#     openspec/specs/**

. "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

INPUT="$(cat)"
require_repo_root

tool="$(json_get "$INPUT" tool_name)"
content="$(json_get "$INPUT" tool_input.content)"
new_string="$(json_get "$INPUT" tool_input.new_string)"
old_string="$(json_get "$INPUT" tool_input.old_string)"

# What is being added to the file by this call.
added="${content}
${new_string}"

is_test_path() {
  case "$1" in
    *"/tests/"*|*_test.rs|*test_*.rs|tests/*) return 0 ;;
  esac
  return 1
}

while IFS= read -r p; do
  [ -z "$p" ] && continue
  case "$p" in *.rs) ;; *) continue ;; esac

  # A #[cfg(test)] module inside a src file counts as a test path too.
  in_test_context=1
  if ! is_test_path "$p"; then
    if printf '%s' "$added" | grep -q 'cfg(test)' ; then
      in_test_context=0
    elif [ -f "$p" ] && grep -q 'cfg(test)' "$p" 2>/dev/null; then
      in_test_context=0
    else
      in_test_context=1
    fi
    [ "$in_test_context" -ne 0 ] && continue
  fi

  # --- 1. #[ignore] ------------------------------------------------------
  if printf '%s' "$added" | grep -qE '#\[\s*ignore'; then
    block <<EOF
BLOCKED: this edit adds #[ignore] to $p.

An ignored test is a test that no longer defends anything, and it makes a red
gate look green. The constitution forbids it with no exception.

If the case genuinely cannot pass:
  1. add an entry to docs/known-gaps.md with the exact case identifiers, the
     root cause, and a decision (accepted / deferred / blocked);
  2. get the owner's approval;
  3. leave the test failing until then.

Recording a gap makes the failure visible. Ignoring it hides it.
EOF
  fi

  # --- 2. #[should_panic] without expected -------------------------------
  if printf '%s' "$added" | grep -qE '#\[\s*should_panic\s*\]'; then
    block <<EOF
BLOCKED: this edit adds a bare #[should_panic] to $p.

A bare should_panic passes on *any* panic, including a panic from a bug
elsewhere, an unwrap on None, or an arithmetic overflow -- so it asserts almost
nothing.

Name the panic you expect:

  #[should_panic(expected = "unsupported search modifier")]
EOF
  fi

  # --- 3. todo!() / unimplemented!() in a test path ----------------------
  if printf '%s' "$added" | grep -qE '\b(todo!|unimplemented!)\s*\('; then
    block <<EOF
BLOCKED: this edit puts todo!() or unimplemented!() into the test path $p.

A test that reaches todo!() panics for a reason unrelated to the behaviour under
test, so the result carries no information.

Write the test so it asserts the specified behaviour and fails because the
behaviour is missing. That is exactly what a pre-implementation test should do.
EOF
  fi

  # --- 4. deleting a #[test] ---------------------------------------------
  if [ -n "$old_string" ]; then
    old_n="$(printf '%s' "$old_string"  | grep -cE '#\[\s*(test|tokio::test)' || true)"
    new_n="$(printf '%s' "$new_string"  | grep -cE '#\[\s*(test|tokio::test)' || true)"
    if [ "${old_n:-0}" -gt "${new_n:-0}" ]; then
      block <<EOF
BLOCKED: this edit removes $(( old_n - new_n )) test(s) from $p.

Deleting a test deletes the record of a behaviour someone decided mattered.

  - if the behaviour changed: update the spec delta first, let spec-reviewer
    approve it, and let oracle-builder rewrite the test;
  - if the test is wrong: say so and stop, rather than removing it;
  - if the case cannot pass: docs/known-gaps.md, with owner approval.
EOF
    fi
  fi

  # Writing a whole file that previously had tests and now has fewer.
  if [ "$tool" = "Write" ] && [ -f "$p" ]; then
    was="$(grep -cE '#\[\s*(test|tokio::test)' "$p" 2>/dev/null || true)"
    now="$(printf '%s' "$content" | grep -cE '#\[\s*(test|tokio::test)' || true)"
    if [ "${was:-0}" -gt "${now:-0}" ]; then
      block <<EOF
BLOCKED: overwriting $p would reduce its tests from $was to $now.

Rewriting a test file wholesale is how tests disappear unnoticed. Use Edit for
targeted changes, and keep every existing test unless its removal has been
specified and approved.
EOF
    fi
  fi

  # --- 5 & 6. covers: tags exist and resolve -----------------------------
  n_tests="$(printf '%s' "$added" | grep -cE '#\[\s*(test|tokio::test)' || true)"
  if [ "${n_tests:-0}" -gt 0 ]; then
    n_covers="$(printf '%s' "$added" | grep -cE '//\s*covers:' || true)"
    if [ "${n_covers:-0}" -eq 0 ]; then
      block <<EOF
BLOCKED: $p adds ${n_tests} test(s) with no '// covers:' tag.

Every test states which specified behaviour it defends, so that coverage can be
checked in both directions: no scenario without a test, no test claiming a
scenario that does not exist.

  // covers: search.chain.014
  #[test]
  fn chained_reference_matches_on_target_type() { ... }

Scenario IDs look like <capability>.<area>.<nnn> and appear in the spec delta as
'#### Scenario: [search.chain.014] ...'.
EOF
    fi

    # Resolve each id against the active change's delta and the settled specs.
    change="$(active_change)"
    search_roots=""
    [ -n "$change" ] && [ -d "openspec/changes/$change" ] && search_roots="openspec/changes/$change"
    [ -d openspec/specs ] && search_roots="$search_roots openspec/specs"

    if [ -n "$search_roots" ]; then
      ids="$(printf '%s' "$added" | grep -oE '//\s*covers:.*' | sed 's|//\s*covers:||' | tr ',' '\n' | tr -d ' \r' | grep -E '^[a-z0-9_]+\.[a-z0-9_]+\.[0-9]+$' || true)"
      missing=""
      for id in $ids; do
        if ! grep -rqF "[$id]" $search_roots 2>/dev/null; then
          missing="$missing $id"
        fi
      done
      if [ -n "$missing" ]; then
        block <<EOF
BLOCKED: $p claims to cover scenario id(s) that do not exist:$missing

Searched: $search_roots

A covers: tag naming a nonexistent scenario is worse than no tag -- it reports
coverage that nobody specified. Either:

  - fix the id to match the spec delta, where it appears as
    '#### Scenario: [<id>] ...'; or
  - add the scenario to the spec delta first (which needs spec-reviewer
    approval), then tag the test.
EOF
      fi
    fi
  fi
done <<< "$(target_paths "$INPUT")"

allow
