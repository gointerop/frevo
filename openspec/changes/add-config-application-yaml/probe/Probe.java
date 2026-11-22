package probe;

import ca.uhn.fhir.jpa.starter.AppProperties;
import ca.uhn.fhir.jpa.starter.cdshooks.CdsHooksProperties;
import ca.uhn.fhir.jpa.starter.cr.*;
import org.springframework.boot.Banner;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.WebApplicationType;
import org.springframework.boot.context.properties.EnableConfigurationProperties;
import org.springframework.context.ConfigurableApplicationContext;
import org.springframework.context.annotation.Configuration;
import org.springframework.core.env.ConfigurableEnvironment;

import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.*;

@Configuration
@EnableConfigurationProperties({
	AppProperties.class,
	CdsHooksProperties.class,
	CrProperties.class,
	CqlProperties.class,
	CqlCompilerProperties.class,
	CqlRuntimeProperties.class,
	CqlTerminologyProperties.class,
	CqlData.class,
	CareGapsProperties.class
})
public class Probe {
	public static void main(String[] args) {
		SpringApplication app = new SpringApplication(Probe.class);
		app.setWebApplicationType(WebApplicationType.NONE);
		app.setBannerMode(Banner.Mode.OFF);
		app.setLogStartupInfo(false);
		String filter = System.getenv("PROBE_FILTER");
		try (ConfigurableApplicationContext ctx = app.run(args)) {
			List<String> out = new ArrayList<>();
			dump(out, "AppProperties", ctx.getBean(AppProperties.class), 0);
			dump(out, "CdsHooks", ctx.getBean(CdsHooksProperties.class), 0);
			dump(out, "Cr", ctx.getBean(CrProperties.class), 0);
			dump(out, "Cql", ctx.getBean(CqlProperties.class), 0);
			dump(out, "CqlCompiler", ctx.getBean(CqlCompilerProperties.class), 0);
			dump(out, "CqlRuntime", ctx.getBean(CqlRuntimeProperties.class), 0);
			dump(out, "CqlTerm", ctx.getBean(CqlTerminologyProperties.class), 0);
			dump(out, "CqlData", ctx.getBean(CqlData.class), 0);
			dump(out, "CareGaps", ctx.getBean(CareGapsProperties.class), 0);
			ConfigurableEnvironment env = ctx.getEnvironment();
			for (String k : new String[] {"hapi.fhir.fhir_version", "hapi.fhir.cr.enabled", "hapi.fhir.cdshooks.enabled",
					"hapi.fhir.mdm_enabled", "hapi.fhir.ips_enabled", "hapi.fhir.ig_runtime_upload_enabled",
					"hapi.fhir.binary_storage_mode", "hapi.fhir.custom_content_path", "hapi.fhir.app_content_path",
					"hapi.fhir.enable_repository_validating_interceptor", "hapi.fhir.search-coord-core-pool-size",
					"probe.x"}) {
				out.add("ENV[" + k + "] = " + env.getProperty(k));
			}
			out.add("PROFILES = " + Arrays.toString(env.getActiveProfiles()));
			for (String l : out) {
				if (filter == null || l.matches(".*(" + filter + ").*")) System.out.println(l);
			}
		}
	}

	static boolean leaf(Object v) {
		return v == null || v instanceof String || v instanceof Number || v instanceof Boolean || v instanceof Enum;
	}

	static String fmt(Object v) {
		if (v == null) return "null";
		if (v instanceof String s) return "\"" + s + "\"";
		if (v instanceof Enum<?> e) return e.getDeclaringClass().getSimpleName() + "." + e.name();
		return v.getClass().getSimpleName() + ":" + v;
	}

	static void dump(List<String> out, String path, Object o, int depth) {
		if (depth > 6) return;
		if (leaf(o)) { out.add(path + " = " + fmt(o)); return; }
		if (o instanceof Collection<?> c) {
			out.add(path + " = " + c.getClass().getSimpleName() + "(size " + c.size() + ")");
			int i = 0;
			for (Object e : c) dump(out, path + "[" + (i++) + "]", e, depth + 1);
			return;
		}
		if (o instanceof Map<?, ?> m) {
			out.add(path + " = " + m.getClass().getSimpleName() + "(size " + m.size() + ")");
			for (Map.Entry<?, ?> e : m.entrySet()) dump(out, path + "{" + e.getKey() + "}", e.getValue(), depth + 1);
			return;
		}
		if (o instanceof byte[]) { out.add(path + " = bytes"); return; }
		Method[] ms = o.getClass().getMethods();
		Arrays.sort(ms, Comparator.comparing(Method::getName));
		for (Method m : ms) {
			if (m.getParameterCount() != 0 || Modifier.isStatic(m.getModifiers())) continue;
			if (m.getDeclaringClass() == Object.class) continue;
			String n = m.getName();
			if (!(n.startsWith("get") || n.startsWith("is"))) continue;
			if (n.equals("getClass")) continue;
			try {
				Object v = m.invoke(o);
				dump(out, path + "." + n, v, depth + 1);
			} catch (Exception ex) {
				out.add(path + "." + n + " = THREW " + ex.getCause());
			}
		}
	}
}
