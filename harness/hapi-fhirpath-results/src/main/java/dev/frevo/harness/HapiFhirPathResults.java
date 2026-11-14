// Derived from hapifhir/org.hl7.fhir.core
// org.hl7.fhir.r4/src/test/java/org/hl7/fhir/r4/test/FHIRPathTests.java at
// 73c432b5f43ddbce979978a760ec4aa81e929094.
// Original work Copyright HAPI FHIR / Health Level Seven contributors, licensed
// under the Apache License, Version 2.0.
// Modified by the Frevo authors: JUnit parameterisation replaced by a loop that
// runs every <test> of the pinned suite file, catches each failure and records
// the outcome as JSON; resources are read from a directory argument instead of
// TestingUtilities.loadTestResource. The assertions and their order are the
// upstream ones.
package dev.frevo.harness;

import java.io.File;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.*;
import javax.xml.parsers.DocumentBuilderFactory;

import org.apache.commons.lang3.NotImplementedException;
import org.hl7.fhir.exceptions.FHIRException;
import org.hl7.fhir.r4.fhirpath.*;
import org.hl7.fhir.r4.fhirpath.FHIRPathUtilityClasses.FunctionDetails;
import org.hl7.fhir.r4.formats.JsonParser;
import org.hl7.fhir.r4.formats.XmlParser;
import org.hl7.fhir.r4.model.*;
import org.hl7.fhir.utilities.fhirpath.FHIRPathConstantEvaluationMode;
import org.w3c.dom.Document;
import org.w3c.dom.Element;
import org.w3c.dom.Node;

public class HapiFhirPathResults {

  enum Fail { OK, SYNTAX, SEMANTICS, EXECUTION }

  static class Services implements IHostApplicationServices {
    public List<Base> resolveConstant(FHIRPathEngine e, Object a, String n, FHIRPathConstantEvaluationMode m) {
      throw new NotImplementedException("resolveConstant");
    }
    public TypeDetails resolveConstantType(FHIRPathEngine e, Object a, String n, FHIRPathConstantEvaluationMode m) {
      throw new NotImplementedException("resolveConstantType");
    }
    public boolean log(String argument, List<Base> focus) { return false; }
    public FunctionDetails resolveFunction(FHIRPathEngine e, String f) {
      throw new NotImplementedException("resolveFunction " + f);
    }
    public TypeDetails checkFunction(FHIRPathEngine e, Object a, String f, TypeDetails focus, List<TypeDetails> p) {
      throw new NotImplementedException("checkFunction");
    }
    public List<Base> executeFunction(FHIRPathEngine e, Object a, List<Base> focus, String f, List<List<Base>> p) {
      throw new NotImplementedException("executeFunction");
    }
    public Base resolveReference(FHIRPathEngine e, Object a, String url, Identifier id, Base base) {
      throw new NotImplementedException("resolveReference");
    }
    public boolean conformsToProfile(FHIRPathEngine e, Object a, Base item, String url) {
      if (url.equals("http://hl7.org/fhir/StructureDefinition/Patient")) return true;
      if (url.equals("http://hl7.org/fhir/StructureDefinition/Person")) return false;
      throw new FHIRException("unknown profile " + url);
    }
    public ValueSet resolveValueSet(FHIRPathEngine e, Object a, String url) {
      return context.fetchResource(ValueSet.class, url);
    }
    public boolean paramIsType(String name, int index) { return false; }
  }

  static void check(boolean cond, String msg) { if (!cond) throw new AssertionError(msg); }
  static void eq(Object exp, Object act, String msg) {
    if (!Objects.equals(exp, act)) throw new AssertionError(msg);
  }
  static boolean noString(String s) { return s == null || s.isEmpty(); }
  static Element named(Element p, String n) {
    for (Node c = p.getFirstChild(); c != null; c = c.getNextSibling())
      if (c instanceof Element && ((Element) c).getTagName().equals(n)) return (Element) c;
    return null;
  }
  static List<Element> namedAll(Element p, String n) {
    List<Element> r = new ArrayList<>();
    for (Node c = p.getFirstChild(); c != null; c = c.getNextSibling())
      if (c instanceof Element && ((Element) c).getTagName().equals(n)) r.add((Element) c);
    return r;
  }

  static FHIRPathEngine fp;
  static org.hl7.fhir.r4.context.IWorkerContext context;
  static final Map<String, Resource> resources = new HashMap<>();
  static File inputDir;

  @SuppressWarnings({"deprecation", "rawtypes"})
  static void runCase(Element test) throws Exception {
    TimeZone.setDefault(TimeZone.getTimeZone("UTC+1100"));
    fp.setHostServices(new Services());
    String input = test.getAttribute("inputfile");
    Element exprEl = named(test, "expression");
    String expression = exprEl.getTextContent();
    Fail fail = Fail.OK;
    String inv = exprEl.getAttribute("invalid");
    if ("syntax".equals(inv)) fail = Fail.SYNTAX;
    else if ("semantic".equals(inv)) fail = Fail.SEMANTICS;
    else if ("execution".equals(inv)) fail = Fail.EXECUTION;
    Resource res = null;
    List<Base> outcome = new ArrayList<>();
    ExpressionNode node = null;
    try {
      node = fp.parse(expression);
      check(fail != Fail.SYNTAX, "Expected exception did not occur parsing " + expression);
    } catch (Exception e) {
      check(fail == Fail.SYNTAX, "Unexpected exception parsing " + expression + ": " + e.getMessage());
    }
    if (node != null) {
      if (!noString(input)) {
        res = resources.get(input);
        if (res == null) {
          byte[] b = Files.readAllBytes(new File(inputDir, input).toPath());
          res = input.endsWith(".json") ? new JsonParser().parse(b) : new XmlParser().parse(b);
          resources.put(input, res);
        }
      }
      try {
        if (noString(input)) fp.check(null, null, null, node);
        else fp.check(res, "Resource", res.getResourceType().toString(), res.getResourceType().toString(), node);
        check(fail != Fail.SEMANTICS, "Expected exception did not occur checking " + expression);
      } catch (Exception e) {
        check(fail == Fail.SEMANTICS, "Unexpected exception checking " + expression + ": " + e.getMessage());
        node = null;
      }
    }
    fp.setDoNotEnforceAsCaseSensitive(false);
    fp.setDoNotEnforceAsSingletonRule(false);
    if (node != null) {
      try {
        outcome = fp.evaluate(res, node);
        check(fail == Fail.OK, "Expected exception did not occur executing " + expression);
      } catch (Exception e) {
        check(fail == Fail.EXECUTION, "Unexpected exception executing " + expression + ": " + e.getMessage());
        node = null;
      }
    }
    if (fp.hasLog()) fp.takeLog();
    if (node != null) {
      if ("true".equals(test.getAttribute("predicate"))) {
        boolean ok = fp.convertToBoolean(outcome);
        outcome.clear();
        outcome.add(new BooleanType(ok));
      }
      List<Element> expected = namedAll(test, "output");
      eq(outcome.size(), expected.size(), "Expected " + expected.size() + " objects but found " + outcome.size() + " for expression " + expression);
      if ("false".equals(test.getAttribute("ordered"))) {
        for (int i = 0; i < Math.min(outcome.size(), expected.size()); i++) {
          String tn = outcome.get(i).fhirType();
          String s = outcome.get(i) instanceof Quantity ? fp.convertToString(outcome.get(i))
              : ((PrimitiveType) outcome.get(i)).asStringValue();
          boolean found = false;
          for (Element e : expected) {
            if ((noString(e.getAttribute("type")) || e.getAttribute("type").equals(tn))
                && (noString(e.getTextContent()) || e.getTextContent().equals(s))) found = true;
          }
          check(found, "Outcome " + i + ": Value " + s + " of type " + tn + " not expected for " + expression);
        }
      } else {
        for (int i = 0; i < Math.min(outcome.size(), expected.size()); i++) {
          String tn = expected.get(i).getAttribute("type");
          if (!noString(tn))
            eq(tn, outcome.get(i).fhirType(), "Outcome " + i + ": Type should be " + tn + " but was " + outcome.get(i).fhirType());
          String v = expected.get(i).getTextContent();
          if (!noString(v)) {
            if (outcome.get(i) instanceof Quantity) {
              Quantity q = fp.parseQuantityString(v);
              check(outcome.get(i).equalsDeep(q), "Outcome " + i + ": Value should be " + v + " but was " + outcome.get(i));
            } else {
              check(outcome.get(i) instanceof PrimitiveType, "Outcome " + i + ": Value should be a primitive type but was " + outcome.get(i).fhirType());
              eq(v, ((PrimitiveType) outcome.get(i)).fpValue(),
                  "Outcome " + i + ": Value should be " + v + " but was " + ((PrimitiveType) outcome.get(i)).fpValue() + " for expression " + expression);
            }
          }
        }
      }
    }
  }

  static String esc(String s) {
    StringBuilder b = new StringBuilder();
    for (char c : s.toCharArray()) {
      if (c == '"') b.append("\\\"");
      else if (c == '\\') b.append("\\\\");
      else if (c < 0x20) b.append(String.format("\\u%04x", (int) c));
      else b.append(c);
    }
    return b.toString();
  }

  public static void main(String[] args) throws Exception {
    File suite = new File(args[0]);
    inputDir = new File(args[1]);
    File out = new File(args[2]);
    DocumentBuilderFactory f = DocumentBuilderFactory.newInstance();
    Document dom = f.newDocumentBuilder().parse(suite);
    org.hl7.fhir.r4.context.IWorkerContext ctx = org.hl7.fhir.r4.context.SimpleWorkerContext.fromPackage(
        new org.hl7.fhir.utilities.npm.FilesystemPackageCacheManager.Builder().build().loadPackage("hl7.fhir.r4.core", "4.0.1"));
    ctx.setUcumService(new org.fhir.ucum.UcumEssenceService(args[3]));
    ctx.setExpansionProfile(new Parameters());
    context = ctx;
    fp = new FHIRPathEngine(ctx);
    List<String> rows = new ArrayList<>();
    for (Element g : namedAll(dom.getDocumentElement(), "group")) {
      int idx = 0;
      for (Node c = g.getFirstChild(); c != null; c = c.getNextSibling()) {
        if (!(c instanceof Element)) continue;
        idx++; // positional index among the group's child elements, as FHIRPathTests.getName
        Element e = (Element) c;
        if (!e.getTagName().equals("test")) continue;
        String addr = g.getAttribute("name") + "/" + e.getAttribute("name") + "[" + idx + "]";
        String status = "passed", msg = "";
        try {
          runCase(e);
        } catch (Throwable t) {
          status = "failed";
          msg = t.getClass().getSimpleName() + ": " + String.valueOf(t.getMessage());
        }
        rows.add("{\"address\":\"" + esc(addr) + "\",\"measured\":\"" + status + "\",\"message\":\"" + esc(msg) + "\"}");
      }
    }
    try (PrintWriter w = new PrintWriter(out, StandardCharsets.UTF_8)) {
      w.println("[" + String.join(",\n", rows) + "]");
    }
    System.out.println("cases: " + rows.size());
  }
}
