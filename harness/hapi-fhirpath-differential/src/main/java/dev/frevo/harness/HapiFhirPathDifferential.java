// The in-process HAPI 8.12.0 probe for the differential-only FHIRPath scenarios
// of add-fhirpath-and-codegen-r4. Original Frevo code; it calls, and does not
// copy, the pinned engine (ca.uhn.hapi.fhir:org.hl7.fhir.r4 6.9.12,
// hapifhir/org.hl7.fhir.core 73c432b5f43ddbce979978a760ec4aa81e929094) and
// HAPI's server-facing worker (hapi-fhir-structures-r4 8.12.0,
// hapifhir/hapi-fhir 7ab4c7c7c49281496faa1b3c2498a652457669ae).
//
// usage: java -cp ... dev.frevo.harness.HapiFhirPathDifferential
//          <probes.json> <suite input dir> <local input dir> <ucum-essence.xml> <out.json>
//
// Every probe is evaluated twice, once under each worker:
//   suite   SimpleWorkerContext over hl7.fhir.r4.core#4.0.1 with a UCUM service,
//           the worker FHIRPathTests evaluates with (TestingUtilities.java:89)
//   server  HapiWorkerContext over FhirContext.forR4(), the worker FhirPathR4 is
//           built on (FhirPathR4.java:34-36) and so the worker of a live server
// The engine is configured as FhirPathR4 configures it: both `as` rules not
// enforced, no host service, a null application context, and no static check,
// except where the probe says otherwise. The JVM default zone is GMT, which is
// what FHIRPathTests' TimeZone.getTimeZone("UTC+1100") yields.
package dev.frevo.harness;

import java.io.File;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.*;
import java.util.concurrent.*;

import com.google.gson.*;

import ca.uhn.fhir.context.FhirContext;
import org.fhir.ucum.UcumEssenceService;
import org.hl7.fhir.exceptions.FHIRException;
import org.hl7.fhir.exceptions.PathEngineException;
import org.hl7.fhir.r4.context.IWorkerContext;
import org.hl7.fhir.r4.context.SimpleWorkerContext;
import org.hl7.fhir.r4.fhirpath.*;
import org.hl7.fhir.r4.fhirpath.ExpressionNode.CollectionStatus;
import org.hl7.fhir.r4.fhirpath.FHIRPathUtilityClasses.FunctionDetails;
import org.hl7.fhir.r4.formats.JsonParser;
import org.hl7.fhir.r4.formats.XmlParser;
import org.hl7.fhir.r4.hapi.ctx.HapiWorkerContext;
import org.hl7.fhir.r4.model.*;
import org.hl7.fhir.utilities.fhirpath.FHIRPathConstantEvaluationMode;
import org.hl7.fhir.utilities.npm.FilesystemPackageCacheManager;

public class HapiFhirPathDifferential {

  static final long TIMEOUT_SECONDS = 30;

  /** A string with every UTF-16 unit outside printable ASCII written as <U+XXXX>. */
  static String vis(String s) {
    if (s == null) return null;
    StringBuilder b = new StringBuilder();
    for (int i = 0; i < s.length(); i++) {
      char c = s.charAt(i);
      if (c < 0x20 || c > 0x7e) b.append(String.format("<U+%04X>", (int) c));
      else b.append(c);
    }
    return b.toString();
  }

  static String render(Base b) {
    if (b instanceof Quantity) {
      Quantity q = (Quantity) b;
      String v = q.hasValueElement() && q.getValueElement().getValueAsString() != null ? q.getValueElement().getValueAsString() : "";
      return "Quantity:" + v + "|" + nz(q.getUnit()) + "|" + nz(q.getSystem()) + "|" + nz(q.getCode());
    }
    if (b.isPrimitive()) return b.fhirType() + ":" + vis(b.primitiveValue());
    return b.fhirType();
  }

  static String nz(String s) { return s == null ? "" : vis(s); }

  static Base item(String spec) {
    int i = spec.indexOf(':');
    String t = spec.substring(0, i), v = spec.substring(i + 1);
    switch (t) {
      case "string": return new StringType(v);
      case "integer": return new IntegerType(v);
      case "boolean": return new BooleanType(v);
      default: throw new IllegalArgumentException("probe item type " + t);
    }
  }

  /** The probe's host service: answers from the probe's table and records every call. */
  static class Host implements IHostApplicationServices {
    final JsonObject spec;
    final IWorkerContext worker;
    final List<String> calls = new ArrayList<>();

    Host(JsonObject spec, IWorkerContext worker) { this.spec = spec; this.worker = worker; }

    JsonArray arr(String k) { return spec.has(k) ? spec.getAsJsonArray(k) : new JsonArray(); }

    static boolean modeMatches(String want, FHIRPathConstantEvaluationMode m) {
      return want.equals("*") || want.equals(m.name());
    }

    public List<Base> resolveConstant(FHIRPathEngine e, Object a, String name, FHIRPathConstantEvaluationMode mode) {
      calls.add("resolveConstant(" + vis(name) + "," + mode.name() + ")");
      List<Base> r = new ArrayList<>();
      for (JsonElement el : arr("constants")) {
        JsonArray c = el.getAsJsonArray();
        if (c.get(0).getAsString().equals(name) && modeMatches(c.get(1).getAsString(), mode))
          for (JsonElement it : c.get(2).getAsJsonArray()) r.add(item(it.getAsString()));
      }
      return r;
    }

    public TypeDetails resolveConstantType(FHIRPathEngine e, Object a, String name, FHIRPathConstantEvaluationMode mode) {
      calls.add("resolveConstantType(" + vis(name) + "," + mode.name() + ")");
      for (JsonElement el : arr("constantTypes")) {
        JsonArray c = el.getAsJsonArray();
        if (c.get(0).getAsString().equals(name) && modeMatches(c.get(1).getAsString(), mode))
          return new TypeDetails(CollectionStatus.SINGLETON, c.get(2).getAsString());
      }
      return null;
    }

    public boolean log(String argument, List<Base> focus) {
      calls.add("log(" + vis(argument) + "," + focus.size() + ")");
      return spec.has("log") && spec.get("log").getAsBoolean();
    }

    JsonObject fn(String name) {
      for (JsonElement el : arr("functions"))
        if (el.getAsJsonObject().get("name").getAsString().equals(name)) return el.getAsJsonObject();
      return null;
    }

    public FunctionDetails resolveFunction(FHIRPathEngine e, String name) {
      calls.add("resolveFunction(" + vis(name) + ")");
      JsonObject f = fn(name);
      return f == null ? null : new FunctionDetails(name, f.get("min").getAsInt(), f.get("max").getAsInt());
    }

    public TypeDetails checkFunction(FHIRPathEngine e, Object a, String name, TypeDetails focus, List<TypeDetails> params) {
      calls.add("checkFunction(" + vis(name) + "," + params.size() + ")");
      JsonObject f = fn(name);
      if (f == null) throw new FHIRException("unknown function " + name);
      return new TypeDetails(CollectionStatus.SINGLETON, f.get("type").getAsString());
    }

    public List<Base> executeFunction(FHIRPathEngine e, Object a, List<Base> focus, String name, List<List<Base>> params) {
      StringBuilder p = new StringBuilder();
      for (List<Base> l : params) {
        if (p.length() > 0) p.append(";");
        List<String> r = new ArrayList<>();
        for (Base b : l) r.add(render(b));
        p.append(String.join(",", r));
      }
      calls.add("executeFunction(" + vis(name) + ",focus=" + focus.size() + ",params=[" + p + "])");
      List<Base> r = new ArrayList<>();
      JsonObject f = fn(name);
      if (f != null) for (JsonElement it : f.getAsJsonArray("result")) r.add(item(it.getAsString()));
      return r;
    }

    public Base resolveReference(FHIRPathEngine e, Object a, String url, Identifier id, Base ctx) {
      calls.add("resolveReference(" + vis(url) + ")");
      return null;
    }

    public boolean conformsToProfile(FHIRPathEngine e, Object a, Base item, String url) {
      calls.add("conformsToProfile(" + vis(url) + ")");
      if (!spec.has("conformsTo")) throw new FHIRException("the probe's host has no conformsTo answer");
      return spec.get("conformsTo").getAsBoolean();
    }

    public ValueSet resolveValueSet(FHIRPathEngine e, Object a, String url) {
      calls.add("resolveValueSet(" + vis(url) + ")");
      if (spec.has("valueSets") && spec.get("valueSets").getAsString().equals("worker"))
        return worker.fetchResource(ValueSet.class, url);
      return null;
    }

    public boolean paramIsType(String name, int index) {
      calls.add("paramIsType(" + vis(name) + "," + index + ")");
      JsonObject f = fn(name);
      if (f == null || !f.has("typeParams")) return false;
      for (JsonElement i : f.getAsJsonArray("typeParams")) if (i.getAsInt() == index) return true;
      return false;
    }
  }

  static final Map<String, Resource> resources = new HashMap<>();
  static File suiteDir, localDir;

  static Resource load(String ref) throws Exception {
    Resource r = resources.get(ref);
    if (r != null) return r;
    File f = ref.startsWith("suite:") ? new File(suiteDir, ref.substring(6)) : new File(localDir, ref.substring(6));
    byte[] b = Files.readAllBytes(f.toPath());
    r = f.getName().endsWith(".json") ? new JsonParser().parse(b) : new XmlParser().parse(b);
    resources.put(ref, r);
    return r;
  }

  static String str(JsonObject o, String k, String d) { return o.has(k) ? o.get(k).getAsString() : d; }
  static boolean bool(JsonObject o, String k) { return o.has(k) && o.get(k).getAsBoolean(); }

  static JsonObject fail(String stage, Throwable t) {
    JsonObject o = new JsonObject();
    o.addProperty("stage", stage);
    o.addProperty("error", vis(String.valueOf(t.getMessage())));
    o.addProperty("exception", t.getClass().getName());
    return o;
  }

  /** One probe under one worker. */
  static JsonObject run(JsonObject probe, IWorkerContext worker) {
    JsonObject settings = probe.has("settings") ? probe.getAsJsonObject("settings") : new JsonObject();
    FHIRPathEngine fp = new FHIRPathEngine(worker);
    fp.setDoNotEnforceAsCaseSensitive(!bool(settings, "asCaseSensitive"));
    fp.setDoNotEnforceAsSingletonRule(!bool(settings, "asSingleton"));
    fp.setAllowPolymorphicNames(bool(settings, "polymorphic"));
    Host host = probe.has("host") ? new Host(probe.getAsJsonObject("host"), worker) : null;
    fp.setHostServices(host);
    JsonObject out = runStages(probe, fp);
    if (host != null) {
      JsonArray c = new JsonArray();
      for (String s : host.calls) c.add(s);
      out.add("hostCalls", c);
    }
    return out;
  }

  static JsonObject runStages(JsonObject probe, FHIRPathEngine fp) {
    String check = str(probe, "check", "skip");
    Resource res = null;
    if (probe.has("input")) {
      try {
        res = load(probe.get("input").getAsString());
      } catch (Throwable t) {
        return fail("input", t);
      }
    }
    ExpressionNode node;
    try {
      node = fp.parse(probe.get("expr").getAsString());
    } catch (Throwable t) {
      return fail("parse", t);
    }
    if (!check.equals("skip")) {
      try {
        if (res == null) fp.check(null, null, null, node);
        else fp.check(null, "Resource", res.fhirType(), res.fhirType(), node);
      } catch (Throwable t) {
        return fail("check", t);
      }
      if (check.equals("only")) {
        JsonObject o = new JsonObject();
        o.addProperty("stage", "checked");
        return o;
      }
    }
    Object app = bool(probe, "app") ? new Object() : null;
    int repeat = probe.has("repeat") ? probe.get("repeat").getAsInt() : 1;
    JsonArray items = new JsonArray();
    for (int i = 0; i < repeat; i++) {
      if (i > 0) items.add("--");
      List<Base> r;
      try {
        r = app == null ? fp.evaluate(res, node) : fp.evaluate(app, res, res, res, node);
      } catch (Throwable t) {
        return fail("eval", t);
      }
      for (Base b : r) {
        try {
          items.add(render(b));
        } catch (Throwable t) {
          items.add(b.fhirType() + ":<render failed: " + vis(String.valueOf(t)) + ">");
        }
      }
    }
    JsonObject o = new JsonObject();
    o.addProperty("stage", "ok");
    o.add("items", items);
    boolean has = fp.hasLog();
    o.addProperty("hasLog", has);
    o.addProperty("log", vis(fp.takeLog()));
    return o;
  }

  static JsonObject runWithTimeout(JsonObject probe, IWorkerContext worker) {
    ExecutorService ex = Executors.newSingleThreadExecutor(r -> {
      Thread t = new Thread(null, r, "probe", 512L * 1024 * 1024);
      t.setDaemon(true);
      return t;
    });
    try {
      return ex.submit(() -> run(probe, worker)).get(TIMEOUT_SECONDS, TimeUnit.SECONDS);
    } catch (TimeoutException t) {
      JsonObject o = new JsonObject();
      o.addProperty("stage", "timeout");
      o.addProperty("error", "no answer within " + TIMEOUT_SECONDS + " s");
      return o;
    } catch (ExecutionException t) {
      return fail("harness", t.getCause());
    } catch (InterruptedException t) {
      throw new RuntimeException(t);
    } finally {
      ex.shutdownNow();
    }
  }

  public static void main(String[] args) throws Exception {
    TimeZone.setDefault(TimeZone.getTimeZone("UTC+1100"));
    JsonArray probes = JsonParser_parse(new File(args[0]));
    suiteDir = new File(args[1]);
    localDir = new File(args[2]);
    String ucum = args[3];
    File out = new File(args[4]);

    SimpleWorkerContext suite = SimpleWorkerContext.fromPackage(
        new FilesystemPackageCacheManager.Builder().build().loadPackage("hl7.fhir.r4.core", "4.0.1"));
    suite.setUcumService(new UcumEssenceService(ucum));
    suite.setExpansionProfile(new Parameters());
    FhirContext ctx = FhirContext.forR4();
    IWorkerContext server = new HapiWorkerContext(ctx, ctx.getValidationSupport());

    JsonObject results = new JsonObject();
    for (JsonElement el : probes) {
      JsonObject p = el.getAsJsonObject();
      JsonObject r = new JsonObject();
      r.add("suite", runWithTimeout(p, suite));
      r.add("server", runWithTimeout(p, server));
      results.add(p.get("id").getAsString(), r);
    }
    JsonObject doc = new JsonObject();
    doc.addProperty("engine", "ca.uhn.hapi.fhir:org.hl7.fhir.r4:6.9.12 FHIRPathEngine, in-process");
    doc.addProperty("server_worker", "ca.uhn.hapi.fhir:hapi-fhir-structures-r4:8.12.0 HapiWorkerContext(FhirContext.forR4())");
    doc.addProperty("suite_worker", "SimpleWorkerContext.fromPackage(hl7.fhir.r4.core#4.0.1) + UcumEssenceService");
    doc.addProperty("ucum", ucumVersion());
    doc.addProperty("jvm", System.getProperty("java.vendor") + " " + System.getProperty("java.version"));
    doc.add("results", results);
    try (PrintWriter w = new PrintWriter(out, StandardCharsets.UTF_8)) {
      // "\n", not println: the committed file is the same on every platform.
      w.print(new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().create().toJson(doc) + "\n");
    }
    System.out.println("probes: " + probes.size());
  }

  static String ucumVersion() {
    String v = UcumEssenceService.class.getPackage().getImplementationVersion();
    if (v != null) return "org.fhir:ucum " + v;
    String loc = String.valueOf(UcumEssenceService.class.getProtectionDomain().getCodeSource().getLocation());
    return loc.substring(loc.lastIndexOf('/') + 1);
  }

  static JsonArray JsonParser_parse(File f) throws Exception {
    return com.google.gson.JsonParser.parseString(new String(Files.readAllBytes(f.toPath()), StandardCharsets.UTF_8)).getAsJsonArray();
  }
}
