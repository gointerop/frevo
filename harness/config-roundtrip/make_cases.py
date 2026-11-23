#!/usr/bin/env python3
"""Writes the config-roundtrip case corpus, harness/config-roundtrip/cases/.

    python harness/config-roundtrip/make_cases.py

cases/ is this script's output: edit the script, not the directories, and rerun it.
Each case directory holds:

    case.yaml   covers: <scenario ids>, and optionally random:, process:, deviation:, note:
    files/      copied into the working directory (/app in the image) before the run
    root/       copied to the file-system root (absolute paths such as /app/extra-classes/)
    env         one NAME=value per line, the case's environment (nothing else is inherited
                on the Frevo side; the image's own PATH, LANG, SSL_CERT_FILE and JAVA_VERSION
                are present on the HAPI side, as in HAPI)
    args        one command-line argument per line

case.yaml keys other than covers:
    random:     <bean prefix>:<path> or reads:<read name>[, ...] values that are random by design
                (config.placeholder.010);
                compared by shape, not value
    process:    <bean prefix>:<path> values that depend on the process (PID, user, working directory;
                config.placeholder.012)
    deviation:  DEV-nnnn; HAPI's recorded result is evidence, the scenario states Frevo's behaviour
    note:       free text

The fixtures of the change's probe/ folder are copied verbatim. all-keys.yaml, all-keys.env
and all-keys-kebab.yaml are generated from keys.md and the recorded bean schema (schema.tsv)
so that a key added to either cannot be missed (config.key.005-.007).
"""

from __future__ import annotations

import re
import shutil
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
CHANGE = REPO / "openspec" / "changes" / "add-config-application-yaml"
FIXTURES = CHANGE / "probe" / "fixtures"
CASES = HERE / "cases"

ALL: dict[str, dict] = {}

# Arguments that keep the packaged files out but let HAPI start: with no file at all the
# starter's version conditions throw (config.gate.005), so scenarios that state class
# defaults set the version on the command line, which binds the class default R4.
NO_FILES = ["--spring.config.location=optional:file:./none.yaml"]
R4 = ["--hapi.fhir.fhir_version=R4"]


def case(cid: str, covers, files=None, env=None, args=None, root=None, random=None, process=None,
         deviation=None, note=None):
    if cid in ALL:
        raise SystemExit(f"duplicate case {cid}")
    if isinstance(covers, str):
        covers = [covers]
    ALL[cid] = dict(covers=covers, files=files or {}, env=env or [], args=args or [], root=root or {},
                    random=random or [], process=process or [], deviation=deviation, note=note)


def env(**kw) -> list[tuple[str, str]]:
    return list(kw.items())


def app(text: str) -> dict:
    return {"application.yaml": text}


def fixture(name: str) -> bytes:
    return (FIXTURES / name).read_bytes()


# ==========================================================================================
# config.dump
# ==========================================================================================
case("config.dump.001", ["config.dump.001", "config.dump.002", "config.dump.004"])
case("config.dump.003", "config.dump.003", files=app("hapi: { fhir: { defaultPageSize: 6 } }\n"))
case("config.dump.005", "config.dump.005", files=app("hapi.fhir.subscription.email.password: s3cret\n"))
case("config.dump.005-b", "config.dump.005", files=app("hapi.fhir.subscription.email.password: ''\n"))
case("config.dump.006", "config.dump.006", files=app("hapi.fhir.default_page_size: abc\n"))

# ==========================================================================================
# config.source
# ==========================================================================================
case("config.source.002", ["config.source.002", "config.yaml.015"])
case("config.source.003", "config.source.003", files={
    "application.yaml": "hapi.fhir.default_page_size: 1\nhapi.fhir.max_page_size: 1\n",
    "config/application.yaml": "hapi.fhir.default_page_size: 2\nhapi.fhir.max_page_size: 2\n",
    "config/sub/application.yaml": "hapi.fhir.default_page_size: 3\n"})
case("config.source.004", "config.source.004", files={
    "config/a/application.yaml": "hapi.fhir.max_page_size: 21\n",
    "config/b/application.yaml": "hapi.fhir.max_page_size: 22\n"})
case("config.source.005", "config.source.005", files={
    "application.properties": "hapi.fhir.default_page_size=4\n",
    "application.yml": "hapi.fhir.default_page_size: 6\n",
    "application.yaml": "hapi.fhir.default_page_size: 1\n"})
case("config.source.005-b", "config.source.005", files={
    "application.yml": "hapi.fhir.default_page_size: 6\n",
    "application.yaml": "hapi.fhir.default_page_size: 1\n"})
case("config.source.006", "config.source.006", files={
    "config/sub/readme.txt": "not an application file\n",
    "notes.txt": "not an application file\n"})
case("config.source.007", "config.source.007", args=["--spring.config.location=file:./other/x.yaml"],
     files={"other/x.yaml": "hapi.fhir.default_page_size: 5\n", "application.yaml": "hapi.fhir.max_page_size: 1\n"},
     note="exactly as the scenario states it: no source sets the version")
case("config.source.007-v", "config.source.007", args=["--spring.config.location=file:./other/x.yaml", *R4],
     files={"other/x.yaml": "hapi.fhir.default_page_size: 5\n", "application.yaml": "hapi.fhir.max_page_size: 1\n"})
case("config.source.008", "config.source.008", args=["--spring.config.location=file:./nope.yaml"])
case("config.source.009", "config.source.009", args=["--spring.config.location=optional:file:./nope.yaml"],
     note="exactly as the scenario states it: no source sets the version")
case("config.source.009-v", "config.source.009", args=["--spring.config.location=optional:file:./nope.yaml", *R4])
case("config.source.010", "config.source.010", args=["--spring.config.location=file:./dir/"],
     files={"dir/application.yaml": "hapi.fhir.default_page_size: 33\n"},
     note="exactly as the scenario states it: no source sets the version")
case("config.source.010-v", "config.source.010", args=["--spring.config.location=file:./dir/", *R4],
     files={"dir/application.yaml": "hapi.fhir.default_page_size: 33\n"})
case("config.source.010-p", "config.source.010", args=["--spring.config.location=file:./dir/", *R4],
     env=env(SPRING_PROFILES_ACTIVE="p"),
     files={"dir/application.yaml": "hapi.fhir.default_page_size: 33\n",
            "dir/application-p.yaml": "hapi.fhir.max_page_size: 34\n"})
case("config.source.011", "config.source.011",
     args=["--spring.config.location=file:./dir/", "--spring.config.name=frevo", *R4],
     files={"dir/frevo.yaml": "hapi.fhir.max_page_size: 44\n", "dir/application.yaml": "hapi.fhir.default_page_size: 33\n"})
case("config.source.012", "config.source.012", args=["--spring.config.additional-location=file:./other/x.yaml"],
     files={"other/x.yaml": "hapi.fhir.default_page_size: 5\n",
            "application.yaml": "hapi.fhir.default_page_size: 1\nhapi.fhir.max_page_size: 2\n"})
case("config.source.013", "config.source.013", args=["--spring.config.location=file:./l/1.yaml,file:./l/2.yaml", *R4],
     files={"l/1.yaml": "hapi.fhir.default_page_size: 31\n", "l/2.yaml": "hapi.fhir.default_page_size: 32\n"})
case("config.source.014", "config.source.014", env=env(SPRING_CONFIG_IMPORT="/app/config/application-extra.yaml"),
     files={"application.yaml": "hapi.fhir.default_page_size: 1\n"},
     root={"app/config/application-extra.yaml": "hapi.fhir.default_page_size: 5\n"})
case("config.source.014-b", "config.source.014", env=env(SPRING_CONFIG_IMPORT="conf/x.yaml"),
     files={"application.yaml": "hapi.fhir.default_page_size: 1\n", "conf/x.yaml": "hapi.fhir.default_page_size: 5\n"})
case("config.source.015", "config.source.015", files={
    "config/application.yaml": "spring.config.import: extra/x.yaml\nhapi.fhir.default_page_size: 1\nhapi.fhir.max_page_size: 1\n",
    "config/extra/x.yaml": "hapi.fhir.default_page_size: 2\n"})
case("config.source.016", "config.source.016", files={
    "config/application.yaml": "spring.config.import: optional:nope.yaml,missing.yaml\n"})
case("config.source.017", "config.source.017", args=["--spring.config.location=classpath:/application-cds.yaml"],
     note="exactly as the scenario states it")
case("config.source.017-v", "config.source.017", args=["--spring.config.location=classpath:/application-cds.yaml", *R4])
case("config.source.017-b", "config.source.017", args=["--spring.config.location=classpath:/another.application.yaml", *R4],
     root={"app/extra-classes/another.application.yaml": "hapi.fhir.default_page_size: 9\n"})
case("config.source.017-c", "config.source.017", args=["--spring.config.location=classpath:/another.application.yaml", *R4])
case("config.source.017-d", "config.source.017", args=["--spring.config.location=classpath:/application-cds.yaml", *R4],
     root={"app/extra-classes/application-cds.yaml": "hapi.fhir.cr.enabled: false\nhapi.fhir.default_page_size: 77\n"})
case("config.source.018", "config.source.018", args=["--spring.config.location=file:///configs/another.application.yaml", *R4],
     root={"configs/another.application.yaml": "hapi.fhir.default_page_size: 9\n"})
case("config.source.019", "config.source.019",
     files=app("hapi.fhir.default_page_size: 1\nhapi.fhir.max_page_size: 1\nhapi.fhir.expunge_thread_count: 1\n"
               "hapi.fhir.reindex_thread_count: 1\n"),
     env=env(HAPI_FHIR_DEFAULT_PAGE_SIZE="2", HAPI_FHIR_MAX_PAGE_SIZE="2", HAPI_FHIR_EXPUNGE_THREAD_COUNT="2",
             SPRING_APPLICATION_JSON='{"hapi":{"fhir":{"default_page_size":3,"max_page_size":3}}}'),
     args=["--hapi.fhir.default_page_size=4", "--hapi.fhir.reindex-thread-count=4"])
case("config.source.020", "config.source.020", env=env(SPRING_CONFIG_IMPORT="configtree:./tree/"),
     files={"tree/hapi.fhir.default_page_size": "7"}, deviation="DEV-0031")
case("config.source.020-b", "config.source.020", env=env(SPRING_CONFIG_IMPORT="x.xml"),
     files={"x.xml": '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE properties SYSTEM "http://java.sun.com/dtd/properties.dtd">\n'
                     '<properties>\n  <entry key="hapi.fhir.default_page_size">8</entry>\n</properties>\n'},
     deviation="DEV-0031")
case("config.source.020-c", "config.source.020", env=env(SPRING_CONFIG_IMPORT="configserver:"))
case("config.source.020-d", "config.source.020", env=env(SPRING_CONFIG_IMPORT="optional:configserver:"))
case("config.source.020-e", "config.source.020", env=env(SPRING_CONFIG_IMPORT="x.txt"),
     files={"x.txt": "hapi.fhir.default_page_size: 8\n"})
case("config.source.020-f", "config.source.020", env=env(SPRING_CONFIG_IMPORT="optional:x.txt"),
     files={"x.txt": "hapi.fhir.default_page_size: 8\n"})
case("config.source.021", "config.source.021", files={"application.yaml": ""})
case("config.source.021-b", "config.source.021", files=app("# only a comment\n\n   # and another\n"))
case("config.source.022", "config.source.022", files=app("hapi.fhir.max_page_size: 50\n"),
     env=env(HAPI_FHIR_DEFAULT_PAGE_SIZE="7"))
case("config.source.023", "config.source.023", files=app("hapi.fhir.default_page_size: 1\n"),
     env=env(JAVA_TOOL_OPTIONS="-Xmx3g -Dhapi.fhir.default_page_size=6 -XX:+UseG1GC", HAPI_FHIR_DEFAULT_PAGE_SIZE="2"))
case("config.source.023-b", "config.source.023",
     env=env(JAVA_TOOL_OPTIONS="-Dhapi.fhir.default_page_size=6", JDK_JAVA_OPTIONS="-Dhapi.fhir.default_page_size=9"))
case("config.source.023-c", "config.source.023",
     env=env(JAVA_TOOL_OPTIONS="-Dhapi.fhir.default_page_size=6 -Dhapi.fhir.default_page_size=7"))
case("config.source.023-d", "config.source.023", env=env(JAVA_TOOL_OPTIONS="-Dhapi.fhir.default-page-size=6"))
case("config.source.023-e", "config.source.023", env=env(JAVA_TOOL_OPTIONS="-Dhapi.fhir.default_page_size=abc"))
case("config.source.023-f", "config.source.023",
     env=env(JAVA_TOOL_OPTIONS="-Dhapi.fhir.default_page_size=6", SPRING_APPLICATION_JSON='{"hapi.fhir.default_page_size":5}',
             HAPI_FHIR_DEFAULT_PAGE_SIZE="2"))
case("config.source.024", "config.source.024", env=env(JAVA_TOOL_OPTIONS="-Xmx3g -XX:+UseG1GC -Dhapi.fhir.server_address"))
case("config.source.024-b", "config.source.024",
     env=env(JAVA_TOOL_OPTIONS="-Dhapi.fhir.server_address=\"a b\" -Dhapi.fhir.custom_content_path='c d'"))
case("config.source.024-c", "config.source.024", env=env(JAVA_TOOL_OPTIONS='-Dhapi.fhir.server_address=a"b c"d'))
case("config.source.025", "config.source.025", env=env(JDK_JAVA_OPTIONS='-Dhapi.fhir.server_address="x y"'))
case("config.source.026", "config.source.026", env=env(SPRING_PROFILES_ACTIVE="p", SPRING_CONFIG_IMPORT="conf/extra.yaml"),
     files={"conf/extra.yaml": "hapi.fhir.default_page_size: 5\n", "conf/extra-p.yaml": "hapi.fhir.default_page_size: 6\n"})
case("config.source.026-b", "config.source.026",
     env=env(SPRING_PROFILES_ACTIVE="p", SPRING_CONFIG_IMPORT="/app/config/application-extra.yaml"),
     root={"app/config/application-extra.yaml": "hapi.fhir.default_page_size: 5\n",
           "app/config/application-extra-p.yaml": "hapi.fhir.default_page_size: 6\n"})
case("config.source.026-c", "config.source.026", env=env(SPRING_PROFILES_ACTIVE="p"),
     args=["--spring.config.location=file:./conf/extra.yaml"],
     files={"conf/extra.yaml": "hapi.fhir.fhir_version: R4\nhapi.fhir.default_page_size: 5\n",
            "conf/extra-p.yaml": "hapi.fhir.default_page_size: 6\n"})
case("config.source.026-d", "config.source.026", env=env(SPRING_PROFILES_ACTIVE="p"),
     args=["--spring.config.additional-location=file:./conf/extra.yaml"],
     files={"conf/extra.yaml": "hapi.fhir.default_page_size: 5\n", "conf/extra-p.yaml": "hapi.fhir.default_page_size: 6\n"})
case("config.source.027", "config.source.027", env=env(SPRING_CONFIG_IMPORT="conf/noext[.yaml]"),
     files={"conf/noext": "hapi.fhir.max_page_size: 7\n"})
case("config.source.027-b", "config.source.027", env=env(SPRING_CONFIG_IMPORT="conf/noext"),
     files={"conf/noext": "hapi.fhir.max_page_size: 7\n"})
case("config.source.028", "config.source.028", env=env(SPRING_PROFILES_ACTIVE="p", SPRING_CONFIG_IMPORT="conf/"),
     files={"conf/application.yaml": "hapi.fhir.max_page_size: 8\n", "conf/application-p.yaml": "hapi.fhir.default_page_size: 9\n"})
case("config.source.028-b", "config.source.028", env=env(SPRING_CONFIG_IMPORT="conf/"),
     files={"conf/x.yaml": "hapi.fhir.max_page_size: 15\n"})
case("config.source.028-c", "config.source.028", env=env(SPRING_CONFIG_IMPORT="conf/"))
case("config.source.029", "config.source.029", args=["--spring.config.additional-location=file:./conf/*/"],
     files={"conf/a/application.yaml": "hapi.fhir.max_page_size: 11\n", "conf/b/application.yaml": "hapi.fhir.max_page_size: 12\n"})
case("config.source.029-b", "config.source.029", args=["--spring.config.additional-location=file:./conf/*/x.yaml"],
     files={"conf/a/x.yaml": "hapi.fhir.max_page_size: 13\n", "conf/b/x.yaml": "hapi.fhir.max_page_size: 14\n"})
case("config.source.029-c", "config.source.029", args=["--spring.config.additional-location=file:./conf/*/*/"],
     files={"conf/a/b/application.yaml": "hapi.fhir.max_page_size: 16\n"})
case("config.source.030", "config.source.030", env=env(SPRING_PROFILES_ACTIVE="foo"), root={
    "app/extra-classes/application.properties": "hapi.fhir.default_page_size=41\n",
    "app/extra-classes/config/application.yaml": "hapi.fhir.max_page_size: 42\n",
    "app/extra-classes/application-foo.yaml": "hapi.fhir.server_address: foo\n",
    "app/extra-classes/application.yaml": "hapi.fhir.openapi_enabled: false\n"})
case("config.source.030-b", "config.source.030", env=env(SPRING_PROFILES_ACTIVE="foo"),
     files={"application.yaml": "hapi.fhir.default_page_size: 51\n", "config/application.yaml": "hapi.fhir.max_page_size: 52\n"},
     root={"app/extra-classes/application.properties": "hapi.fhir.default_page_size=41\n",
           "app/extra-classes/config/application.yaml": "hapi.fhir.max_page_size: 42\n"})

# ==========================================================================================
# config.yaml
# ==========================================================================================
case("config.yaml.001", "config.yaml.001", files=app("hapi.fhir.default_page_size: 3\n"))
case("config.yaml.001-b", "config.yaml.001", files=app("hapi: { fhir: { default_page_size: 3 } }\n"))
case("config.yaml.001-c", "config.yaml.001", files=app("hapi.fhir: { default_page_size: 3 }\n"))
case("config.yaml.002", "config.yaml.002",
     files=app("hapi.fhir.default_page_size: 1\n---\nhapi.fhir.default_page_size: 2\n"))
case("config.yaml.003", "config.yaml.003",
     files=app("hapi:\n  fhir:\n    default_page_size: 3\n    default_page_size: 4\n"))
BOOLS = "true True TRUE false False FALSE yes Yes YES no No NO on On ON off Off OFF".split()
case("config.yaml.004", "config.yaml.004",
     files=app("hapi.fhir.supported_resource_types:\n" + "".join(f"  - {b}\n" for b in BOOLS)))
case("config.yaml.004-b", "config.yaml.004", files=app("hapi.fhir.supported_resource_types: [y, Y, n, tRUE]\n"))
INTS = ["0x1F", "010", "0777", "0b101", "1_000", "190:20:30", "09", "0o17"]
case("config.yaml.005", "config.yaml.005",
     files=app("hapi.fhir.supported_resource_types:\n" + "".join(f"  - {v}\n" for v in INTS)))
FLOATS = ["1.10", "1.0", ".5", "+.5", "1_0.5", "1e3", "1.0e+3", "-.inf", ".NaN", "1e7", "0.0001"]
case("config.yaml.006", "config.yaml.006",
     files=app("hapi.fhir.supported_resource_types:\n" + "".join(f"  - {v}\n" for v in FLOATS)))
case("config.yaml.007", "config.yaml.007", files=app(
    "hapi.fhir.server_address: 2026-01-02\n"
    "hapi.fhir.custom_content_path: 2026-01-02T10:00:00Z\n"
    "hapi.fhir.mdm_rules_json_location: \"010\"\n"
    "hapi.fhir.app_content_path: 'yes'\n"
    "hapi.fhir.logger.name: !!str 010\n"))
case("config.yaml.008", "config.yaml.008", files=app(
    "hapi.fhir.server_address:\n"
    "hapi.fhir.custom_content_path: ~\n"
    "hapi.fhir.mdm_rules_json_location: null\n"
    "hapi.fhir.binary_storage_filesystem_base_directory: Null\n"
    "hapi.fhir.logger.name: NULL\n"))
case("config.yaml.009", "config.yaml.009",
     files=app("base: &b { default_page_size: 5 }\nhapi: { fhir: { <<: *b, max_page_size: 9 } }\n"))
case("config.yaml.010", "config.yaml.010",
     files=app("hapi.fhir.cors.allowed_origin: []\nhapi.fhir.partitioning: {}\n"))
case("config.yaml.011", "config.yaml.011", files=app("hapi:\n  fhir:\n    supported_resource_types: [Patient, Observation]\n"))
case("config.yaml.012", "config.yaml.012", files=app("hapi.fhir.implementationguides:\n  no:\n    name: x\n"))
case("config.yaml.013", "config.yaml.013", files=app("hapi.fhir.implementationguides:\n  \"[a.b]\":\n    name: e\n"))
case("config.yaml.014", "config.yaml.014", files=app(
    "hapi:\n  fhir:\n    logger:\n      format: >-\n"
    "        Path[${servletPath}] Source[${requestHeader.x-forwarded-for}]\n"
    "        Operation[${operationType} ${operationName} ${idOrResourceName}]\n"
    "        UA[${requestHeader.user-agent}] Params[${requestParameters}]\n"
    "        ResponseEncoding[${responseEncodingNoDefault}]\n"))
case("config.yaml.016", "config.yaml.016", files={"application.yaml": fixture("config.yaml.016.yaml")})

# ==========================================================================================
# config.properties
# ==========================================================================================
case("config.properties.001", "config.properties.001", files={"application.properties":
    "hapi.fhir.server_address = http://a\nhapi.fhir.mdm_rules_json_location: x\nhapi.fhir.custom_content_path abc\n"})
case("config.properties.002", "config.properties.002", files={"application.properties":
    "# hapi.fhir.default_page_size=1\n! hapi.fhir.max_page_size=1\n#hapi.fhir.server_address=x\n!x=y\nhapi.fhir.expunge_thread_count=3\n"})
case("config.properties.003", "config.properties.003", files={"application.properties": "hapi.fhir.server_address = http://a   \n"})
case("config.properties.004", "config.properties.004",
     files={"application.properties": fixture("config.properties.004.properties")})
PROPS_DOCS = "hapi.fhir.default_page_size=5\n#---\nspring.config.activate.on-profile=p\nhapi.fhir.default_page_size=6\n"
case("config.properties.005", "config.properties.005", files={"application.properties": PROPS_DOCS})
case("config.properties.005-b", "config.properties.005", files={"application.properties": PROPS_DOCS},
     env=env(SPRING_PROFILES_ACTIVE="p"))
case("config.properties.006", "config.properties.006", files={"application.properties": "hapi.fhir.graphql_enabled=1\n"})
case("config.properties.007", "config.properties.007", files={"application.properties": "hapi.fhir.app_content_path=\n"})

# ==========================================================================================
# config.profile
# ==========================================================================================
case("config.profile.001", "config.profile.001", env=env(SPRING_PROFILES_ACTIVE="cds"))
case("config.profile.002", ["config.profile.002", "config.gate.020"], env=env(SPRING_PROFILES_ACTIVE="elastic"))
case("config.profile.003", "config.profile.003", env=env(SPRING_PROFILES_ACTIVE="cds"),
     files=app("hapi.fhir.cdshooks.enabled: false\n"))
AB = {"application-a.yaml": "hapi.fhir.default_page_size: 11\n", "application-b.yaml": "hapi.fhir.default_page_size: 12\n"}
case("config.profile.004", "config.profile.004", files=AB, args=["--spring.profiles.active=a,b"])
case("config.profile.004-b", "config.profile.004", files=AB, args=["--spring.profiles.active=b,a"])
P5 = {"application.yaml": "hapi.fhir.default_page_size: 1\n---\nspring.config.activate.on-profile: a\nhapi.fhir.default_page_size: 2\n",
      "application-b.yaml": "hapi.fhir.default_page_size: 10\n"}
case("config.profile.005", "config.profile.005", files=P5, args=["--spring.profiles.active=a,b"])
case("config.profile.005-b", "config.profile.005", files=P5, args=["--spring.profiles.active=b,a"])
P6 = app("hapi.fhir.default_page_size: 1\n"
         "---\nspring.config.activate.on-profile: a\nhapi.fhir.default_page_size: 2\nhapi.fhir.max_page_size: 2\n"
         "---\nspring.config.activate.on-profile: \"!a\"\nhapi.fhir.max_page_size: 3\n"
         "---\nspring.config.activate.on-profile: a & b\nhapi.fhir.server_address: a-and-b\n")
case("config.profile.006", "config.profile.006", files=P6)
case("config.profile.006-b", "config.profile.006", files=P6, args=["--spring.profiles.active=a"])
case("config.profile.006-c", "config.profile.006", files=P6, args=["--spring.profiles.active=a,b"])
P7 = {"application-default.yaml": "hapi.fhir.server_address: default-profile\n"}
case("config.profile.007", "config.profile.007", files=P7)
case("config.profile.007-b", "config.profile.007", files=P7, args=["--spring.profiles.active=x"])
case("config.profile.008", "config.profile.008", args=["--spring.profiles.default=dd"],
     files={"application-dd.yaml": "hapi.fhir.server_address: dflt\n"})
case("config.profile.009", "config.profile.009", env=env(SPRING_PROFILES_ACTIVE="cds"),
     files={"application.yaml": "spring.profiles.include: b\n", "application-b.yaml": "hapi.fhir.default_page_size: 15\n"})
case("config.profile.010", "config.profile.010", files=app("spring.profiles.active: cds\n"))
case("config.profile.011", "config.profile.011", args=["--spring.profiles.active=g"],
     files={"application.yaml": "spring.profiles.group.g: b,a\n", **AB})
case("config.profile.012", "config.profile.012", env=env(SPRING_PROFILES_ACTIVE="q"),
     files={"application-q.yaml": "spring.profiles.active: x\n"})
case("config.profile.013", "config.profile.013", env=env(SPRING_PROFILES_ACTIVE="q"),
     files=app("hapi.fhir.default_page_size: 1\n---\nspring.config.activate.on-profile: q\nspring.profiles.active: r\n"))
case("config.profile.014", "config.profile.014", env=env(KUBERNETES_SERVICE_HOST="10.0.0.1", KUBERNETES_SERVICE_PORT="443"),
     files=app("hapi.fhir.max_page_size: 1\n---\nspring.config.activate.on-cloud-platform: kubernetes\nhapi.fhir.default_page_size: 7\n"),
     deviation="DEV-0031")
for suffix, value in [("", "a b"), ("-b", "-x"), ("-c", "a/b"), ("-d", "x!"), ("-e", "a,,b"), ("-f", "a.b+c@d_e"), ("-g", " a , b ")]:
    case(f"config.profile.015{suffix}", "config.profile.015", env=env(SPRING_PROFILES_ACTIVE=value))
case("config.profile.016", "config.profile.016", env=env(SPRING_PROFILES_ACTIVE="-x"), args=["--spring.profiles.validate=false"])
case("config.profile.017", "config.profile.017", files=app("spring.profiles: dev\nhapi.fhir.default_page_size: 3\n"))
case("config.profile.018", "config.profile.018", env=env(SPRING_PROFILES_ACTIVE="q"),
     files={"application-q.yaml": "spring.profiles.include: x\n"})
case("config.profile.018-b", "config.profile.018", env=env(SPRING_PROFILES_ACTIVE="q"),
     files={"application-q.yaml": "spring.profiles.default: x\n"})
case("config.profile.018-c", "config.profile.018", env=env(SPRING_PROFILES_ACTIVE="q"),
     files={"application-q.yaml": "spring.profiles.group.g: x\n"})

# ==========================================================================================
# config.env
# ==========================================================================================
case("config.env.001", "config.env.001", env=env(HAPI_FHIR_DEFAULT_PAGE_SIZE="11"))
case("config.env.002", "config.env.002", env=env(HAPI_FHIR_DEFAULTPAGESIZE="12"))
case("config.env.003", "config.env.003", env=[("hapi.fhir.default_page_size", "13")])
case("config.env.003-b", "config.env.003", env=[("hapi.fhir.default-page-size", "14")])
case("config.env.003-c", "config.env.003", env=[("hapi.fhir.defaultPageSize", "17")])
case("config.env.004", "config.env.004", env=env(HAPI_FHIR__DEFAULT_PAGE_SIZE="18"))
case("config.env.004-b", "config.env.004", env=[("hapi_fhir_default_page_size", "15")],
     note="Linux: names are case-sensitive; the Windows clause of the scenario has no HAPI counterpart here")
case("config.env.004-c", "config.env.004", env=[("Hapi_Fhir_Default_Page_Size", "16")],
     note="Linux: names are case-sensitive; the Windows clause of the scenario has no HAPI counterpart here")
case("config.env.005", "config.env.005", env=env(HAPI_FHIR_CORS_ALLOW_CREDENTIALS="true"))
case("config.env.006", "config.env.006",
     env=env(HAPI_FHIR_SUPPORTEDRESOURCETYPES_0="Patient", HAPI_FHIR_SUPPORTEDRESOURCETYPES_1="Obs"))
case("config.env.006-b", "config.env.006", env=env(HAPI_FHIR_SUPPORTED_RESOURCE_TYPES_0="Patient"))
case("config.env.006-c", "config.env.006", env=env(HAPI_FHIR_SUPPORTED_RESOURCE_TYPES_0_="Patient"))
case("config.env.007", "config.env.007", env=env(HAPI_FHIR_LOGICAL_URLS_0="x"))
case("config.env.008", "config.env.008", env=env(HAPI_FHIR_SEARCH_PREFETCH_THRESHOLDS="1,2"))
case("config.env.009", "config.env.009",
     env=env(HAPI_FHIR_IMPLEMENTATIONGUIDES_SWISS_NAME="swiss.x", HAPI_FHIR_CR_CQL_NAMESPACES_FOO="http://foo"))
case("config.env.009-b", "config.env.009", env=[("hapi.fhir.implementationguides.someIg.name", "com.org.something")])
case("config.env.010", "config.env.010", env=env(HAPI_FHIR_TESTER_HOME_NAME="EnvTester"))
case("config.env.011", "config.env.011", env=env(HAPI_FHIR_IMPLEMENTATIONGUIDES_IPS_1_0_0_NAME="ips.x"))
case("config.env.012", "config.env.012", env=env(HAPI_FHIR_GRAPHQL_ENABLED="1"))
COMPOSE_ENV = [
    ("SPRING_DATASOURCE_URL", "jdbc:postgresql://hapi-db:5432/hapi"),
    ("SPRING_DATASOURCE_USERNAME", "hapi"),
    ("SPRING_DATASOURCE_PASSWORD", "hapi"),
    ("SPRING_DATASOURCE_DRIVERCLASSNAME", "org.postgresql.Driver"),
    ("SPRING_JPA_PROPERTIES_HIBERNATE_DIALECT", "ca.uhn.fhir.jpa.model.dialect.HapiFhirPostgres94Dialect"),
    ("SPRING_DATASOURCE_HIKARI_MAXIMUMPOOLSIZE", "30"),
    ("HAPI_FHIR_FHIR_VERSION", "R4"),
    ("HAPI_FHIR_SERVER_ADDRESS", "http://localhost:8090/fhir"),
    ("HAPI_FHIR_ALLOW_MULTIPLE_DELETE", "true"),
    ("HAPI_FHIR_ALLOW_CASCADING_DELETES", "true"),
    ("HAPI_FHIR_ALLOW_EXTERNAL_REFERENCES", "true"),
    ("HAPI_FHIR_ALLOW_PLACEHOLDER_REFERENCES", "true"),
    ("HAPI_FHIR_AUTO_CREATE_PLACEHOLDER_REFERENCE_TARGETS", "true"),
    ("HAPI_FHIR_EXPUNGE_ENABLED", "true"),
    ("HAPI_FHIR_DELETE_EXPUNGE_ENABLED", "true"),
    ("HAPI_FHIR_ENFORCE_REFERENTIAL_INTEGRITY_ON_DELETE", "true"),
    ("HAPI_FHIR_ENFORCE_REFERENTIAL_INTEGRITY_ON_WRITE", "true"),
    ("HAPI_FHIR_ETAG_SUPPORT_ENABLED", "true"),
    ("HAPI_FHIR_FILTER_SEARCH_ENABLED", "true"),
    ("HAPI_FHIR_GRAPHQL_ENABLED", "true"),
    ("HAPI_FHIR_BULK_EXPORT_ENABLED", "true"),
    ("HAPI_FHIR_BULK_IMPORT_ENABLED", "true"),
    ("HAPI_FHIR_ENABLE_INDEX_MISSING_FIELDS", "false"),
    ("HAPI_FHIR_NARRATIVE_ENABLED", "false"),
    ("HAPI_FHIR_DAO_SCHEDULING_ENABLED", "false"),
    ("HAPI_FHIR_SUBSCRIPTION_RESTHOOK_ENABLED", "false"),
    ("HAPI_FHIR_CR_ENABLED", "false"),
    ("HAPI_FHIR_MDM_ENABLED", "false"),
    ("JAVA_TOOL_OPTIONS", "-Xmx3g -XX:+UseG1GC"),
]
case("config.env.013", ["config.env.013", "config.unknown.001"], env=COMPOSE_ENV,
     note="harness/docker-compose.yml service hapi, with HAPI_PORT at its default 8090")
case("config.env.014", "config.env.014", env=[("--spring.config.location", "file:///configs/x.yaml")],
     root={"configs/x.yaml": "hapi.fhir.default_page_size: 61\n"})

# ==========================================================================================
# config.args
# ==========================================================================================
case("config.args.001", "config.args.001", args=["--hapi.fhir.default_page_size=4"],
     files=app("hapi.fhir.default_page_size: 1\n"), env=env(HAPI_FHIR_DEFAULT_PAGE_SIZE="2"))
case("config.args.002", "config.args.002", args=["--hapi.fhir.reindex-thread-count=4"])
case("config.args.003", "config.args.003", args=["notanoption", "-x"])
case("config.args.004", "config.args.004", args=["--hapi.fhir.server_address"])
case("config.args.005", "config.args.005",
     args=["--hapi.fhir.supported_resource_types=A", "--hapi.fhir.supported_resource_types=B"])
case("config.args.006", "config.args.006", env=env(SPRING_APPLICATION_JSON='{"hapi":{"fhir":{"server_address":"json"}}}'))
case("config.args.006-b", "config.args.006",
     env=env(SPRING_APPLICATION_JSON='{"hapi":{"fhir":{"default_page_size":7,"graphql_enabled":true,"mdm_rules_json_location":1.10,"server_address":true}}}'))
case("config.args.007", "config.args.007", env=env(SPRING_APPLICATION_JSON="{bad"))
J7 = '--spring.application.json={"hapi":{"fhir":{"default_page_size":7}}}'
case("config.args.008", "config.args.008", args=[J7])
case("config.args.008-b", "config.args.008", args=[J7], env=env(SPRING_APPLICATION_JSON='{"hapi":{"fhir":{"default_page_size":9}}}'))
case("config.args.008-c", "config.args.008", args=[J7, "--hapi.fhir.default_page_size=4"])
case("config.args.008-d", "config.args.008", args=['--spring.application.json={"hapi":{"fhir":{"default_page_size":"abc"}}}'])
JTO8 = "-Dspring.application.json='{\"hapi\":{\"fhir\":{\"default_page_size\":8}}}'"
case("config.args.009", "config.args.009", env=env(JAVA_TOOL_OPTIONS=JTO8))
case("config.args.009-b", "config.args.009",
     env=env(JAVA_TOOL_OPTIONS=JTO8, SPRING_APPLICATION_JSON='{"hapi":{"fhir":{"default_page_size":9}}}'))
case("config.args.009-c", "config.args.009", env=env(JAVA_TOOL_OPTIONS=JTO8), args=[J7])
case("config.args.009-d", "config.args.009",
     env=env(JAVA_TOOL_OPTIONS="-Dspring.application.json='{\"hapi\":{\"fhir\":{\"default_page_size\":\"abc\"}}}'"))
case("config.args.009-e", "config.args.009",
     env=env(JAVA_TOOL_OPTIONS='-Dspring.application.json={"hapi":{"fhir":{"default_page_size":8}}}'))
case("config.args.010", "config.args.010", args=["--=x"])
case("config.args.011", "config.args.011", args=["--", "--hapi.fhir.default_page_size=3"])
case("config.args.011-b", "config.args.011", args=["--hapi.fhir.default_page_size=3", "--", "--hapi.fhir.max_page_size=4"])
case("config.args.012", "config.args.012", args=["--hapi.fhir.server_address=a=b"])

# ==========================================================================================
# config.placeholder
# ==========================================================================================
P1 = app("hapi.fhir.server_address: ${MY_BASE:http://fallback}/fhir\n")
case("config.placeholder.001", "config.placeholder.001", files=P1)
case("config.placeholder.001-b", "config.placeholder.001", files=P1, env=env(MY_BASE="http://env"))
P2 = app("hapi.fhir.mdm_rules_json_location: ${MDMX:${MDMY:nested}}\n")
case("config.placeholder.002", "config.placeholder.002", files=P2)
case("config.placeholder.002-b", "config.placeholder.002", files=P2, env=env(MDMY="y"))
case("config.placeholder.003", "config.placeholder.003", files=app(
    "hapi.fhir.custom_content_path: ${hapi.fhir.mdm-rules-json-location}\n"
    "hapi.fhir.mdm_rules_json_location: ${MDMX:${MDMY:nested}}\n"))
case("config.placeholder.004", "config.placeholder.004", files=app("hapi.fhir.supported_resource_types: [\"${nope3}\"]\n"))
case("config.placeholder.005", "config.placeholder.005", files=app("hapi.fhir.default_page_size: ${NOPE}\n"))
case("config.placeholder.006", "config.placeholder.006", files=app("hapi.fhir.logger.name: \\${escaped}\n"))
case("config.placeholder.007", "config.placeholder.007", files=app("hapi.fhir.server_address: ${hapi.fhir.server_address}\n"))
P8 = app("hapi.fhir.cors.allowed_origin: ${ORIGINS:a,b}\n")
case("config.placeholder.008", "config.placeholder.008", files=P8)
case("config.placeholder.008-b", "config.placeholder.008", files=P8, env=env(ORIGINS="x, y"))
P9 = app("hapi.fhir.logger.error_format: \"ERROR - ${requestVerb} ${requestUrl}\"\n")
case("config.placeholder.009", "config.placeholder.009", files=P9, env=env(requestVerb="GET"))
case("config.placeholder.009-b", "config.placeholder.009", files=P9)
case("config.placeholder.010", "config.placeholder.010",
     files=app("hapi.fhir.server_address: ${random.int(10)}\nhapi.fhir.default_page_size: ${random.int[5,6]}\n"),
     random=["hapi.fhir:server-address"])
case("config.placeholder.010-b", "config.placeholder.010", files=app(
    "hapi.fhir.server_address: ${random.uuid}\nhapi.fhir.custom_content_path: ${random.long}\n"
    "hapi.fhir.mdm_rules_json_location: ${random.int}\nhapi.fhir.app_content_path: ${random.value}\n"),
     random=["hapi.fhir:server-address", "hapi.fhir:custom-content-path", "hapi.fhir:mdm-rules-json-location",
             "hapi.fhir:app-content-path", "reads:hapi.fhir.custom_content_path", "reads:hapi.fhir.app_content_path"])
for suffix, key in [("", "hapi.fhir.app_content_path"), ("-b", "hapi.fhir.custom_content_path"), ("-c", "hapi.fhir.mdm_enabled"),
                    ("-d", "hapi.fhir.binary_storage_mode"), ("-e", "hapi.fhir.enable_repository_validating_interceptor"),
                    ("-f", "hapi.fhir.ig_runtime_upload_enabled"), ("-g", "hapi.fhir.ips_enabled"),
                    ("-h", "spring.ai.mcp.server.enabled")]:
    covers = ["config.placeholder.011", "config.read.007"] if suffix == "" else "config.placeholder.011"
    case(f"config.placeholder.011{suffix}", covers, files=app(f"{key}: ${{nope}}\n"))
# config.read.007, owed by tasks.md task 2 (review.md, fifth review, finding 2): ${nope} in the
# five other reads. fhir_version and cr.enabled are also bound (AppProperties, CrProperties);
# custom-bean-packages and scheduling_disabled have no bound counterpart.
for suffix, key in [("-b", "hapi.fhir.fhir_version"), ("-c", "hapi.fhir.cr.enabled"),
                    ("-d", "hapi.fhir.cdshooks.enabled"), ("-e", "hapi.fhir.custom-bean-packages"),
                    ("-f", "scheduling_disabled")]:
    case(f"config.read.007{suffix}", "config.read.007", files=app(f"{key}: ${{nope}}\n"))
case("config.placeholder.012", "config.placeholder.012", files=app(
    'hapi.fhir.server_address: "${java.io.tmpdir}|${user.dir}|${PID:nopid}|${os.name}|${file.separator}|${loader.path:none}|${user.name}"\n'),
     process=["hapi.fhir:server-address"])
case("config.placeholder.013", "config.placeholder.013", files=app(
    'hapi.fhir.server_address: "${java.version}|${java.vendor}|${java.home}|${java.class.path}"\n'))

# ==========================================================================================
# config.relaxed
# ==========================================================================================
case("config.relaxed.001", "config.relaxed.001", files=app("hapi:\n  fhir:\n    default-page-size: 5\n"))
case("config.relaxed.001-b", "config.relaxed.001", files=app("hapi:\n  fhir:\n    defaultPageSize: 6\n"))
case("config.relaxed.001-c", "config.relaxed.001", files=app("HAPI:\n  FHIR:\n    DEFAULT_PAGE_SIZE: 7\n"))
case("config.relaxed.001-d", "config.relaxed.001", files=app("hapi:\n  fhir:\n    Default__Page--Size: 9\n"))
case("config.relaxed.002", "config.relaxed.002", files=app("hapi.fhir.default.page.size: 8\n"))
case("config.relaxed.003", "config.relaxed.003", files=app(
    "hapi:\n  fhir:\n"
    "    uplifted_refchains_enabled: true\n"
    "    user-request-retry-version-conflicts-interceptor-enabled: true\n"
    "    cdshooks.client-id-header-name: x\n"
    "    cors.allow-credentials: true\n"
    "    cr.terminology_server_client_settings.max_retry_count: 5\n"
    "    implementationguides.a.install_mode: STORE_ONLY\n"
    "    subscription.email.start_tls_enable: true\n"))
case("config.relaxed.004", "config.relaxed.004", files=app("hapi.fhir.cr.cql.compiler.error_level: Error\n"))
case("config.relaxed.004-b", "config.relaxed.004", files=app("hapi.fhir.cr.cql.compiler.error_severity_level: error\n"))
case("config.relaxed.005", "config.relaxed.005", env=[("hapi.fhir.fhir-version", "R5")])
case("config.relaxed.006", "config.relaxed.006", files=app("hapi:\n  fhir:\n    default_page_size: 3\n    default-page-size: 4\n"))
case("config.relaxed.007", "config.relaxed.007", files=app("hapi:\n  fhir:\n    defaultPageSize: 4\n    default_page_size: 3\n"),
     deviation="DEV-0032")
case("config.relaxed.007-b", "config.relaxed.007", files=app("hapi:\n  fhir:\n    max_page_size: 3\n    maxPageSize: 4\n"),
     deviation="DEV-0032")
case("config.relaxed.008", "config.relaxed.008", files=app("hapi.fhir.tester.Home.name: Upper\n"))

# ==========================================================================================
# config.coerce
# ==========================================================================================
for i, v in enumerate(["yes", "on", "TRUE", "Yes"]):
    case(f"config.coerce.001{'-' + 'abcd'[i] if i else ''}", "config.coerce.001", files=app(f"hapi.fhir.graphql_enabled: {v}\n"))
for i, v in enumerate(["yes", "on", "1", "true ", "TRUE", "no", "off", "0", "false"]):
    case(f"config.coerce.002{'-' + 'abcdefghi'[i] if i else ''}", "config.coerce.002",
         files=app(f"hapi.fhir.graphql_enabled: \"{v}\"\n"))
case("config.coerce.003", "config.coerce.003", files=app("hapi.fhir.graphql_enabled: \"Y\"\n"))
case("config.coerce.003-b", "config.coerce.003", files=app("hapi.fhir.graphql_enabled: \"2\"\n"))
case("config.coerce.004", "config.coerce.004", files=app("hapi.fhir.graphql_enabled: 1\n"))
for i, v in enumerate(['""', "~", "''"]):
    case(f"config.coerce.005{'-' + 'abc'[i] if i else ''}", "config.coerce.005", files=app(f"hapi.fhir.etag_support_enabled: {v}\n"))
for i, v in enumerate(['"+5"', '" 5 "', '"5 0"']):
    case(f"config.coerce.006{'-' + 'abc'[i] if i else ''}", "config.coerce.006", files=app(f"hapi.fhir.default_page_size: {v}\n"))
for i, v in enumerate(['"0x10"', '"#10"', '"-0x10"', '"010"']):
    case(f"config.coerce.007{'-' + 'abcd'[i] if i else ''}", "config.coerce.007", files=app(f"hapi.fhir.default_page_size: {v}\n"))
case("config.coerce.008", "config.coerce.008", files=app('hapi.fhir.default_page_size: "1_000"\n'))
case("config.coerce.008-b", "config.coerce.008", files=app('hapi.fhir.default_page_size: "5.0"\n'))
for i, v in enumerate(["1_000", "1:20", "5.7", "1e3", "0x10"]):
    case(f"config.coerce.009{'-' + 'abcde'[i] if i else ''}", "config.coerce.009", files=app(f"hapi.fhir.default_page_size: {v}\n"))
case("config.coerce.010", "config.coerce.010", files=app("hapi.fhir.default_page_size: 99999999999\n"))
case("config.coerce.010-b", "config.coerce.010", files=app("hapi.fhir.default_page_size: .inf\n"))
case("config.coerce.010-c", "config.coerce.010", files=app("hapi.fhir.retain_cached_searches_mins: 99999999999\n"))
case("config.coerce.011", "config.coerce.011", files=app('hapi.fhir.default_page_size: ""\nhapi.fhir.max_binary_size: ""\n'))
case("config.coerce.011-b", "config.coerce.011", files=app("hapi.fhir.default_page_size: ~\nhapi.fhir.max_binary_size: ~\n"))
case("config.coerce.012", "config.coerce.012",
     files=app("hapi.fhir.subscription.email.port:\nhapi.fhir.subscription.email.username:\n"))
for i, v in enumerate(["off", "yes", "1.10", "1.0", "0.8.0", "010", "0x1F", "1:20", ".5", "1e3"]):
    case(f"config.coerce.013{'-' + 'abcdefghij'[i] if i else ''}", "config.coerce.013",
         files=app(f"hapi.fhir.mdm_rules_json_location: {v}\n"))
for i, v in enumerate(['""', "~", "null"]):
    case(f"config.coerce.014{'-' + 'abc'[i] if i else ''}", "config.coerce.014", files=app(f"hapi.fhir.server_address: {v}\n"))
case("config.coerce.015", "config.coerce.015", files=app('hapi.fhir.server_address: " http://x "\n'))
case("config.coerce.016", "config.coerce.016", files=app("hapi.fhir.fhir_version: R4B\n"))
case("config.coerce.016-b", "config.coerce.016", files=app("hapi.fhir.fhir_version: DSTU2_HL7ORG\n"))
for i, (k, v) in enumerate([("fhir_version", "r4"), ("fhir_version", "r-4"), ("fhir_version", "Dstu3"), ("fhir_version", '" R5 "'),
                            ("implementationguides.x.installMode", "store-and-install"),
                            ("cr.cql.data.profile_mode", "en-forced"), ("cr.cql.data.profile_mode", "Declared")]):
    case(f"config.coerce.017{'-' + 'abcdefg'[i] if i else ''}", "config.coerce.017", files=app(f"hapi.fhir.{k}: {v}\n"))
case("config.coerce.018", "config.coerce.018", files=app('hapi.fhir.fhir_version: "4.0.1"\n'))
case("config.coerce.018-b", "config.coerce.018", files=app("hapi.fhir.fhir_version: STU3\n"))
for i, v in enumerate(["OFF", "off", "false"]):
    case(f"config.coerce.019{'-' + 'abc'[i] if i else ''}", "config.coerce.019", files=app(f"hapi.fhir.cr.cql.data.profile_mode: {v}\n"))
case("config.coerce.020", ["config.coerce.020", "config.gate.003"], files=app('hapi.fhir.fhir_version: ""\n'))
case("config.coerce.021", "config.coerce.021", files=app('hapi.fhir.supported_resource_types: " Patient , Observation,,Encounter "\n'))
case("config.coerce.022", "config.coerce.022", files=app(
    'hapi.fhir.search_prefetch_thresholds: "13, 503 ,-1"\nhapi.fhir.allowed_bundle_types: "collection, transaction-response"\n'))
case("config.coerce.023", "config.coerce.023", files=app("hapi.fhir.supported_resource_types: [Patient, 1.10, yes, ~]\n"))
case("config.coerce.024", "config.coerce.024",
     files=app("hapi.fhir.implementationguides.x.additional_resource_folders: [ex1, ex1]\n"))
case("config.coerce.025", "config.coerce.025",
     files=app("hapi.fhir.cors.allowed_origin: []\nhapi.fhir.supported_resource_types:\n"))
case("config.coerce.025-b", "config.coerce.025", files=app("hapi.fhir.cors.allowed_origin:\n"))
case("config.coerce.025-c", "config.coerce.025", files=app(
    "hapi.fhir.cors.allowed_headers:\nhapi.fhir.cors.exposed_headers: []\nhapi.fhir.cors.allowed_methods: ''\n"))
case("config.coerce.026", "config.coerce.026", files=app("hapi.fhir.fhir_version: 6\nhapi.fhir.binary_storage_mode: 1\n"))
case("config.coerce.026-b", "config.coerce.026", files=app('hapi.fhir.client_id_strategy: "1"\n'))
case("config.coerce.027", "config.coerce.027", files=app("hapi.fhir.binary_storage_mode: 7\n"))

# ==========================================================================================
# config.collection, config.object
# ==========================================================================================
case("config.collection.001", "config.collection.001", files=app("hapi.fhir.logical_urls[0]: http://x/*\n"))
case("config.collection.002", "config.collection.002", files=app("hapi.fhir.supported_resource_types[1]: Patient\n"))
case("config.collection.003", "config.collection.003", files=app("hapi.fhir.tester.other.name: Other\n"))
case("config.collection.004", "config.collection.004", files=app(
    "hapi.fhir.implementationguides:\n  ips_1_0_0:\n    name: ips\n  Swiss:\n    name: swiss\n"
    "hapi.fhir.cr.cql.namespaces:\n  Mixed_Key: http://m\n"))
case("config.collection.005", "config.collection.005", files=app(
    "hapi.fhir.cr.cql.namespaces:\n  hl7.fhir.uv.cql: http://hl7.org/fhir/uv/cql\n  \"[hl7.fhir.us.cql]\": http://hl7.org/fhir/us/cql\n"))
case("config.collection.006", "config.collection.006", files=app(
    "hapi.fhir.custom_interceptor_classes: a.B\nhapi.fhir.custom_provider_classes: c.D\nhapi.fhir.local_base_urls: http://l\n"
    "hapi.fhir.auto_version_reference_at_paths: \"Device.patient, Observation.subject\"\n"))
case("config.object.001", ["config.object.001", "config.gate.014"], files=app("hapi.fhir.partitioning: {}\nhapi.fhir.subscription.email: {}\n"))
case("config.object.001-b", ["config.object.001", "config.gate.014"], files=app("hapi.fhir.partitioning:\n  default_partition_id:\n"))
case("config.object.002", "config.object.002", files=app("hapi.fhir.subscription.email.username:\n"))
case("config.object.003", "config.object.003", files=app("hapi.fhir.partitioning:\n"))
case("config.object.004", "config.object.004", files=app("hapi.fhir.cors: true\n"))
case("config.object.005", "config.object.005", args=[*NO_FILES, *R4])

# ==========================================================================================
# config.unknown
# ==========================================================================================
case("config.unknown.001", "config.unknown.001",
     files=app("hapi.fhir.no_such_key: 1\nhapi.fhir.cors.no_such: x\nhapi.fhir.default_page_size: 3\n"))
case("config.unknown.002", "config.unknown.002", files=app(
    "hapi.fhir.search-coord-core-pool-size: abc\nhapi.fhir.search-coord-max-pool-size: abc\nhapi.fhir.search-coord-queue-capacity: abc\n"))
case("config.unknown.003", "config.unknown.003", files=app(
    "hapi.fhir.database_partition_mode_enabled: true\nhapi.fhir.patient_id_partitioning_mode: true\n"
    "hapi.fhir.request_tenant_partitioning_mode: false\nhapi.fhir.additionalResourceFolders: [example]\n"))
case("config.unknown.003-b", "config.unknown.003", files=app(
    "hapi.fhir.partitioning.database_partition_mode_enabled: true\nhapi.fhir.partitioning.patient_id_partitioning_mode: true\n"
    "hapi.fhir.partitioning.request_tenant_partitioning_mode: false\n"))
case("config.unknown.004", "config.unknown.004", files=app(
    "spring.ai.mcp.server.enabled: false\nserver.port: not-a-number\nmanagement.endpoints.web.exposure.include: abc\n"
    "spring.main.banner-mode: off\n"))
case("config.unknown.005", "config.unknown.005", files=app("hapi.fhir.cr.cql.runtime.enable_validation: true\n"))
case("config.unknown.005-b", "config.unknown.005", files=app("hapi.fhir.cr.cql.runtime.enable_validation: false\n"))
case("config.unknown.006", ["config.unknown.006", "config.gate.019"],
     files=app("hapi.fhir.remote_terminology_services_map.a.url: http://x\n"))
case("config.unknown.006-b", ["config.unknown.006", "config.gate.015"], files=app("hapi.fhir.remote_terminology_service.a.url: http://x\n"))
case("config.unknown.007", "config.unknown.007", files=app("hapi.fhir.bulk_export_enabled: true\n"))
case("config.unknown.008", "config.unknown.008", files=app("hapi.fhir.cr.cql.data.retrieve_settings.profile_mode: ENFORCED\n"))
case("config.unknown.008-b", "config.unknown.008", files=app("hapi.fhir.cr.cql.retrieve_settings.profile_mode: OFF\n"))
case("config.unknown.008-c", "config.unknown.008",
     files=app("hapi.fhir.cr.cql.terminology.terminology_settings.code_lookup_mode: AUTO\n"))

# ==========================================================================================
# config.error
# ==========================================================================================
case("config.error.002", ["config.error.002", "config.error.005", "config.error.008", "config.error.001"],
     files={"config/application.yaml": 'hapi:\n  fhir:\n    graphql_enabled: "Y"\n'})
for suffix, key in [("", "graphqlEnabled"), ("-b", "GRAPHQL_ENABLED"), ("-c", "graphql-enabled")]:
    case(f"config.error.003{suffix}", "config.error.003", files=app(f'hapi:\n  fhir:\n    {key}: "Y"\n'))
case("config.error.004", "config.error.004", env=env(HAPI_FHIR_DEFAULT_PAGE_SIZE="abc"))
case("config.error.004-b", "config.error.004", args=["--hapi.fhir.default_page_size=abc"])
case("config.error.004-c", "config.error.004", env=env(SPRING_APPLICATION_JSON='{"hapi":{"fhir":{"default_page_size":"abc"}}}'))
case("config.error.006", "config.error.006", files=app('hapi.fhir.default_page_size: abc\nhapi.fhir.graphql_enabled: "Y"\n'))
case("config.error.007", ["config.error.007", "config.error.008"], args=["--=x"])
case("config.error.009", ["config.error.009", "config.error.008"], env=env(SPRING_PROFILES_ACTIVE="cds"),
     files=app("hapi.fhir.default_page_size: abc\n"))
case("config.error.010", ["config.error.010", "config.key.012"],
     files=app("hapi.fhir.implementationguides.x.package_contents: abc\n"))

# config.error.008, owed by tasks.md task 2 (review.md, fifth review): a binding failure in
# each of the other properties beans, which the record did not have.
# CrProperties (prefix hapi.fhir.cr): a gate key, and a key no gate reads.
case("config.error.008-cr", "config.error.008", files=app("hapi.fhir.cr.enabled: maybe\n"))
case("config.error.008-cr-b", "config.error.008",
     files=app("hapi.fhir.cr.terminology_server_client_settings.max_retry_count: abc\n"))
# CareGapsProperties (prefix hapi.fhir.cr.caregaps) has String members only: a circular
# placeholder (config.placeholder.007's failure) and a scalar for the bean itself.
case("config.error.008-caregaps", "config.error.008",
     files=app("hapi.fhir.cr.caregaps.reporter: ${hapi.fhir.cr.caregaps.reporter}\n"))
case("config.error.008-caregaps-b", "config.error.008", files=app("hapi.fhir.cr.caregaps: abc\n"))
# CqlCompilerProperties (prefix hapi.fhir.cr.cql.compiler): a boolean and an enum.
case("config.error.008-compiler", "config.error.008", files=app("hapi.fhir.cr.cql.compiler.verify_only: maybe\n"))
case("config.error.008-compiler-b", "config.error.008",
     files=app("hapi.fhir.cr.cql.compiler.error_severity_level: Bogus\n"))
# CdsHooksProperties (prefix hapi.fhir.cdshooks): the gate key, and the String member.
case("config.error.008-cdshooks", "config.error.008", files=app("hapi.fhir.cdshooks.enabled: maybe\n"))
case("config.error.008-cdshooks-b", "config.error.008",
     files=app("hapi.fhir.cdshooks.client_id_header_name: ${hapi.fhir.cdshooks.client_id_header_name}\n"))
# Which failure surfaces first when one configuration has several (config.error.008's table).
BIND = "hapi.fhir.default_page_size: abc\n"          # AppProperties binding
ES = "spring.elasticsearch.connection-timeout: abc\n"  # ElasticsearchProperties binding
STRICT = "hapi.fhir.app_content_path: ${nope}\n"     # strict placeholder in a gate read
CQL = "hapi.fhir.cr.cql.runtime.enable_validation: true\n"  # the CqlProperties bean (config.unknown.005)
CDS = "hapi.fhir.cdshooks.enabled: maybe\n"          # CdsHooksProperties binding
NOVERSION = ["--spring.config.location=file:./x.yaml"]  # no hapi.fhir.fhir_version (config.gate.005)
case("config.error.008-order-noversion-bind", "config.error.008", args=NOVERSION, files={"x.yaml": BIND})
case("config.error.008-order-noversion-es", "config.error.008", args=NOVERSION, files={"x.yaml": ES})
case("config.error.008-order-noversion-strict", "config.error.008", args=NOVERSION, files={"x.yaml": STRICT})
case("config.error.008-order-noversion-cql", "config.error.008", args=NOVERSION, files={"x.yaml": CQL})
case("config.error.008-order-bind-es", "config.error.008", files=app(BIND + ES))
case("config.error.008-order-es-bind", "config.error.008", files=app(ES + BIND))
case("config.error.008-order-bind-strict", "config.error.008", files=app(BIND + STRICT))
case("config.error.008-order-es-strict", "config.error.008", files=app(ES + STRICT))
case("config.error.008-order-bind-cql", "config.error.008", files=app(BIND + CQL))
case("config.error.008-order-es-cql", "config.error.008", files=app(ES + CQL))
case("config.error.008-order-strict-cql", "config.error.008", files=app(STRICT + CQL))
case("config.error.008-order-bind-cds", "config.error.008", files=app(BIND + CDS))
case("config.error.008-order-es-cds", "config.error.008", files=app(ES + CDS))

# ==========================================================================================
# config.read
# ==========================================================================================
case("config.read.001", "config.read.001")
case("config.read.002", "config.read.002", files=app("hapi:\n  fhir:\n    mdm-enabled: true\n    IPS_ENABLED: \"true\"\n"))
case("config.read.003", "config.read.003", files=app("hapi.fhir.cr.Enabled: true\nhapi.fhir.CdsHooks.enabled: true\n"))
case("config.read.003-b", "config.read.003", files=app("hapi:\n  fhir:\n    customBeanPackages: a.b\n"))
case("config.read.004", "config.read.004", env=env(HAPI_FHIR_MDM_ENABLED="true"))
case("config.read.004-b", "config.read.004", env=env(HAPI_FHIR_IPSENABLED="true"))
case("config.read.004-c", "config.read.004", env=env(HAPI_FHIR_IPS_ENABLED="true"))
case("config.read.004-d", "config.read.004", env=[("hapi_fhir_ips_enabled", "true")])
case("config.read.005", "config.read.005", env=[("hapi.fhir.fhir-version", "R5")])
case("config.read.006", "config.read.006", files=app("hapi.fhir.mdm_enabled: yes\n"))
case("config.read.006-b", "config.read.006", files=app("hapi.fhir.mdm_enabled: ~\n"))
case("config.read.008", ["config.read.008", "config.gate.021"], env=env(SCHEDULING_DISABLED="true"))
case("config.read.008-b", ["config.read.008", "config.gate.021"], files=app("scheduling_disabled: TRUE\n"))
case("config.read.008-c", ["config.read.008", "config.gate.021"], files=app("scheduling_disabled: yes\n"))
case("config.read.008-d", ["config.read.008", "config.gate.021"], files=app('scheduling_disabled: "TRUE"\n'))
case("config.read.008-e", ["config.read.008", "config.gate.021"], files=app("scheduling-disabled: true\n"))

# ==========================================================================================
# config.gate
# ==========================================================================================
case("config.gate.001", "config.gate.001")
for i, v in enumerate(["R4", "r4", "R4B", "R5", "DSTU2", "DSTU3", "Dstu3", "4.0", "4.0.1"]):
    case(f"config.gate.002{'-' + 'abcdefghi'[i] if i else ''}", "config.gate.002", files=app(f'hapi.fhir.fhir_version: "{v}"\n'))
case("config.gate.004", "config.gate.004", files=app("hapi.fhir.fhir_version: r-4\n"))
case("config.gate.004-b", "config.gate.004", files=app("hapi.fhir.fhir_version: DSTU2_HL7ORG\n"))
case("config.gate.005", ["config.gate.005", "config.error.008"], args=["--spring.config.location=file:./x.yaml"],
     files={"x.yaml": "x: 1\n"})
for i, v in enumerate(['"true"', '"TRUE"', '"yes"', '"1"']):
    case(f"config.gate.006{'-' + 'abcd'[i] if i else ''}", "config.gate.006", files=app(f"hapi.fhir.cr.enabled: {v}\n"))
case("config.gate.006-e", "config.gate.006", args=[*NO_FILES, *R4])
case("config.gate.006-f", "config.gate.006", env=env(HAPI_FHIR_CR_ENABLED="yes"))
case("config.gate.006-g", "config.gate.006", files={"application.properties": "hapi.fhir.cr.enabled=yes\n"})
case("config.gate.006-h", "config.gate.006", files=app('hapi.fhir.cdshooks.enabled: "TRUE"\n'))
case("config.gate.006-i", "config.gate.006", files=app('hapi.fhir.cdshooks.enabled: "yes"\n'))
for i, v in enumerate(['"true"', '"no"', '"0"', '""', '"FALSE"']):
    case(f"config.gate.007{'-' + 'abcde'[i] if i else ''}", "config.gate.007", files=app(f"hapi.fhir.mdm_enabled: {v}\n"))
case("config.gate.007-f", "config.gate.007", args=[*NO_FILES, *R4])
G8 = "hapi.fhir.ips_enabled: \"{v}\"\nhapi.fhir.ig_runtime_upload_enabled: \"{v}\"\nhapi.fhir.enable_repository_validating_interceptor: \"{v}\"\n"
case("config.gate.008", "config.gate.008", files=app(G8.format(v="TRUE")))
case("config.gate.008-b", "config.gate.008", files=app(G8.format(v="yes")))
case("config.gate.009", "config.gate.009")
for i, v in enumerate(["DATABASE", "filesystem", "file-system"]):
    case(f"config.gate.009-{'bcd'[i]}", "config.gate.009", files=app(f'hapi.fhir.binary_storage_mode: "{v}"\n'))
for i, v in enumerate(['"./custom"', '""', '"false"']):
    case(f"config.gate.010{'-' + 'abc'[i] if i else ''}", "config.gate.010",
         files=app(f"hapi.fhir.custom_content_path: {v}\nhapi.fhir.app_content_path: {v}\n"))
case("config.gate.010-d", "config.gate.010")
case("config.gate.011", "config.gate.011", files=app("spring.ai.mcp.server.enabled: false\n"))
case("config.gate.011-b", "config.gate.011", files=app('hapi.fhir.cdshooks.enabled: "TRUE"\n'))
case("config.gate.012", "config.gate.012", args=[*NO_FILES, *R4, "--hapi.fhir.CORS.allow_credentials=true"])
case("config.gate.012-b", "config.gate.012", args=["--spring.config.location=file:./x.yaml"],
     files={"x.yaml": "hapi.fhir.fhir_version: R4\n"})
case("config.gate.013", "config.gate.013", files=app("hapi.fhir.implementationguides: {}\n"))
case("config.gate.013-b", "config.gate.013", files=app("hapi.fhir.implementationguides.x.name: x\n"))
case("config.gate.014", "config.gate.014", files=app("hapi.fhir.partitioning.allow_references_across_partitions: true\n"))
case("config.gate.015", "config.gate.015", files=app("hapi.fhir.remote_terminology_service: {}\n"))
case("config.gate.016", "config.gate.016", args=["--spring.config.location=file:./x.yaml"],
     files={"x.yaml": "hapi.fhir.fhir_version: R4\nhapi.fhir.testerx: 1\n"})
case("config.gate.016-b", "config.gate.016", args=["--spring.config.location=file:./x.yaml"],
     files={"x.yaml": "hapi.fhir.fhir_version: R4\n"}, env=env(HAPI_FHIR_TESTER_HOME_NAME="x"))
case("config.gate.017", "config.gate.017", files=app("hapi.fhir.cdshooks.enabled: true\n"))
case("config.gate.018", "config.gate.018", env=env(SPRING_PROFILES_ACTIVE="p"),
     files={"application-p.yaml": "hapi.fhir.mdm_enabled: true\n"})
case("config.gate.018-b", "config.gate.018", env=env(SPRING_CONFIG_IMPORT="x.yaml"), files={"x.yaml": "hapi.fhir.mdm_enabled: true\n"})
case("config.gate.018-c", "config.gate.018", env=env(HAPI_FHIR_MDM_ENABLED="true"))
case("config.gate.018-d", "config.gate.018", args=["--hapi.fhir.mdm_enabled=true"])
case("config.gate.020", "config.gate.020", env=[("spring.elasticsearch.uris", "http://x")])
case("config.gate.020-b", "config.gate.020", env=[("spring.elasticsearch.connection-timeout", "5s")])
case("config.gate.020-c", "config.gate.020", files=app("spring.elasticsearch.restclient.sniffer.interval: 10s\n"))
case("config.gate.020-d", "config.gate.020", files=app("spring.elasticsearch.uris: []\n"))
case("config.gate.020-e", "config.gate.020", env=[("spring.elasticsearch.foo", "x")])
case("config.gate.021", "config.gate.021", env=env(SCHEDULING_DISABLED="TRUE"))
case("config.gate.022", "config.gate.022", env=env(JAVA_TOOL_OPTIONS="-Dhapi.fhir.testerx=1"),
     args=["--spring.config.location=file:./x.yaml"], files={"x.yaml": "hapi.fhir.fhir_version: R4\n"})
case("config.gate.023", "config.gate.023", files=app("hapi.fhir.cr.enabled: yes\n"))
DUR = "spring.elasticsearch.connection-timeout: {}\nspring.elasticsearch.socket-timeout: {}\n" \
      "spring.elasticsearch.restclient.sniffer.interval: {}\nspring.elasticsearch.restclient.sniffer.delay-after-failure: {}\n"
case("config.gate.024", "config.gate.024", files=app(DUR.format("PT5S", "pt5s", "10", "-5s")))
case("config.gate.024-b", "config.gate.024", files=app(DUR.format("5S", "3d", "500ms", "7us")))
case("config.gate.024-c", "config.gate.024", files=app(DUR.format("9ns", "1m", "2h", "5m")))
case("config.gate.024-d", "config.gate.024", files=app(
    "spring.elasticsearch.uris: http://a, http://b\nspring.elasticsearch.username: u\nspring.elasticsearch.password: p\n"
    "spring.elasticsearch.socket-keep-alive: true\nspring.elasticsearch.path-prefix: /es\n"
    "spring.elasticsearch.restclient.ssl.bundle: b\n"))
for i, v in enumerate(['"5 s"', "1.5s", "abc"]):
    case(f"config.gate.024-{'efg'[i]}", "config.gate.024", files=app(f"spring.elasticsearch.connection-timeout: {v}\n"))
case("config.gate.024-h", "config.gate.024", files=app("spring.elasticsearch.socket-keep-alive: maybe\n"))
case("config.gate.024-i", "config.gate.024", files=app("spring:\n  elasticsearch:\n"))
case("config.gate.024-j", "config.gate.024", env=[("spring.elasticsearch.connection-timeout", "abc")])

# ==========================================================================================
# config.key
# ==========================================================================================
case("config.key.001", ["config.key.001", "config.key.003"], args=NO_FILES,
     note="exactly as the scenario states it: no source sets the version")
case("config.key.001-v", ["config.key.001", "config.key.003"], args=[*NO_FILES, *R4])
case("config.key.002", "config.key.002", args=["--spring.config.location=file:./x.yaml"], files={"x.yaml": (
    "hapi.fhir.fhir_version: R4\n"
    "hapi.fhir.cors.allow_credentials: false\n"
    "hapi.fhir.partitioning.allow_references_across_partitions: false\n"
    "hapi.fhir.elasticsearch.index_prefix: ''\n"
    "hapi.fhir.subscription.email.host: h\n"
    "hapi.fhir.tester.t.name: t\n"
    "hapi.fhir.remote_terminology_service.r.url: u\n")})
case("config.key.004", "config.key.004")
case("config.key.008", "config.key.008", files=app("hapi.fhir.reuse_cached_search_results_millis: 0\n"))
case("config.key.008-b", "config.key.008", files=app("hapi.fhir.reuse_cached_search_results_millis: 1\n"))
case("config.key.009", ["config.key.009", "config.gate.013"], files=app("hapi.fhir.implementationguides.x.name: x\n"))
case("config.key.009-b", "config.key.009", files=app(
    "hapi.fhir.implementationguides.x:\n  name: x\n  installResourceTypes: CodeSystem, ValueSet\n  fetch_dependencies: yes\n"
    "  reloadExisting: false\n"))
case("config.key.010", "config.key.010", files=app(
    "hapi.fhir.cr_enabled: true\nhapi.fhir.install_additional_resources_from_ig_folders: [f]\n"
    "hapi.fhir.subscription.email.auth: true\nhapi.fhir.subscription.email.quitWait: true\n"
    "hapi.fhir.subscription.email.startTlsRequired: true\nhapi.fhir.cr.cql.compiler.translator_format: XML\n"
    "hapi.fhir.cr.cql.use_embedded_libraries: false\n"))
case("config.key.012", "config.key.012", files=app('hapi.fhir.implementationguides.x.packageContents: "65,66"\n'))
case("config.key.012-b", "config.key.012", files=app("hapi.fhir.implementationguides.x.packageContents: QUI=\n"))

# ==========================================================================================
# config.oracle
# ==========================================================================================
case("config.oracle.001", ["config.oracle.001", "config.oracle.002"],
     note="the baseline case; its System.getProperties() is the record's system_properties (design.md section 3.6)")


# ==========================================================================================
# all-keys (config.key.005 - .007)
# ==========================================================================================

def keys_md_defaults() -> dict[str, str]:
    """Path -> class default (JSON text) from keys.md sections 1 and 2."""
    text = (CHANGE / "keys.md").read_text(encoding="utf-8")
    out = {}
    section = text.split("## 1.")[1].split("## 3.")[0]
    for m in re.finditer(r"^\| `([^`]+)` \| `([^`]*)` \|$", section, re.M):
        out[m.group(1)] = m.group(2)
    return out


def schema() -> dict[str, tuple[str, bool, list[str]]]:
    out = {}
    for line in (HERE / "schema.tsv").read_text(encoding="utf-8").splitlines():
        path, typ, setter, consts = (line.split("\t") + [""])[:4]
        out[path] = (typ, setter == "setter", consts.strip("[]").split(", ") if consts else [])
    return out


SPELLINGS = {
    # the packaged file's or the starter documentation's spelling, where it is not snake case
    "hapi.fhir.uplifted-refchains-enabled": "hapi.fhir.upliftedRefchains_enabled",
    "hapi.fhir.user-request-retry-version-conflicts-interceptor-enabled": "hapi.fhir.userRequestRetryVersionConflictsInterceptorEnabled",
    "hapi.fhir.cdshooks.client-id-header-name": "hapi.fhir.cdshooks.clientIdHeaderName",
    "hapi.fhir.cors.allow-credentials": "hapi.fhir.cors.allow_Credentials",
    "hapi.fhir.cr.cql.data.profile-mode": "hapi.fhir.cr.cql.data.profileMode",
    "hapi.fhir.cr.cql.data.search-parameter-mode": "hapi.fhir.cr.cql.data.searchParameterMode",
    "hapi.fhir.cr.cql.terminology.code-lookup-mode": "hapi.fhir.cr.cql.terminology.codeLookupMode",
    "hapi.fhir.cr.cql.terminology.valueset-pre-expansion-mode": "hapi.fhir.cr.cql.terminology.valueset_preexpansion_mode",
    "hapi.fhir.subscription.email.quit-wait": "hapi.fhir.subscription.email.quitWait",
    "hapi.fhir.subscription.email.start-tls-enable": "hapi.fhir.subscription.email.startTlsEnable",
    "hapi.fhir.subscription.email.start-tls-required": "hapi.fhir.subscription.email.startTlsRequired",
    "hapi.fhir.remote-terminology-service": "hapi.fhir.remote_terminology_service",
    "hapi.fhir.implementation-guides": "hapi.fhir.implementationguides",
}
NOT_SET = {
    # bound only to its current value: any other value fails to start (config.unknown.005)
    "hapi.fhir.cr.cql.runtime.enable-validation": "false",
}
MAP_DEFAULTS = {
    # class defaults of map-entry members (config.key.002, config.key.009), for picking a non-default value
    "hapi.fhir.tester.<key>.refuse-to-fetch-third-party-urls": "true",
    "hapi.fhir.tester.<key>.fhir-version": '"R4"',
    "hapi.fhir.implementation-guides.<key>.fetch-dependencies": "false",
    "hapi.fhir.implementation-guides.<key>.reload-existing": "true",
    "hapi.fhir.implementation-guides.<key>.dry-run": "false",
    "hapi.fhir.implementation-guides.<key>.overwrite-content-not-present-code-systems": "false",
    "hapi.fhir.implementation-guides.<key>.version-policy": '"MULTI_VERSION"',
}
SKIP = {"hapi.fhir.remote-terminology-services-map"}  # getter of remote_terminology_service, set through it


def snake(path: str) -> str:
    if path in SPELLINGS:
        return SPELLINGS[path]
    for k, v in SPELLINGS.items():
        if path.startswith(k + "."):
            return v + "." + snake_tail(path[len(k) + 1:])
    head, _, tail = path.partition("hapi.fhir.")
    return "hapi.fhir." + snake_tail(tail)


def snake_tail(t: str) -> str:
    return t.replace("-", "_")


def nondefault(path: str, typ: str, consts: list[str], default: str | None):
    """A YAML scalar or list text, and the env text, for a non-default value."""
    name = path.rsplit(".", 1)[1]
    if typ in ("boolean", "java.lang.Boolean"):
        v = "false" if default == "true" else "true"
        return v, v
    if typ in ("int", "java.lang.Integer", "long", "java.lang.Long"):
        if default in (None, "null"):
            v = "7"
        else:
            n = int(default)
            v = str(n - 1 if n >= 2**31 - 1 else n + 1)
        return v, v
    if typ == "java.lang.String":
        v = f"v-{name}"
        return v, v
    if consts:
        d = (default or "").strip('"')
        if typ.startswith("java.util.List") or typ.startswith("java.util.Set"):
            pick = [c for c in consts if c != "NULL"][:2]
            return "[" + ", ".join(pick) + "]", ",".join(pick)
        pick = next(c for c in reversed(consts) if c != d)
        return pick, pick
    if typ == "java.util.List<java.lang.Integer>":
        return "[5, 6]", "5,6"
    if typ.startswith("java.util.List<java.lang.String>") or typ.startswith("java.util.Set<java.lang.String>"):
        return f"[{name}-b, {name}-a]", f"{name}-b,{name}-a"
    if typ == "byte[]":
        return '"65,66"', "65,66"
    raise SystemExit(f"all-keys: no non-default value rule for {path} ({typ})")


def all_keys():
    defaults = keys_md_defaults()
    sch = schema()
    rows: list[tuple[str, str, str]] = []  # canonical path, yaml text, env text
    containers = ("java.util.Map", "ca.uhn.", "org.opencds.")
    for path, (typ, setter, consts) in sorted(sch.items()):
        if any(path == k or path.startswith(k + ".") for k in SKIP) or ".retrieve-settings" in path:
            continue
        if any(typ.startswith(c) for c in containers) and not consts:
            continue  # an object or map: its members are listed separately
        concrete = path.replace(".<key>", ".alpha")
        if path in NOT_SET:
            rows.append((concrete, NOT_SET[path], NOT_SET[path]))
            continue
        y, e = nondefault(path, typ, consts, defaults.get(path, MAP_DEFAULTS.get(path)))
        rows.append((concrete, y, e))
    # string maps, and the keys.md section 6 entries a schema walk does not reach
    rows.append(("hapi.fhir.cr.cql.namespaces.alpha", "http://alpha", "http://alpha"))
    rows.append(("hapi.fhir.remote-terminology-service.alpha.system", "http://sys", "http://sys"))
    rows.append(("hapi.fhir.remote-terminology-service.alpha.url", "http://url", "http://url"))
    # the CareGapsProperties prefix (the same keys as cr.care-gaps)
    # ElasticsearchProperties (keys.md section 6), bound by the elasticsearch gate
    es = [("spring.elasticsearch.uris", "[http://es1, http://es2]", "http://es1,http://es2"),
          ("spring.elasticsearch.username", "u", "u"), ("spring.elasticsearch.password", "p", "p"),
          ("spring.elasticsearch.connection-timeout", "2s", "2s"), ("spring.elasticsearch.socket-timeout", "31s", "31s"),
          ("spring.elasticsearch.socket-keep-alive", "true", "true"), ("spring.elasticsearch.path-prefix", "/es", "/es"),
          ("spring.elasticsearch.restclient.sniffer.interval", "6m", "6m"),
          ("spring.elasticsearch.restclient.sniffer.delay-after-failure", "2m", "2m"),
          ("spring.elasticsearch.restclient.ssl.bundle", "b", "b")]
    # every keys.md section 1/2 path must be set by some row (a key added there cannot be missed)
    have = {r[0] for r in rows}
    for p in defaults:
        if p in ("hapi.fhir.cors", "hapi.fhir.partitioning", "hapi.fhir.elasticsearch", "hapi.fhir.subscription.email",
                 "hapi.fhir.tester", "hapi.fhir.implementation-guides", "hapi.fhir.remote-terminology-service",
                 "hapi.fhir.cr.cql.namespaces"):
            if not any(h.startswith(p + ".") for h in have):
                raise SystemExit(f"all-keys: nothing set beneath {p}")
            continue
        if p.startswith("hapi.fhir.cr.caregaps."):
            continue  # same keys as hapi.fhir.cr.care-gaps.*
        if p not in have:
            raise SystemExit(f"all-keys: keys.md path {p} is not set")
    yaml_lines, kebab_lines, env_lines = [], [], []
    for path, y, e in rows + es:
        key = snake(path) if path.startswith("hapi.") else path
        yaml_lines.append(f"{yaml_key(key)}: {yaml_value(y)}")
        kebab_lines.append(f"{yaml_key(path)}: {yaml_value(y)}")
        env_lines.append(env_line(path, e))
    return "\n".join(yaml_lines) + "\n", "\n".join(kebab_lines) + "\n", env_lines


def yaml_key(k: str) -> str:
    return k


def yaml_value(v: str) -> str:
    if v.startswith("[") or v.startswith('"'):
        return v
    if v.startswith("v-") or v.startswith("http") or v.startswith("/") or v[0].isalpha() or v[0].isdigit() or v[0] == "-":
        return v
    return f'"{v}"'


def env_line(path: str, value: str) -> tuple[str, str]:
    # The upper-case form where config.env.011 allows it (map keys of lower-case letters only).
    if "namespaces" in path:
        return (path, value)  # a string map entry: the dotted form keeps the key whole
    return (path.upper().replace("-", "").replace(".", "_"), value)


def generate_all_keys():
    y, k, e = all_keys()
    case("all-keys.yaml", "config.key.005", files={"application.yaml": y})
    case("all-keys-kebab.yaml", "config.key.007", files={"application.yaml": k})
    case("all-keys.env", "config.key.006", env=e)


# ==========================================================================================
# writer
# ==========================================================================================

def write():
    if CASES.exists():
        shutil.rmtree(CASES)
    CASES.mkdir()
    for cid, c in ALL.items():
        d = CASES / cid
        d.mkdir()
        lines = [f"# config-roundtrip case {cid}; written by make_cases.py, do not edit",
                 f"covers: {', '.join(c['covers'])}"]
        if c["random"]:
            lines.append(f"random: {', '.join(c['random'])}")
        if c["process"]:
            lines.append(f"process: {', '.join(c['process'])}")
        if c["deviation"]:
            lines.append(f"deviation: {c['deviation']}")
        if c["note"]:
            lines.append(f"note: {c['note']}")
        (d / "case.yaml").write_bytes(("\n".join(lines) + "\n").encode("utf-8"))
        for sub, files in (("files", c["files"]), ("root", c["root"])):
            for rel, content in files.items():
                p = d / sub / rel
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_bytes(content if isinstance(content, bytes) else content.encode("utf-8"))
        if c["env"]:
            for name, value in c["env"]:
                if "\n" in name or "\n" in value or "=" in name:
                    raise SystemExit(f"{cid}: unrepresentable variable {name!r}")
            (d / "env").write_bytes("".join(f"{n}={v}\n" for n, v in c["env"]).encode("utf-8"))
        if c["args"]:
            (d / "args").write_bytes("".join(f"{a}\n" for a in c["args"]).encode("utf-8"))
    print(f"wrote {len(ALL)} cases to {CASES}")


if __name__ == "__main__":
    generate_all_keys()
    write()
