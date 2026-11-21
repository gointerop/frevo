"""Writes probes.json and inputs/ for the differential-only FHIRPath scenarios.

usage: python -I probes.py        (from this directory)

Every scenario of `add-fhirpath-and-codegen-r4` that the vendored suite cannot
grade (`openspec/changes/add-fhirpath-and-codegen-r4/spec.md`, "Thirty-five
scenarios ..." and the six partial ones, plus every scenario of
`specs/path/spec.md` that calls itself differential-only) gets one or more
probes: an expression, an input, and the engine configuration the scenario
states. `harness/hapi-fhirpath-differential` evaluates every probe on HAPI
8.12.0's R4 FHIRPathEngine in-process, under both workers; the
`differential` binary of `harness/fhirpath-compare` evaluates them on Frevo
and writes `../differential-results.json`.

Probe fields (all but id, scenario and expr optional):
  input     "suite:<file>" (oracles/fhir-test-cases/fetched/r4) or "local:<file>" (inputs/)
  check     "skip" (default; FhirPathR4 never checks), "run" (check, then evaluate
            if the check passed) or "only"
  worker    the HAPI worker the scenario states: "suite" (default; the
            TestingUtilities-style SimpleWorkerContext with UCUM, which every
            path.quantity.* scenario but .019 states) or "server"
            (HapiWorkerContext, the worker of FhirPathR4)
  settings  {"asCaseSensitive": bool, "asSingleton": bool, "polymorphic": bool}
            (defaults: false, false, false -- the R4 initFlags and FhirPathR4)
  host      a host service, see HOST below; absent means no host service at all
  app       true to pass a non-null application context
  repeat    evaluate the parsed expression this many times (default 1)
  grade     "exact" (default): stage, items and message; "error": both fail at
            the same stage, message not compared (the scenario says the text
            is the JVM's); "shape": stage and item types only (now())
  documented  the register entry that already records a Frevo/HAPI difference
            for this probe, when the scenario itself says Frevo differs
  note      free text

HOST: {"constants": [[name, mode or "*", [items]]], "constantTypes": [[name,
mode or "*", type]], "functions": [{name, min, max, type, result, typeParams}],
"conformsTo": bool, "valueSets": "worker", "log": bool}. Every call the engine
makes is recorded. Items are "type:value" for string, integer and boolean.
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
PROBES = []
INPUTS = {}


def P(scenario, expr, **kw):
    n = sum(1 for p in PROBES if p["scenario"] == scenario) + 1
    p = {"id": f"{scenario}#{n:02d}", "scenario": scenario, "expr": expr}
    p.update(kw)
    PROBES.append(p)


def inp(name, text):
    INPUTS[name] = text
    return "local:" + name


PAT = "suite:patient-example.xml"
OBS = "suite:observation-example.xml"

# ---------------------------------------------------------------- inputs
OBS_NOVALUE = inp("observation-quantity-novalue.json", json.dumps({
    "resourceType": "Observation", "id": "nv", "status": "final",
    "code": {"text": "x"},
    "valueQuantity": {"unit": "mg", "system": "http://unitsofmeasure.org", "code": "mg"}}, indent=2))

MEDREQ = inp("medicationrequest-positiveint.json", json.dumps({
    "resourceType": "MedicationRequest", "id": "pi", "status": "active", "intent": "order",
    "subject": {"reference": "Patient/example"},
    "medicationCodeableConcept": {"text": "x"},
    "dosageInstruction": [{"timing": {"repeat": {"count": 2, "countMax": 3}}}]}, indent=2))

IMAGING = inp("imagingstudy-unsignedint.json", json.dumps({
    "resourceType": "ImagingStudy", "id": "ui", "status": "available",
    "subject": {"reference": "Patient/example"},
    "numberOfSeries": 2, "numberOfInstances": 3}, indent=2))

MEDIA = inp("media-positiveint.json", json.dumps({
    "resourceType": "Media", "id": "m", "status": "completed",
    "content": {"contentType": "image/png"}, "height": 2, "width": 3}, indent=2))


def params_xml(name, parts):
    body = "\n".join(f'  <parameter>\n    <name value="{n}"/>\n    {v}\n  </parameter>' for n, v in parts)
    return inp(name, f'<Parameters xmlns="http://hl7.org/fhir">\n{body}\n</Parameters>\n')


def dec_input(tag, lexical):
    return params_xml(f"parameters-decimal-{tag}.xml", [("a", f'<valueDecimal value="{lexical}"/>')])


NOVALUE_NUMS = params_xml("parameters-novalue-numbers.xml", [
    ("d", '<valueDecimal><extension url="http://example.org/x"><valueString value="y"/></extension></valueDecimal>'),
    ("i", '<valueInteger><extension url="http://example.org/x"><valueString value="y"/></extension></valueInteger>'),
])

QTY_NONUCUM = params_xml("parameters-quantities-nonucum.xml", [
    ("q1", '<valueQuantity><value value="1"/><system value="http://example.org/u"/><code value="x"/></valueQuantity>'),
    ("q2", '<valueQuantity><value value="1"/><system value="http://example.org/u"/><code value="x"/></valueQuantity>'),
    ("q3", '<valueQuantity><value value="1"/><system value="http://example.org/u"/><code value="y"/></valueQuantity>'),
    ("q4", '<valueQuantity><value value="2"/><system value="http://example.org/u"/><code value="x"/></valueQuantity>'),
    ("u1", '<valueQuantity><value value="1"/><system value="http://unitsofmeasure.org"/></valueQuantity>'),
    ("u0", '<valueQuantity><value value="0"/><system value="http://unitsofmeasure.org"/></valueQuantity>'),
])

DIV = '<div xmlns="http://www.w3.org/1999/xhtml">hi</div>'
PAT_DIV = inp("patient-div.json", json.dumps({
    "resourceType": "Patient", "id": "d", "text": {"status": "generated", "div": DIV}}, indent=2))
PAT_DIV_QUOTE = inp("patient-div-quote.json", json.dumps({
    "resourceType": "Patient", "id": "q", "text": {"status": "generated",
    "div": '<div xmlns="http://www.w3.org/1999/xhtml">say "a"<br></br>b</div>'}}, indent=2))
PAT_DIV_UL = inp("patient-div-ul.json", json.dumps({
    "resourceType": "Patient", "id": "u", "text": {"status": "generated",
    "div": '<div xmlns="http://www.w3.org/1999/xhtml">x<ul><li>a</li><li>b</li></ul></div>'}}, indent=2))
PAT_NODIV = inp("patient-nodiv.json", json.dumps({
    "resourceType": "Patient", "id": "n", "text": {"status": "generated"}}, indent=2))

OBS_CONTAINED = inp("observation-contained.json", json.dumps({
    "resourceType": "Observation", "id": "c", "status": "final",
    "contained": [{"resourceType": "Patient", "id": "p1", "active": True}],
    "code": {"text": "x"},
    "subject": {"reference": "#p1"},
    "performer": [{"reference": "#missing"}, {"identifier": {"system": "http://example.org", "value": "1"}},
                  {"reference": "Patient/example"}]}, indent=2))
OBS_NOCONTAINED = inp("observation-no-contained.json", json.dumps({
    "resourceType": "Observation", "id": "nc", "status": "final", "code": {"text": "x"},
    "subject": {"reference": "#p1"}}, indent=2))

PAT_MODEXT = inp("patient-modifierextension.json", json.dumps({
    "resourceType": "Patient", "id": "me",
    "modifierExtension": [{"url": "http://example.org/x", "valueString": "m"}],
    "extension": [{"url": "http://example.org/y", "valueString": "e"}]}, indent=2))

# ================================================================ path.boundary
S = "path.boundary.012"
for f in ("lowBoundary", "highBoundary"):
    P(S, f"1.587 'cm'.{f}(-1)")
    P(S, f"12.5 'cm'.{f}(-1)")
    P(S, f"1.587 'cm'.{f}(-2)", grade="error", note="StringIndexOutOfBoundsException text is the JDK's")
P(S, "1.587 'cm'.lowBoundary(20)")
P(S, "1.587.lowBoundary(20)")

S = "path.boundary.013"
P(S, "{}.precision()")
P(S, "(1.5 | 2.5).precision()")
P(S, "{}.lowBoundary()")
P(S, "{}.highBoundary()")
P(S, "(1.5 | 2.5).lowBoundary()")
P(S, "(1.5 | 2.5).highBoundary()")

S = "path.boundary.014"
for e in ("1.precision()", "'abc'.precision()", "'abc'.lowBoundary()", "true.highBoundary()", "1 'cm'.precision()"):
    P(S, e)
for e in ("1 'cm'.precision()", "1.precision()", "'abc'.lowBoundary()", "1.lowBoundary()"):
    P(S, e, check="run")
for f in ("lowBoundary()", "highBoundary()", "precision()"):
    P(S, f"Patient.telecom.rank.first().{f}", input=PAT)
    P(S, f"Patient.gender.{f}", input=PAT)
    P(S, f"Patient.meta.lastUpdated.{f}", input="suite:patient-glossy-example.xml")
    P(S, f"ImagingStudy.numberOfSeries.{f}", input=IMAGING, note="an unsignedInt element")

S = "path.boundary.015"
for e in ("@2014-01-15.lowBoundary(7)", "@2014.lowBoundary(10)", "@2014.lowBoundary(5)", "@2014.highBoundary(0)",
          "@2014-01-01T08.lowBoundary(12)", "@2014.lowBoundary(20)", "@2014-01-01T08.lowBoundary(14)",
          "@2014-01-01T08.highBoundary(14)", "@2014-01-01T08:05+08:00.lowBoundary(14)",
          "@2014-01-01T08:05-05:00.highBoundary(14)", "@2014-01-01T08:05-10:00.highBoundary(14)",
          "@2014-01-01T08:05Z.lowBoundary(14)"):
    P(S, e)

S = "path.boundary.016"
for e in ("@T10:30.lowBoundary(2)", "@T10:30.lowBoundary(4)", "@T10:30.lowBoundary(6)", "@T10:30.highBoundary(2)",
          "@T10:30.highBoundary(6)", "@T10:30.lowBoundary(3)", "@T10:30.lowBoundary(8)", "@T10:30.lowBoundary(17)"):
    P(S, e)

S = "path.boundary.017"
for e in ("1.587.lowBoundary(1 | 2)", "1.587.highBoundary(1 | 2)", "1.587.lowBoundary({})",
          "1.587.lowBoundary('2')", "1.587.lowBoundary(1.5)"):
    P(S, e)

S = "path.boundary.018"
for e in ("0.lowBoundary()", "0.highBoundary()", "0.lowBoundary(0)", "0.highBoundary(0)", "0.0.lowBoundary(1)",
          "(-0.0).lowBoundary(1)", "0.000.highBoundary(1)"):
    P(S, e)

S = "path.boundary.019"
for e in ("Observation.value.lowBoundary()", "Observation.value.highBoundary()",
          "Observation.value.lowBoundary(2)", "Observation.value.highBoundary(20)"):
    P(S, e, input=OBS_NOVALUE)

# ================================================================ path.collection / filter
S = "path.collection.010"
P(S, "{}.subsetOf(1 | 2)", check="run")
P(S, "{}.subsetOf(1 | 2)")

S = "path.collection.013"
P(S, "Patient.name.given.distinct()", input=PAT)
P(S, "@2012.combine(@2012-01).distinct()", note="doEquals answers null for the pair")
P(S, "(1 | 2 | 3).distinct()")

S = "path.filter.003"
P(S, "Patient.name.where('x')", input=PAT, check="run")
P(S, "Patient.name.where('x')", input=PAT)

S = "path.filter.009"
P(S, "Patient.ofType(FHIR.Patient).type().name", input=PAT)
P(S, "Patient.ofType(System.Patient)", input=PAT)

# ================================================================ path.compare
S = "path.compare.007"
for op in ("<", "<=", ">", ">="):
    P(S, f"(1 | 2) {op} 3")

S = "path.compare.011"
for op in ("<", "<=", ">", ">="):
    P(S, f"MedicationRequest.dosageInstruction.timing.repeat.count {op} "
         f"MedicationRequest.dosageInstruction.timing.repeat.countMax", input=MEDREQ)
    P(S, f"ImagingStudy.numberOfSeries {op} ImagingStudy.numberOfInstances", input=IMAGING)
    P(S, f"Media.height {op} Media.width", input=MEDIA)

S = "path.compare.014"
P(S, "1 'kg' <= 1 year")

S = "path.compare.015"
L = "@2012-04-15T10:00:00+02:00"
Z = "@2013-01-01T00:00:00Z"
Z5 = "@2015-01-01T00:00:00Z"
for e in (f"({L}).select($this < {Z} and $this.toString() = '2012-04-15T08:00:00Z')",
          f"({L}).select(($this < {Z}) and $this.toString() = '2012-04-15T10:00:00+02:00')",
          f"({L}).where($this < {Z})",
          f"({L}).where($this < {Z}).select($this)",
          f"({L}).select($this)",
          f"now().where($this > @2000-01-01T00:00:00Z).toString().endsWith('Z')",
          f"({L} + 1 day).where($this < {Z})",
          f"{L}.lowBoundary().where($this < {Z})",
          f"@2014-01-01.lowBoundary(17).where($this < {Z5})",
          f"@2014-01-01T08.lowBoundary(8).where($this < {Z5})",
          f"@2014-01-01T08.lowBoundary(14).where($this < {Z5})",
          f"@2014-01-01T08:05+08:00.lowBoundary(14).where($this < {Z5})",
          f"@2014-01-01.lowBoundary(14).where($this < {Z5})",
          f"@2014-01-01T08:05-05:00.highBoundary(14).where($this < {Z5})",
          f"@2014-01-01T08.highBoundary(14).where($this < {Z5})",
          f"(@2012-04-15 + 1 day).where($this < @2013-01-01)",
          f"today().where($this > @2000-01-01).toString().length()",
          f"(@2012-04-15T10:00:00).select($this < {Z} and $this.toString() = '2012-04-15T10:00:00Z')",
          f"(@2012-04-15).where($this < @2013-01-01)",
          f"({L} < {Z}) and {L}.toString() = '2012-04-15T10:00:00+02:00'"):
    P(S, e)
P(S, f"({L}).select($this.toString() = '2012-04-15T10:00:00+02:00' and $this < {Z})", repeat=2)

# ================================================================ path.convert
S = "path.convert.004"
for v in ("2015", "2015-02", "2015-02-04", "2015-02-04T14", "2015-02-04T14:34", "2015-02-04T14:34:28+10:00",
          "abc", "2015-13", "2015-02-04T25"):
    P(S, f"'{v}'.convertsToDate()")
for e in ("@2015.convertsToDate()", "@2015-02-04T14:34.convertsToDate()", "('2015' | '2016').convertsToDate()",
          "{}.convertsToDate()"):
    P(S, e)

S = "path.convert.011"
for e in ("@2014.toDateTime()", "'2015-02-04'.toDateTime()", "{}.toDateTime()"):
    P(S, e)
P(S, "'2015-02-04'.toDateTime()", check="run")
P(S, "Patient.toDateTime()", input=PAT, check="run")
for e in ("@2014.convertsToDateTime()", "'2015-02-04'.convertsToDateTime()", "'2015-02-04T14:34'.convertsToDateTime()",
          "'abc'.convertsToDateTime()", "(1 | 2).convertsToDateTime()"):
    P(S, e)

S = "path.convert.012"
P(S, "'2015'.toDate()")
P(S, "'2015'.convertsToDate()")

S = "path.convert.013"
for e in ("'14:34:28'.toTime()", "'abc'.toTime()", "{}.toTime()"):
    P(S, e)
P(S, "'14:34:28'.toTime()", check="run")
P(S, "Patient.toTime()", input=PAT, check="run")
for v in ("abc", "14", "14:34", "14:34:28", "14:34:28.123"):
    P(S, f"'{v}'.convertsToTime()")

# ================================================================ path.datetime
S = "path.datetime.003"
P(S, "now() = now()", grade="shape", note="two separately read instants; only the type is compared")

# ================================================================ path.env
S = "path.env.015"
H_OWNED = {"constants": [["owned", "*", ["string:owned-value"]]], "constantTypes": [["owned", "*", "string"]]}
H_PCT = {"constants": [["owned", "*", ["string:owned-value"]]], "constantTypes": [["%owned", "*", "string"]]}
P(S, "%owned", host=H_OWNED, note="evaluation strips the %: the host is asked for owned, EXPLICIT")
P(S, "%owned", host=H_OWNED, check="only", note="the check passes %owned with its %: a host keyed owned fails it")
P(S, "%owned", host=H_PCT, check="run", note="a host keyed %owned for types passes the check")
P(S, "name", host={"constantTypes": [["name", "NOVALUE", "string"]]}, check="only",
  note="check with no focus type: the leading name is offered to the host as NOVALUE")
P(S, "name.given", input=PAT, host={"constantTypes": [["name", "*", "string"]]}, check="only",
  note="with a focus type the name is not offered to the host")

S = "path.env.016"
P(S, "name", input=PAT, app=True, host={"constants": [["name", "IMPLICIT_BEFORE", ["string:from-host"]]]})
P(S, "Patient.name", input=PAT, app=True, host={"constants": [["Patient", "IMPLICIT_BEFORE", ["string:host-patient"]]]})
P(S, "Patient", input=PAT, app=True, host={"constants": [["Patient", "IMPLICIT_BEFORE", ["string:host-patient"]]]})
P(S, "Patient.name.where(use = 'official').given.trace('t')", input=PAT, app=True, host={"log": False},
  note="records every name the host is offered")
P(S, "Patient.name.given", input=PAT, app=True, host={}, note="an empty answer leaves navigation alone")
P(S, "name", input=PAT, app=False, host={"constants": [["name", "IMPLICIT_BEFORE", ["string:from-host"]]]},
  note="no application context: the host is not asked")

S = "path.env.017"
H_AFTER = {"constants": [["zzz", "IMPLICIT_AFTER", ["string:after"]]]}
P(S, "zzz", input=PAT, app=True, host=H_AFTER)
P(S, "zzz", input=PAT, app=False, host=H_AFTER)
P(S, "zzz", input=PAT, app=True, host=H_AFTER, check="run")

# ================================================================ path.errors
S = "path.errors.006"
P(S, "-'a'")
P(S, "- 'a'")

S = "path.errors.007"
P(S, "(1 | 2).iif(true, 'a')")
P(S, "1.is('x')")
P(S, "1.is(FHIR.toString())")

# ================================================================ path.math
S = "path.math.015"
for e in ("1 / 2", "1 / 8", "3 / 8", "1 / 64", "1 / 1024", "7 / 2", "19 / 20", "123456789 / 1000", "4 / 2",
          "1 / 3", "2 / 3", "5 / 6", "1 / 7", "1 / 9", "5 / 9", "1 / 6", "10 / 3", "20 / 3", "22 / 7", "100 / 3",
          "200 / 3", "1000 / 3", "1 / 11", "1 / 30", "1 / 300",
          "1 / 3.0", "2 / 3.0", "2 / 3.00", "10 / 3.0", "1 / 8.0", "1 / 0.3", "1 / 0.9", "8 / 0.9", "2 / 0.6",
          "3 / 1.5", "1 / 0.125",
          "1.0 / 3", "1.00 / 3", "10.0 / 3", "10.00 / 3", "0.1 / 3", "0.10 / 3", "1.000 / 7", "1.0 / 8", "1.0 / 16",
          "1.5 / 3", "1.9 / 2", "10.0 / 4", "9.99 / 3",
          "1.2 / 1.8", "4.0 / 2.0", "10.00 / 3.0", "0.5 / 0.3", "1.5 / 0.5", "7.5 / 2.5", "1.25 / 0.5",
          "1.9 / 0.2", "0.95 / 0.1", "3.8 / 0.4", "9.99 / 1.0", "99.9 / 1.0", "1.99 / 2.0", "19 / 2.0", "1.9 / 0.20",
          "95 / 10.0", "(0 - 1) / 3", "(0 - 2) / 3",
          "(1 / 3).toString()", "1 / 3 = 0.3", "1 / 3 = 0.333"):
    P(S, e)
P(S, "Parameters.parameter.value / 3", input=dec_input("1e3", "1e3"))
P(S, "Parameters.parameter.value / 3", input=dec_input("1.5e-1", "1.5e-1"))
P(S, "1 / 3", worker="server", note="no worker is consulted: the server-facing engine answers the same")

S = "path.math.016"
for e in ("2.2 div 1.8", "7.5 div 2", "(0 - 7.5) div 2", "5.5 div 2", "6.0 div 2", "10 div 3.0", "1 div 0.3",
          "10 div 0.3", "0.5 div 1", "1 div 3.0", "0.5 div 0.03", "0.5 mod 0.03", "0.15 mod 0.04", "1.9 div 0.2",
          "0.95 div 0.1", "3000000000.0 div 1",
          "2.2 mod 1.8", "5.5 mod 2", "(0 - 5.5) mod 2", "5.5 mod (0 - 2)", "7 mod 2.5", "10 mod 3.0", "1 mod 0.3",
          "1.9 mod 0.2", "0 mod 2.5",
          "4.0 mod 2", "6.0 mod 2", "0.0 mod 2", "(0 - 4.0) mod 2", "4.0 mod (0 - 2)", "4 mod 2.0", "6 mod 3.0",
          "(0 - 6) mod 3.0", "4.0 mod 2.0", "5 mod 2.5", "7.5 mod 2.5", "10.0 mod 2.5", "3000000000.0 mod 1",
          "4.00 mod 2", "4 mod 2.00", "1.25 mod 0.25", "10 mod 2.5", "12 mod 1.5",
          "1.5 div 0", "1.5 mod 0",
          "0.05 mod 0.03", "0.08 mod 0.03", "0.05 mod 0.05", "0.099 mod 0.04", "(0 - 0.05) mod 0.03",
          "0.05 mod (0 - 0.03)", "0.1 mod 0.03", "0.05 mod 0.3"):
    P(S, e)
for tag, lex, rhs in (("1e3", "1e3", "7"), ("1e3", "1e3", "3"), ("1e3", "1e3", "1"), ("2e3", "2e3", "7"),
                      ("-1e3", "-1e3", "7"), ("3.5e38", "3.5e38", "1"), ("1e2", "1e2", "30"), ("1e3", "1e3", "400"),
                      ("3.5e1", "3.5e1", "1")):
    P(S, f"Parameters.parameter.value mod {rhs}", input=dec_input(tag, lex))

S = "path.math.017"
REFUSED = ("16777216e0", "16777217e0", "16777215.5e0", "-16777220e0", "-33554432e0", "-1234567890.5e0",
           "1234567890.5e0")
ACCEPTED = ("16777215e0", "16777215.4e0", "16777218e0", "-16777216e0", "-16777217e0", "-16777218e0", "1.6777216e7")
for lex in REFUSED + ACCEPTED + ("1e+3", "1E+3"):
    i = dec_input(lex.replace("+", "p").replace("E", "upperE"), lex)  # case-insensitive file systems
    for op in ("/", "div", "mod"):
        P(S, f"Parameters.parameter.value {op} 2", input=i)
P(S, "Parameters.parameter.value / 3", input=dec_input("1.6777216e7", "1.6777216e7"))
for lex in ("1e", "1e-", "e3", "-e3"):
    i = dec_input(lex, lex)
    P(S, "Parameters.parameter.value / 2", input=i, note="HAPI's own parser may refuse the input")
    P(S, "Parameters.parameter.value mod 2", input=i, note="HAPI's own parser may refuse the input")
for op in ("/", "div", "mod"):
    P(S, f"Parameters.parameter.where(name = 'd').value {op} 2", input=NOVALUE_NUMS)
for op in ("div", "mod"):
    P(S, f"Parameters.parameter.where(name = 'i').value {op} 2", input=NOVALUE_NUMS, grade="error",
      note="Integer.parseInt(null): the JDK's message, not graded")
ARABIC = dec_input("arabic", "١٢")
for op in ("/", "div", "mod"):
    P(S, f"Parameters.parameter.value {op} 1", input=ARABIC, documented="DEV-0030",
      note="Frevo reads ASCII digits only; HAPI raises StringIndexOutOfBoundsException")

S = "path.mathfn.004"
for e in ("12345.round(-2)", "1234.5.round(-3)", "10000000000.abs()", "0.0000001.abs()", "0.0001.abs()",
          "12345678.abs()"):
    P(S, e)

S = "path.mathfn.010"
P(S, "{}.exp()")
for f in ("abs()", "ceiling()", "floor()", "sqrt()", "ln()", "log(2)", "power(2)", "round(2)", "truncate()"):
    P(S, "{}." + f)

# ================================================================ path.navigation
S = "path.navigation.025"
for e in ("Patient.text.div", "Patient.text.div is string", "Patient.text.div is xhtml",
          "Patient.text.div is FHIR.string", "Patient.text.div.is(string)", "Patient.text.div.is(FHIR.string)",
          "Patient.text.div.is(xhtml)", "Patient.text.div.ofType(string)", "Patient.text.div.toString()",
          f"Patient.text.div = '{DIV}'", "Patient.text.div.contains('>hi<')", "Patient.text.children().count()",
          "Patient.text.children().ofType(xhtml).count()", "Patient.text.descendants().ofType(xhtml).count()"):
    P(S, e, input=PAT_DIV)
P(S, "Patient.text.div.contains('>hi<')", input=PAT_DIV, check="run")
P(S, "Patient.text.div", input=PAT_DIV_QUOTE)
P(S, "Patient.text.div", input=PAT_DIV_UL)
P(S, "Patient.text.div", input=PAT_NODIV)
P(S, "Patient.text.children().count()", input=PAT_NODIV)

# ================================================================ path.quantity
S = "path.quantity.015"
P(S, "1 'cm' < 1 's'")

S = "path.quantity.017"
for e in ("1 'zzz' = 1 'zzz'", "1 'zzz' = 1 'cm'", "1 'zzz' = 2 'zzz'", "1 'zzz' != 1 'cm'", "1 'cm' = 1 'zzz'",
          "1 'zzz' ~ 1 'zzz'", "1 'zzz' ~ 1 'cm'", "1 'zzz' < 2 'zzz'", "1 'zzz' < 1 'cm'", "1 'cm' < 1 'zzz'",
          "2 'zzz' > 1 'cm'", "1 'zzz' >= 1 'cm'", "1 'zzz' <= 2 'cm'", "1 'zzz'.comparable(1 'cm')",
          "1 'zzz' * 1 'm'", "1 'm' * 1 'zzz'", "1 'zzz' / 1 'm'"):
    P(S, e)

S = "path.quantity.018"
for e in ("1 year < 2 'a'", "1 year > 1 'a'", "1 year < 2 years", "1 year <= 2 'a'", "1 year * 1 'm'",
          "1 'm' * 1 year", "1 year / 1 'm'", "1 'm' / 1 year", "1 year.comparable(1 'a')", "1 year = 1 year",
          "1 week = 7 days", "2 months.comparable(2 months)"):
    P(S, e)
Q = "Parameters.parameter.where(name = '{}').value"
for a, op, b in (("q1", "=", "q2"), ("q1", "=", "q3"), ("q1", "=", "q4"), ("q1", "~", "q2"), ("q1", "~", "q3"),
                 ("q1", "<", "q3"), ("q1", ">", "q3"), ("q1", "<", "q4")):
    P(S, f"{Q.format(a)} {op} {Q.format(b)}", input=QTY_NONUCUM)
for a, b in (("q1", "q2"), ("q1", "q3")):
    P(S, f"{Q.format(a)}.comparable({Q.format(b)})", input=QTY_NONUCUM)
P(S, f"{Q.format('q1')}.comparable(1 'cm')", input=QTY_NONUCUM)
P(S, f"{Q.format('q1')} * 1 'm'", input=QTY_NONUCUM)
P(S, f"{Q.format('q1')} / 1 'm'", input=QTY_NONUCUM)

S = "path.quantity.019"
SRV = dict(worker="server", documented="DEV-0029")
for e in ("1 'mg' = 1 'mg'", "1 'mg' ~ 1 'mg'", "1 'mg' != 2 'mg'", "6 days < 1 week", "8 days > 1 week",
          "1 'cm' < 1 's'", "2.0 'cm' * 2.0 'm'", "4.0 'g' / 2.0 'm'", "1 'cm'.comparable(1 '[in_i]')",
          "1 'zzz' = 1 'cm'"):
    P(S, e, **SRV)
for e in ("1 'cm'.comparable(2 'cm')", "1 'cm'.comparable(1 'cm')", "1 year.comparable(1 'a')",
          "1 'cm' <= 2 'cm'", "2 'cm' > 1 'cm'", "1 'cm' < 2 'cm'", "1 'kg' <= 1 year"):
    P(S, e, worker="server", note="never consults the worker: same answer under both workers")
P(S, "Parameters.parameter[0].value <= Parameters.parameter[2].value", input="suite:parameters-example-quantities.xml",
  worker="server")

S = "path.quantity.020"
for q in ("4.0000 'g'", "4000.0 'mg'", "4 'g'", "4000 'mg'", "4040 'mg'", "1 'kg'", "1 '[lb_av]'", "185 '[lb_av]'",
          "1 '[in_i]'", "2.54 'cm'", "1 '[ft_i]'", "12 '[in_i]'", "1.5 'km'", "7 'd'", "1 'wk'", "1 'h'", "1 'a'",
          "1 'mo'", "1 'mo_s'", "1 'L'", "1 'l'", "1 'mL'", "1 'mg/dL'", "1 'mm[Hg]'", "1 '/min'", "1 '%'",
          "1 '[pi]'", "1 'mol/L'", "60 '/min'", "60.0 '/min'", "1 'g/m'", "1 'kg.m/s2'", "1 '10*3/uL'",
          "1 '[ppth]'", "1 '10*-3'", "1 '{tablets}'", "1 '[iU]'", "1 'm/s.s'", "1 'm/(s.s)'", "1 '1'"):
    P(S, f"({q} * 1 '1').toString()")
P(S, "(-5.5 'mg' * 1 '1').toString()")
for e in ("1 '[pi]' * 1 'm'", "2 '[pi]' * 1 '1'", "1 '[pi]' = 1 '1'", "1 '[pi]' / 1 '1'", "(1 '[pi]' * 1 '1').value",
          "1 '[pi]' < 4 '1'"):
    P(S, e, note="the [pi] factor carries 65 significant digits")

S = "path.quantity.021"
for e in ("2.0 'cm' * 2.0 'm'", "2 'cm' * 2 'm'", "2 'm' * 3 'm'", "2 'm/s' * 3 's'", "2 'g' * 3 '1'",
          "2 'm' * 3 '{tablets}'", "2 'kg' * 3 '[lb_av]'", "2 'mm[Hg]' * 3 'm2'", "2 '%' * 3 '%'",
          "4.0 'g' / 2.0 'm'", "1.0 'm' / 1.0 'm'", "(1.0 'm' / 1.0 'm').toString()", "2 'g' / 4 'g'",
          "1 'm' / 3 's'", "6 'km' / 2 'h'", "6 'kg.m' / 2 's'", "6 'g/m' / 2 'm/s'", "6 'g/m' / 3 's'",
          "6 'm' / 3 's/m'", "6 'm.s' / 3 'g'", "6 '1' / 3 'm'", "6 '10*3' / 3 '1'",
          "2.0 'cm' * 2.0 'm' = 0.040 'm2'", "4.0 'g' / 2.0 'm' = 2 'g/m'", "1.0 'm' / 1.0 'm' = 1 '1'",
          "1 'm' / 0 's'", "0 'm' / 0 's'", "1 'm' * 2", "1 'm' / 2", "2 * 1 'm'"):
    P(S, e)
for e in (f"{Q.format('u1')} * 1 'm'", f"{Q.format('u1')} / 1 'm'", f"1 'm' / {Q.format('u1')}",
          f"{Q.format('u1')} / 0 'm'", f"{Q.format('u0')} / 0 'm'", f"{Q.format('u1')} = 1 '1'"):
    P(S, e, input=QTY_NONUCUM)
# An empty unit literal: Quantity.setCode("") leaves the code absent
# (Quantity.java:620-629), so the literal is a UCUM quantity with no code
# (this scenario).
for e in ("1 '' = 1 '1'", "1 '' / 1 'm'", "1 ''.toString()", "1 ''.comparable(1 '1')"):
    P(S, e, note="an empty unit literal")
# The code, system and absent unit this scenario states, observed by navigating
# into the result; upstream builds every quantity value as a FHIR Quantity.
for e in ("(1 'cm').code", "(1 'cm').value", "(1 'cm').unit", "(1 'cm').system", "(1 year).unit",
          "(2 'cm' * 1 'm').code"):
    P(S, e, note="a quantity literal's elements")

S = "path.quantity.022"
for e in ("1 'cm' = 1 's'", "1 'g' != 1 'm'", "1 'cm' ~ 1 's'", "4.0000 'g' = 4000.0 'mg'", "4 'g' = 4000 'mg'",
          "1 '[in_i]' = 2.54 'cm'", "1 'L' = 1000 'mL'", "1 'h' = 60 'min'", "1 'a' = 365.25 'd'",
          "1 'kg' = 1000 'g'", "1 'g/L' = 1000 'mg/L'", "185 '[lb_av]' = 83914.58845 'g'", "100 '%' = 1 '1'",
          "1 '10*3' = 1000 '1'", "60 '/min' = 1 '/s'", "4 'g' = 4040 'mg'", "1 'mo' = 30 'd'",
          "1 'mg/dL' = 10 'mg/L'", "10 'mg/L' = 1 'mg/dL'", "60.0 '/min' = 1 '/s'",
          "1 'kg' < 2000 'g'", "4 'g' < 4040 'mg'", "1 'mo' < 30 'd'", "6 days < 1 week", "1 'cm' < 1 's'",
          "1 'kg' ~ 1000 'g'", "1 'kg' > 999 'g'", "1 'kg' >= 1000 'g'", "1 'kg' <= 1000 'g'"):
    P(S, e)

S = "path.quantity.023"
# A ' inside a FHIRPath string literal is written \'.
for u in ("Cel", "[degF]", "[degRe]", r"[hp\'_X]", r"[hp\'_C]", r"[hp\'_M]", r"[hp\'_Q]", "B[10.nV]", "[m/s2/Hz^(1/2)]"):
    P(S, f"1 '{u}' * 1 'm'")
for e in ("1 '[pH]' = 1 'mol/L'", "1 'B' = 1 '1'", "1 'Np' = 1 '1'", "1 'B[SPL]' = 2 '10*-5.Pa'",
          "1 '%[slope]' = 1 'deg'", r"1 '[p\'diop]' = 1 'deg'", "1 'bit_s' = 1 '1'", "1 'B[V]' = 1 'V'",
          "1 'B[kW]' = 1 'kW'",
          "1 'Cel' = 1 'Cel'", "1 'Cel' = 274.15 'K'", "1 'Cel' ~ 274.15 'K'", "1 'Cel' < 2 'K'",
          "1 'Cel'.comparable(1 'K')", "1 'Cel' / 1 'm'"):
    P(S, e)
# The same codes from data, where no FHIRPath string escape is involved.
QTY_SPECIAL = params_xml("parameters-quantities-special.xml", [
    ("hx", '<valueQuantity><value value="1"/><system value="http://unitsofmeasure.org"/><code value="[hp\'_X]"/></valueQuantity>'),
    ("pd", '<valueQuantity><value value="1"/><system value="http://unitsofmeasure.org"/><code value="[p\'diop]"/></valueQuantity>'),
])
P(S, r"(1 '[hp\'_X]').code", note="what the literal's code is")
P(S, f"{Q.format('hx')}.code", input=QTY_SPECIAL)
P(S, f"{Q.format('hx')} * 1 'm'", input=QTY_SPECIAL)
P(S, f"{Q.format('pd')} = 1 'deg'", input=QTY_SPECIAL)

S = "path.quantity.024"
for code in ("zzz", "m(s", ".m", "m.)", "{abc", "+", "m+", "m-x", "m+.", "[[m]]", "[m[", "m[[g]]", "m]", "m)m", "m)",
             "m#", "", "m2", "10*3", "m10000000000", "10000000000"):
    P(S, f"1 '{code}' * 1 'm'")
    P(S, f"1 'm' * 1 '{code}'")
for e in ("1 '10000000000' = 1 'm'", "1 '10000000000' = 1 '10000000000'", "1 '10000000000' ~ 1 'm'",
          "1 '10000000000' < 1 'm'", "1 'm10000000000' / 1 'm'", "1 '10000000000'.comparable(1 'm')",
          "1 'cm'.comparable(1 '[s]')", "1 'cm'.comparable(1 '[in_i]')", "1 'm(s' = 1 'm'", "1 'm(s' ~ 1 'm'",
          "1 'm(s'.comparable(1 'm')", "1 'm(s' / 1 'm'", "1 'm(s' < 1 'm'", "1 'm)' = 1 'm'"):
    P(S, e)

# ================================================================ path.strictness
S = "path.strictness.005"
for w in ("server", "suite"):
    doc = {} if w == "server" else {"documented": "DEV-0026"}
    P(S, "Patient.gender.as(Code)", input=PAT, worker=w, **doc)
    P(S, "Patient.gender.as(Code)", input=PAT, worker=w, settings={"asCaseSensitive": True}, **doc)
    P(S, "Patient.gender as Code", input=PAT, worker=w, **doc)
    P(S, "Patient.gender as Code", input=PAT, worker=w, settings={"asCaseSensitive": True}, **doc)
    P(S, "Patient.gender.ofType(Code)", input=PAT, worker=w, **doc)
    P(S, "Patient.gender.ofType(Code)", input=PAT, worker=w, settings={"asCaseSensitive": True}, **doc)
    P(S, "Observation.value.as(quantity)", input=OBS, worker=w)
    P(S, "Observation.value.as(quantity)", input=OBS, worker=w, settings={"asCaseSensitive": True})

S = "path.strictness.006"
P(S, "Observation.valueQuantity.exists()", input=OBS, check="run")
P(S, "Observation.valueQuantity.exists()", input=OBS, check="run", settings={"polymorphic": True})
P(S, "Observation.value", input=OBS, settings={"polymorphic": True})
P(S, "Observation.value.exists()", input=OBS, settings={"polymorphic": True})
P(S, "Observation.value", input=OBS)

# ================================================================ path.string
S = "path.string.019"
for e in (r"'a b&c=d/é~'.escape('url')", r"'a+b%20c'.unescape('url')",
          r"'a*b&c#d[e>f<g`h'.escape('md')", r"'a]b_c(d)e\\f'.escape('md')", r"'a\\*b'.unescape('md')",
          r"'a\\xb'.unescape('md')", r"'a\\'.unescape('md')", r"'\\\\*'.unescape('md')", r"'a'.escape('xyz')",
          r"'a'.unescape('xyz')", r"'\\x'.unescape('json')",
          r"'\\\"1<2\\\"'.unescape('json')", r"'\\u0041B'.unescape('json')", r"'\"1<2\"'.escape('json')"):
    P(S, e)
# The scenario names these failures, not their text: an IllegalArgumentException
# from URLDecoder, and an index error.
P(S, r"'%zz'.unescape('url')", grade="error")
P(S, r"'a\\'.unescape('json')", grade="error")

S = "path.string.025"
D23 = {"documented": "DEV-0023"}
for pat in (r"(?u)a", r"(?U)a", r"(?x)a", r"\\v", r"a*+", r"\\p{Alpha}", r"[a-z--b]", r"[a-z~~b]"):
    P(S, f"'a'.matches('{pat}')", **D23)
    P(S, f"'a'.matchesFull('{pat}')", **D23)
    P(S, f"'a'.replaceMatches('{pat}', 'b')", **D23)
P(S, r"'-'.matches('[a-z--b]')", **D23)
P(S, r"'b'.matches('[a-z--b]')", **D23)
for e in (r"'aa'.matches('(a)\\1')", r"'ab'.matches('a(?=b)')", r"'a'.matches('(?>a)')", r"'a.'.matches('\\Qa.\\E')"):
    P(S, e, **D23)
P(S, r"'a'.matches('[a--b]')", grade="error", note="an error on both sides; the message is java.util.regex's")
P(S, r"'a'.matches('(a')", grade="error", note="an error on both sides; the message is java.util.regex's")

S = "path.string.026"
for e in (r"'٣'.matches('\\d')", r"'é'.matches('\\w')", r"'é'.matches('\\W')", "'a b'.matches('a\\\\sb')",
          r"'7'.matches('\\d')", r"'_'.matches('\\w')", r"'a b'.matches('a\\sb')", r"'ab'.matches('\\bab\\b')",
          r"'é'.matches('[\\w]')", r"'٣'.matches('\\D')", "'a b'.matches('a\\\\Sb')", r"'ab'.matches('a\\Bb')",
          r"'é'.matches('\\b')", r"'7'.matches('\\D')"):
    P(S, e)

S = "path.string.028"
for e in ("'abc'.replaceMatches('x*', '-')", "'abc'.replaceMatches('b*', '-')",
          "'\U0001F600'.replaceMatches('x*', '-').length()", "'a\U0001F600'.replaceMatches('x*', '-').length()",
          "'\U0001F600'.replaceMatches('x*', '-').substring(0, 1)",
          "'\U0001F600'.replaceMatches('x*', '-').substring(2, 1)",
          "'\U0001F600'.replaceMatches('x*', '-').substring(4)",
          "'\U0001F600'.replaceMatches('x*', '-').substring(1, 1) = '\U0001F600'.substring(0, 1)",
          "'abc'.replaceMatches('', 'x')"):
    P(S, e)

S = "path.string.029"
P(S, "'a'.matches('(?:a{1000}){1000}')", **D23)
P(S, "'a'.matches('" + "(" * 200 + "a" + ")" * 200 + "')", **D23)
P(S, "'a'.matches('" + "(" * 50 + "a" + ")" * 50 + "')")
P(S, "'a'.matches('a{3}')")

# ================================================================ path.subset
S = "path.subset.003"
P(S, "Patient.name.prefix.single()", input=PAT)

S = "path.subset.013"
for f in ("[0]", ".first()", ".last()", ".tail()", ".skip(1)", ".take(1)"):
    P(S, f"Patient.children(){f}", input=PAT, check="only")
P(S, "Patient.name.first()", input=PAT, check="only", note="an ordered focus passes")

# ================================================================ path.utility
S = "path.utility.017"
for e in ("Observation.subject.resolve().id", "Observation.subject.resolve().active", "'#p1'.resolve().id",
          "Observation.performer.resolve()", "Observation.performer.count()"):
    P(S, e, input=OBS_CONTAINED)
P(S, "Observation.subject.resolve()", input=OBS_NOCONTAINED)

S = "path.utility.022"
U = "'http://hl7.org/fhir/StructureDefinition/Patient'"
P(S, f"Patient.conformsTo({U})", input=PAT)
P(S, f"Patient.conformsTo({U})", input=PAT, host={"conformsTo": True})
P(S, f"Patient.name.conformsTo({U})", input=PAT, host={"conformsTo": True})
P(S, f"{{}}.conformsTo({U})", input=PAT, host={"conformsTo": True})

S = "path.utility.023"
P(S, "Patient.extension('http://example.org/x').value", input=PAT_MODEXT)
P(S, "Patient.extension('http://example.org/y').value", input=PAT_MODEXT)
P(S, "Patient.extension.count()", input=PAT_MODEXT)

S = "path.utility.024"
VS = "'http://hl7.org/fhir/ValueSet/name-use'"
VSH = {"valueSets": "worker"}
P(S, f"Patient.name.use memberOf {VS}", input=PAT, host=VSH)
P(S, f"Patient.name.use.memberOf({VS})", input=PAT, host=VSH)
P(S, f"Patient.name.first().use.memberOf({VS})", input=PAT, host=VSH)
P(S, f"Patient.gender memberOf {VS}", input=PAT, host=VSH)

S = "path.utility.027"
FN = {"functions": [{"name": "custom", "min": 0, "max": 1, "type": "string", "result": ["string:custom-result"]},
                    {"name": "typed", "min": 1, "max": 1, "type": "boolean", "result": ["boolean:true"],
                     "typeParams": [0]}]}
P(S, "Patient.custom()", input=PAT, host=FN)
P(S, "Patient.custom(name.given)", input=PAT, host=FN)
P(S, "Patient.custom(name.given)", input=PAT, host=FN, check="run")
P(S, "Patient.typed(Patient)", input=PAT, host=FN)
P(S, "Patient.custom(1, 2)", input=PAT, host=FN, note="more arguments than the host declares")
P(S, "Patient.custom()", input=PAT, note="no host service: the name is not a function")

S = "path.utility.028"
TR = "Patient.name.given.trace('n').count()"
P(S, TR, input=PAT, host={"log": True})
P(S, TR, input=PAT, host={"log": False})
P(S, TR, input=PAT)
P(S, "Patient.name.given.trace('n', $this.length()).count()", input=PAT, host={"log": True})
P(S, "2.log(2)", host={"log": True}, note="the math log() never reaches the hook")


def main():
    os.makedirs(os.path.join(HERE, "inputs"), exist_ok=True)
    for name, text in sorted(INPUTS.items()):
        with open(os.path.join(HERE, "inputs", name), "w", encoding="utf-8", newline="\n") as f:
            f.write(text if text.endswith("\n") else text + "\n")
    with open(os.path.join(HERE, "probes.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(PROBES, f, indent=1, ensure_ascii=True)
        f.write("\n")
    print(f"{len(PROBES)} probes over {len({p['scenario'] for p in PROBES})} scenarios, {len(INPUTS)} inputs")


if __name__ == "__main__":
    main()
