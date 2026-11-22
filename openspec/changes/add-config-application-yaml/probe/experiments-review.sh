#!/usr/bin/env bash
# Batch of experiments for the review findings. Output: one block per case.
S="${PROBE_DIR:?set PROBE_DIR to the probe working directory}"
C="$S/c2"; rm -rf "$C"; mkdir -p "$C"
F='GATES|ENV\[(scheduling|hapi.fhir.cr.enabled|hapi.fhir.app_content)|PROFILES|Default_page_size|Max_page_size|getServer_address =|Custom_content_path =|Cr.getEnabled|Package|getImplementationGuides\{x\}.getName|Exception|Description|Property:|Value:|Origin:|Reason:|Invalid|must|invalid|Application run failed|APPLICATION FAILED|does not exist|Could not|Cannot'
run() { # name, then env assignments..., then -- args
  local name="$1"; shift
  local envs=(); while [ $# -gt 0 ] && [ "$1" != "--" ]; do envs+=("$1"); shift; done; [ "$1" = "--" ] && shift
  echo "=== $name"
  ( cd "$C/$name" && env "${envs[@]}" PROBE_FILTER="$F" bash "$S/run2.sh" "$C/$name" "$@" 2>&1 ) | grep -v "^\s*at \|^\s*\.\.\." | grep -E "$F" | grep -v "getTester" | head -${MAXL:-25}
  echo "exit=${PIPESTATUS[0]}"
}
mk() { mkdir -p "$C/$1"; }
put() { mkdir -p "$(dirname "$C/$1")"; printf '%b' "$2" > "$C/$1"; }

# F1 scheduling_disabled
mk f1a; run f1a SCHEDULING_DISABLED=true --
put f1b/application.yaml 'scheduling_disabled: TRUE\n'; run f1b --
put f1c/application.yaml 'scheduling_disabled: yes\n'; run f1c --
put f1d/application.yaml 'scheduling-disabled: true\n'; run f1d --

# F2 JVM system properties in placeholders
put f2/application.yaml 'hapi.fhir.server_address: "${java.io.tmpdir}|${user.dir}|${PID:nopid}|${os.name}|${file.separator}|${loader.path:none}|${java.version}|${user.name}"\n'
run f2 --
mk f2b; run f2b 'JAVA_TOOL_OPTIONS=-Dhapi.fhir.default_page_size=6 -Dhapi.fhir.default_page_size=7' 'JDK_JAVA_OPTIONS=-Dhapi.fhir.max_page_size=8' --
mk f2c; run f2c 'JAVA_TOOL_OPTIONS=-Dhapi.fhir.default_page_size=6' 'JDK_JAVA_OPTIONS=-Dhapi.fhir.default_page_size=9' --
mk f2d; run f2d 'JAVA_TOOL_OPTIONS=-Dhapi.fhir.default_page_size=6' 'SPRING_APPLICATION_JSON={"hapi.fhir.default_page_size":5}' HAPI_FHIR_DEFAULT_PAGE_SIZE=2 --
mk f2e; run f2e 'JAVA_TOOL_OPTIONS=-Dhapi.fhir.default_page_size=abc' --

# F3 spring.application.json as argument / system property
mk f3a; run f3a -- '--spring.application.json={"hapi":{"fhir":{"default_page_size":7}}}'
mk f3b; run f3b "JAVA_TOOL_OPTIONS=-Dspring.application.json='{\"hapi\":{\"fhir\":{\"default_page_size\":8}}}'" --
mk f3c; run f3c 'SPRING_APPLICATION_JSON={"hapi":{"fhir":{"default_page_size":9}}}' -- '--spring.application.json={"hapi":{"fhir":{"default_page_size":7}}}'
mk f3d; run f3d 'SPRING_APPLICATION_JSON={"hapi":{"fhir":{"default_page_size":9}}}' 'JAVA_TOOL_OPTIONS=-Dspring.application.json={"hapi":{"fhir":{"default_page_size":8}}}' --
mk f3e; run f3e -- '--spring.application.json={"hapi":{"fhir":{"default_page_size":"abc"}}}'
mk f3f; run f3f 'JAVA_TOOL_OPTIONS=-Dspring.application.json={"hapi":{"fhir":{"default_page_size":"abc"}}}' --
mk f3g; run f3g -- '--spring.application.json={"hapi":{"fhir":{"default_page_size":7}}}' '--hapi.fhir.default_page_size=4'

# F4 quoting
mk f4a; run f4a "JAVA_TOOL_OPTIONS=-Dhapi.fhir.server_address=\"a b\" -Dhapi.fhir.custom_content_path='c d'" --
mk f4b; run f4b "JDK_JAVA_OPTIONS=-Dhapi.fhir.server_address=\"x y\"" --
mk f4c; run f4c "JAVA_TOOL_OPTIONS=-Dhapi.fhir.server_address=a\"b c\"d" --

# F5 profiles
mk f5a; run f5a 'SPRING_PROFILES_ACTIVE=a b' --
mk f5b; run f5b 'SPRING_PROFILES_ACTIVE=-x' --
mk f5c; run f5c 'SPRING_PROFILES_ACTIVE=a/b' --
mk f5d; run f5d 'SPRING_PROFILES_ACTIVE=x!' --
mk f5e; run f5e 'SPRING_PROFILES_ACTIVE=-x' -- --spring.profiles.validate=false
mk f5f; run f5f 'SPRING_PROFILES_ACTIVE=a.b+c@d_e' --
put f5g/application.yaml 'spring.profiles: dev\nhapi.fhir.default_page_size: 3\n'; run f5g --
put f5h/application-q.yaml 'spring.profiles.include: x\n'; run f5h SPRING_PROFILES_ACTIVE=q --
put f5i/application-q.yaml 'spring.profiles.default: x\n'; run f5i SPRING_PROFILES_ACTIVE=q --
put f5j/application-q.yaml 'spring.profiles.group.g: x\n'; run f5j SPRING_PROFILES_ACTIVE=q --
mk f5k; run f5k 'SPRING_PROFILES_ACTIVE= a , b ' --
mk f5l; run f5l 'SPRING_PROFILES_ACTIVE=a,,b' --

# F6 argument parsing
mk f6a; run f6a -- '--=x'
mk f6b; run f6b -- '--' '--hapi.fhir.default_page_size=3'
mk f6c; run f6c -- '--hapi.fhir.default_page_size=3' '--' '--hapi.fhir.max_page_size=4'
mk f6d; run f6d -- '--hapi.fhir.server_address=a=b'

# F7 locations
put f7a/conf/extra.yaml 'hapi.fhir.default_page_size: 5\n'; put f7a/conf/extra-p.yaml 'hapi.fhir.default_page_size: 6\n'
run f7a SPRING_PROFILES_ACTIVE=p SPRING_CONFIG_IMPORT=conf/extra.yaml --
put f7b/conf/noext 'hapi.fhir.max_page_size: 7\n'; run f7b 'SPRING_CONFIG_IMPORT=conf/noext[.yaml]' --
put f7c/conf/application.yaml 'hapi.fhir.max_page_size: 8\n'; put f7c/conf/application-p.yaml 'hapi.fhir.default_page_size: 9\n'; run f7c SPRING_PROFILES_ACTIVE=p SPRING_CONFIG_IMPORT=conf/ --
put f7d/conf/a/application.yaml 'hapi.fhir.max_page_size: 11\n'; put f7d/conf/b/application.yaml 'hapi.fhir.max_page_size: 12\n'; run f7d -- '--spring.config.additional-location=file:./conf/*/'
put f7e/conf/a/x.yaml 'hapi.fhir.max_page_size: 13\n'; put f7e/conf/b/x.yaml 'hapi.fhir.max_page_size: 14\n'; run f7e -- '--spring.config.additional-location=file:./conf/*/x.yaml'
put f7f/conf/noext 'hapi.fhir.max_page_size: 7\n'; run f7f 'SPRING_CONFIG_IMPORT=conf/noext' --
put f7g/conf/x.yaml 'hapi.fhir.max_page_size: 15\n'; run f7g 'SPRING_CONFIG_IMPORT=conf/' --
mk f7h; run f7h 'SPRING_CONFIG_IMPORT=conf/' --
put f7i/conf/a/b/application.yaml 'hapi.fhir.max_page_size: 16\n'; run f7i -- '--spring.config.additional-location=file:./conf/*/*/'

# F9 setter-less derived properties and packageContents
put f9a/application.yaml 'hapi.fhir.cr.cql.data.retrieve_settings.profile_mode: ENFORCED\n'; run f9a --
put f9b/application.yaml 'hapi.fhir.cr.cql.retrieve_settings.profile_mode: OFF\n'; run f9b --
put f9c/application.yaml 'hapi.fhir.cr.cql.terminology.terminology_settings.code_lookup_mode: AUTO\n'; run f9c --
put f9d/application.yaml 'hapi.fhir.implementationguides.x.packageContents: "65,66"\n'; MAXL=30 run f9d --
put f9e/application.yaml 'hapi.fhir.implementationguides.x.package_contents: abc\n'; run f9e --
put f9f/application.yaml 'hapi.fhir.implementationguides.x.packageContents: QUI=\n'; run f9f --

# F10 elasticsearch gate
mk f10a; run f10a 'spring.elasticsearch.uris=http://x' --
mk f10b; run f10b 'spring.elasticsearch.foo=x' --
mk f10c; run f10c 'spring.elasticsearch.connection-timeout=5s' --
mk f10d; run f10d 'spring.elasticsearch.connection-timeout=abc' --
put f10e/application.yaml 'spring:\n  elasticsearch:\n'; run f10e --
put f10f/application.yaml 'spring.elasticsearch.restclient.sniffer.interval: 10s\n'; run f10f --
put f10g/application.yaml 'spring.elasticsearch.uris: []\n'; run f10g --
put f10h/application.yaml 'spring.elasticsearch.socket-keep-alive: maybe\n'; run f10h --

# F11 cr.enabled yes
put f11a/application.yaml 'hapi.fhir.cr.enabled: yes\n'; run f11a --
put f11b/application.yaml 'hapi.fhir.cr.enabled: "yes"\n'; run f11b --
