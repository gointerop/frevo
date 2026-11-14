"""Turns the driver's raw measurements into oracles/fhirpath/hapi-8.12.0-results.json.

usage: python postprocess.py <suite.xml> <raw.json> <out.json>

Status rules (see oracles/fhirpath/hapi-8.12.0-results.README.md):
  hapi-cannot-pass  cases whose expression text and inputfile are byte-identical to
                    another case in the same group, that differ only in
                    context/contextType, and whose expected outcomes differ. HAPI's
                    R4 harness reads neither attribute, so it cannot tell them apart.
  passed / failed   what the driver measured, for every other case.
"""
import json, sys
import xml.etree.ElementTree as ET

suite, raw, out = sys.argv[1:4]
root = ET.parse(suite).getroot()
measured = {r["address"]: r for r in json.load(open(raw, encoding="utf-8"))}

cases = []
for g in root.findall("group"):
    for idx, el in enumerate(list(g), start=1):  # comments are not elements in ElementTree
        if el.tag != "test":
            continue
        ex = el.find("expression")
        outs = tuple((o.get("type", ""), o.text or "") for o in el.findall("output"))
        outcome = (ex.get("invalid", ""), outs, el.get("predicate", ""), el.get("ordered", ""))
        cases.append(dict(
            addr=f'{g.get("name")}/{el.get("name")}[{idx}]', group=g.get("name"),
            expr=ex.text or "", inputfile=el.get("inputfile", ""),
            ctx=(el.get("context", ""), el.get("contextType", "")), outcome=outcome))

clusters = {}
for c in cases:
    clusters.setdefault((c["group"], c["expr"], c["inputfile"]), []).append(c)
cannot = set()
for members in clusters.values():
    if len({m["ctx"] for m in members}) > 1 and len({m["outcome"] for m in members}) > 1:
        cannot.update(m["addr"] for m in members)

result = []
for c in cases:
    m = measured[c["addr"]]
    if c["addr"] in cannot:
        reason = "reads neither context nor contextType; HAPI measured: " + m["measured"]
        if m["message"]:
            reason += " (" + m["message"] + ")"
        result.append({"address": c["addr"], "status": "hapi-cannot-pass", "reason": reason})
    elif m["measured"] == "passed":
        result.append({"address": c["addr"], "status": "passed"})
    else:
        result.append({"address": c["addr"], "status": "failed", "reason": m["message"]})

assert len(result) == len({r["address"] for r in result}) == len(measured)
doc = {"engine": "org.hl7.fhir.r4.test.FHIRPathTests",
       "sha": "73c432b5f43ddbce979978a760ec4aa81e929094",
       "suite": "oracles/fhirpath/tests-fhir-r4.xml", "cases": result}
with open(out, "w", encoding="utf-8", newline="\n") as f:
    f.write("{" + ",\n".join(f'"{k}":{json.dumps(v, ensure_ascii=False)}' for k, v in doc.items() if k != "cases"))
    f.write(',\n"cases":[\n' + ",\n".join(json.dumps(r, ensure_ascii=False) for r in result) + "\n]}\n")
print(len(result), "cases; cannot-pass:", sorted(cannot))
