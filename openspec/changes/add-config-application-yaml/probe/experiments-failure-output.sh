#!/usr/bin/env bash
# Failure output with logback (the starter's logback.xml), stdout and stderr separated.
S="${PROBE_DIR:?set PROBE_DIR to the probe working directory}"
C="$S/c3"; rm -rf "$C"; mkdir -p "$C"
put() { mkdir -p "$(dirname "$C/$1")"; printf '%b' "$2" > "$C/$1"; }
run() {
  local name="$1"; shift
  local envs=(); while [ $# -gt 0 ] && [ "$1" != "--" ]; do envs+=("$1"); shift; done; [ "$1" = "--" ] && shift
  mkdir -p "$C/$name"
  ( cd "$C/$name" && env "${envs[@]}" LOGBACK=1 PROBE_FILTER='GATES|ENV\[hapi.fhir.app_content|getDefault_page_size' bash "$S/run2.sh" "$C/$name" "$@" > "$C/$name.out" 2> "$C/$name.err" ); local rc=$?
  echo "=== $name exit=$rc stdout=$(wc -l < "$C/$name.out") stderr=$(wc -l < "$C/$name.err")"
  echo "--- stdout (first 22 non-'at' lines)"; grep -v "^\s*at \|^\s*\.\.\. " "$C/$name.out" | head -22 | cut -c1-260
  echo "--- stderr (first 6)"; grep -v "^\s*at " "$C/$name.err" | head -6 | cut -c1-200
}
put bind/application.yaml 'hapi:\n  fhir:\n    graphql_enabled: "Y"\n'; run bind --
put npe/x.yaml 'x: 1\n'; run npe -- --spring.config.location=file:./x.yaml
put ph/application.yaml 'hapi.fhir.app_content_path: ${nope}\n'; run ph --
run json 'SPRING_APPLICATION_JSON={bad' --
run prof 'SPRING_PROFILES_ACTIVE=-x' --
run arg -- '--=x'
run loc -- --spring.config.location=file:./nope.yaml
put dup/application.yaml 'hapi:\n  fhir:\n    default_page_size: 3\n    default_page_size: 4\n'; run dup --
run ok --
