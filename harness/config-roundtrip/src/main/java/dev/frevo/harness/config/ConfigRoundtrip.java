package dev.frevo.harness.config;

import ca.uhn.fhir.jpa.starter.AppProperties;
import ca.uhn.fhir.jpa.starter.cdshooks.CdsHooksProperties;
import ca.uhn.fhir.jpa.starter.cr.CareGapsProperties;
import ca.uhn.fhir.jpa.starter.cr.CqlCompilerProperties;
import ca.uhn.fhir.jpa.starter.cr.CqlData;
import ca.uhn.fhir.jpa.starter.cr.CqlProperties;
import ca.uhn.fhir.jpa.starter.cr.CqlRuntimeProperties;
import ca.uhn.fhir.jpa.starter.cr.CqlTerminologyProperties;
import ca.uhn.fhir.jpa.starter.cr.CrProperties;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.WebApplicationType;
import org.springframework.boot.context.properties.EnableConfigurationProperties;
import org.springframework.boot.context.properties.bind.DataObjectPropertyName;
import org.springframework.context.ConfigurableApplicationContext;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.core.env.ConfigurableEnvironment;

import java.beans.Introspector;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collection;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;

/**
 * The HAPI side of the {@code config-roundtrip} oracle (design.md section 7).
 *
 * <p>Started inside the pinned HAPI image by the image's own launcher
 * ({@code PropertiesLauncher}, {@code loader.main} from {@code /app/loader.properties}), so the
 * class path, its order, {@code /app/extra-classes/}, the packaged {@code application*.yaml}
 * files and {@code logback.xml} are HAPI's. It binds the starter's nine properties classes
 * exactly as the starter declares them (the classes come from the image's
 * {@code WEB-INF/classes}, which precede {@code /app/extra-classes} on the class path),
 * evaluates the starter's own condition classes as gates ({@link GateEvaluator}), reads the
 * thirteen named keys and writes the effective-configuration document of config.dump.001 to
 * {@code /tmp/config-roundtrip/document.json}.
 *
 * <p>A failing configuration fails inside {@link SpringApplication#run}, which prints HAPI's
 * failure output on standard output and exits {@code 1}; this class adds nothing to it.
 */
@Configuration(proxyBeanMethods = false)
@EnableConfigurationProperties({
	AppProperties.class,
	CdsHooksProperties.class,
	CrProperties.class,
	CareGapsProperties.class,
	CqlProperties.class,
	CqlCompilerProperties.class,
	CqlRuntimeProperties.class,
	CqlTerminologyProperties.class,
	CqlData.class
})
public class ConfigRoundtrip {

	static final Path OUT = Path.of("/tmp/config-roundtrip");

	/** The thirteen reads of config.read.001, in the spec's order. */
	static final String[] READS = {
		"hapi.fhir.fhir_version",
		"hapi.fhir.cr.enabled",
		"hapi.fhir.cdshooks.enabled",
		"hapi.fhir.mdm_enabled",
		"hapi.fhir.ips_enabled",
		"hapi.fhir.ig_runtime_upload_enabled",
		"hapi.fhir.binary_storage_mode",
		"hapi.fhir.custom_content_path",
		"hapi.fhir.app_content_path",
		"hapi.fhir.enable_repository_validating_interceptor",
		"hapi.fhir.custom-bean-packages",
		"spring.ai.mcp.server.enabled",
		"scheduling_disabled"
	};

	@Bean
	static GateEvaluator gateEvaluator() {
		return new GateEvaluator();
	}

	public static void main(String[] args) throws Exception {
		SpringApplication app = new SpringApplication(ConfigRoundtrip.class);
		app.setWebApplicationType(WebApplicationType.NONE);
		try (ConfigurableApplicationContext ctx = app.run(args)) {
			ConfigurableEnvironment env = ctx.getEnvironment();

			Map<String, Object> doc = new LinkedHashMap<>();

			Map<String, Object> profiles = new LinkedHashMap<>();
			profiles.put("active", Arrays.asList(env.getActiveProfiles()));
			profiles.put("default", Arrays.asList(env.getDefaultProfiles()));
			doc.put("profiles", profiles);

			Map<String, Object> props = new LinkedHashMap<>();
			props.put("hapi.fhir", bean(ctx.getBean(AppProperties.class), "hapi.fhir"));
			props.put("hapi.fhir.cdshooks", bean(ctx.getBean(CdsHooksProperties.class), "hapi.fhir.cdshooks"));
			props.put("hapi.fhir.cr", bean(ctx.getBean(CrProperties.class), "hapi.fhir.cr"));
			props.put("hapi.fhir.cr.caregaps", bean(ctx.getBean(CareGapsProperties.class), "hapi.fhir.cr.caregaps"));
			props.put("hapi.fhir.cr.cql", bean(ctx.getBean(CqlProperties.class), "hapi.fhir.cr.cql"));
			props.put("hapi.fhir.cr.cql.compiler", bean(ctx.getBean(CqlCompilerProperties.class), "hapi.fhir.cr.cql.compiler"));
			props.put("hapi.fhir.cr.cql.runtime", bean(ctx.getBean(CqlRuntimeProperties.class), "hapi.fhir.cr.cql.runtime"));
			props.put("hapi.fhir.cr.cql.terminology", bean(ctx.getBean(CqlTerminologyProperties.class), "hapi.fhir.cr.cql.terminology"));
			props.put("hapi.fhir.cr.cql.data", bean(ctx.getBean(CqlData.class), "hapi.fhir.cr.cql.data"));
			doc.put("properties", props);

			Map<String, Object> reads = new LinkedHashMap<>();
			for (String r : READS) {
				reads.put(r, env.getProperty(r));
			}
			doc.put("reads", reads);
			doc.put("gates", new LinkedHashMap<>(GateEvaluator.GATES));

			Map<String, Object> sys = new TreeMap<>();
			for (String n : System.getProperties().stringPropertyNames()) {
				sys.put(n, System.getProperty(n));
			}

			Files.createDirectories(OUT);
			Files.writeString(OUT.resolve("document.json"), Json.write(doc), StandardCharsets.UTF_8);
			Files.writeString(OUT.resolve("system-properties.json"), Json.write(sys), StandardCharsets.UTF_8);
		}
	}

	// ------------------------------------------------------------------------------------
	// The dumper of config.dump.002: every bean property with a getter, by its canonical
	// (dashed) name, the way Spring Boot's JavaBeanBinder names it.
	// ------------------------------------------------------------------------------------

	/** Getters that are not bean properties of the document (design.md section 8). */
	static boolean excluded(Class<?> owner, String property) {
		// Derived getters returning a new object on every call, with no setter
		// (cr/CqlData.java:39, cr/CqlProperties.java:70); config.unknown.008.
		if (owner == CqlData.class && property.equals("retrieveSettings")) return true;
		if (owner == CqlProperties.class && property.equals("retrieveSettings")) return true;
		return false;
	}

	/**
	 * The name the document reports a getter under, when it is not the getter's own name.
	 * {@code AppProperties.getRemoteTerminologyServicesMap()} is the getter of the property
	 * that binds as {@code remote_terminology_service} (its setter); the document reports it
	 * under the bound name (keys.md section 1).
	 */
	static String rename(Class<?> owner, String property) {
		if (owner == AppProperties.class && property.equals("remoteTerminologyServicesMap")) {
			return "remote_terminology_service";
		}
		return property;
	}

	static Map<String, Object> bean(Object o, String path) throws Exception {
		// CqlTerminologyProperties has setters only; its getTerminologySettings() wraps the
		// four modes, which the document flattens into the bean (design.md section 8).
		if (o instanceof CqlTerminologyProperties t) {
			return bean(t.getTerminologySettings(), path);
		}
		Map<String, Object> out = new TreeMap<>();
		Map<String, Method> getters = new TreeMap<>();
		Method[] methods = o.getClass().getMethods();
		// JavaBeanBinder prefers a get- getter over an is- getter of the same property
		// (AppProperties.Partitioning has both for request_tenant_partitioning_mode).
		Arrays.sort(methods, java.util.Comparator.comparing((Method m) -> m.getName().startsWith("is") ? 0 : 1)
				.thenComparing(Method::getName));
		for (Method m : methods) {
			if (Modifier.isStatic(m.getModifiers()) || m.getParameterCount() != 0) continue;
			if (m.getDeclaringClass() == Object.class || m.getDeclaringClass() == Class.class) continue;
			String n = m.getName();
			String prop;
			if (n.startsWith("get") && n.length() > 3) prop = Introspector.decapitalize(n.substring(3));
			else if (n.startsWith("is") && n.length() > 2) prop = Introspector.decapitalize(n.substring(2));
			else continue;
			if (m.getReturnType() == void.class) continue;
			if (excluded(o.getClass(), prop)) continue;
			String name = dashed(rename(o.getClass(), prop));
			String childPath = path + "." + name;
			Object v = m.invoke(o);
			Object rendered = value(v, childPath);
			if (childPath.equals("hapi.fhir.subscription.email.password") && rendered instanceof String s && !s.isEmpty()) {
				rendered = "******"; // config.dump.005
			}
			Method previous = getters.put(name, m);
			if (previous != null && !previous.getName().startsWith("is")) {
				throw new IllegalStateException("two get- getters report " + childPath);
			}
			out.put(name, rendered);
		}
		return out;
	}

	static Object value(Object v, String path) throws Exception {
		if (v == null) return null;
		if (v instanceof Boolean) return v;
		if (v instanceof CharSequence) return v.toString();
		if (v instanceof Integer || v instanceof Long || v instanceof Short || v instanceof Byte) return ((Number) v).longValue();
		if (v instanceof Number) throw new IllegalStateException("non-integral number at " + path + ": " + v.getClass());
		if (v instanceof Enum<?> e) return e.name();
		if (v instanceof byte[] b) {
			List<Object> l = new ArrayList<>();
			for (byte x : b) l.add((long) x);
			return l;
		}
		if (v instanceof Set<?> s) {
			List<String> l = new ArrayList<>();
			for (Object x : s) {
				Object r = value(x, path + "[]");
				if (!(r instanceof String str)) throw new IllegalStateException("non-text set element at " + path);
				l.add(str);
			}
			l.sort(ConfigRoundtrip::codePointCompare);
			return new ArrayList<Object>(l);
		}
		if (v instanceof Collection<?> c) {
			List<Object> l = new ArrayList<>();
			int i = 0;
			for (Object x : c) l.add(value(x, path + "[" + (i++) + "]"));
			return l;
		}
		if (v instanceof Map<?, ?> m) {
			Map<String, Object> out = new TreeMap<>();
			for (Map.Entry<?, ?> e : m.entrySet()) {
				out.put(String.valueOf(e.getKey()), value(e.getValue(), path + "." + e.getKey()));
			}
			return out;
		}
		String pkg = v.getClass().getPackageName();
		if (pkg.startsWith("ca.uhn.") || pkg.startsWith("org.opencds.") || pkg.startsWith("org.cqframework.")) {
			return bean(v, path);
		}
		throw new IllegalStateException("unhandled type at " + path + ": " + v.getClass().getName());
	}

	/** The canonical name, by Spring Boot's own rule. */
	static String dashed(String name) {
		return DataObjectPropertyName.toDashedForm(name);
	}

	static int codePointCompare(String a, String b) {
		int i = 0, j = 0;
		while (i < a.length() && j < b.length()) {
			int ca = a.codePointAt(i), cb = b.codePointAt(j);
			if (ca != cb) return Integer.compare(ca, cb);
			i += Character.charCount(ca);
			j += Character.charCount(cb);
		}
		return Integer.compare(a.length() - i, b.length() - j);
	}
}
