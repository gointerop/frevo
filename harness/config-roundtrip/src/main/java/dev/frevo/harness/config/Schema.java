package dev.frevo.harness.config;

import ca.uhn.fhir.jpa.starter.AppProperties;
import ca.uhn.fhir.jpa.starter.cdshooks.CdsHooksProperties;
import ca.uhn.fhir.jpa.starter.cr.CrProperties;
import org.springframework.boot.context.properties.bind.DataObjectPropertyName;

import java.beans.Introspector;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.util.Arrays;
import java.util.Collection;
import java.util.Comparator;
import java.util.Map;
import java.util.TreeSet;

/**
 * Prints the bean-property schema of the starter's properties classes as the image's
 * classes declare it: one line per property, {@code path<TAB>type<TAB>setter?<TAB>enum
 * constants}. Used by {@code make_cases.py} to check that {@code all-keys.yaml} sets every
 * bound property and uses valid, non-default enum constants (config.key.005-.007). Run with
 * {@code run.py schema}; not part of any case.
 */
public final class Schema {
	public static void main(String[] args) {
		walk("hapi.fhir", AppProperties.class, 0);
		walk("hapi.fhir.cdshooks", CdsHooksProperties.class, 0);
		walk("hapi.fhir.cr", CrProperties.class, 0);
	}

	static void walk(String path, Type t, int depth) {
		Class<?> c = raw(t);
		if (depth > 6) return;
		TreeSet<String> lines = new TreeSet<>();
		Method[] ms = c.getMethods();
		Arrays.sort(ms, Comparator.comparing(Method::getName));
		for (Method m : ms) {
			if (Modifier.isStatic(m.getModifiers()) || m.getParameterCount() != 0) continue;
			if (m.getDeclaringClass() == Object.class) continue;
			String n = m.getName();
			String prop;
			if (n.startsWith("get") && n.length() > 3) prop = n.substring(3);
			else if (n.startsWith("is") && n.length() > 2) prop = n.substring(2);
			else continue;
			prop = Introspector.decapitalize(prop);
			boolean setter = false;
			for (Method s : ms) {
				if (s.getParameterCount() == 1 && s.getName().equals("set" + capitalize(prop))) setter = true;
			}
			Type rt = m.getGenericReturnType();
			String p = path + "." + DataObjectPropertyName.toDashedForm(prop);
			Class<?> rc = raw(rt);
			String consts = "";
			if (rc.isEnum()) consts = Arrays.toString(rc.getEnumConstants());
			Type elem = null;
			if (Collection.class.isAssignableFrom(rc) && rt instanceof ParameterizedType pt) elem = pt.getActualTypeArguments()[0];
			if (Map.class.isAssignableFrom(rc) && rt instanceof ParameterizedType pt) elem = pt.getActualTypeArguments()[1];
			if (elem != null && raw(elem).isEnum()) consts = Arrays.toString(raw(elem).getEnumConstants());
			System.out.println(p + "\t" + rt.getTypeName() + "\t" + (setter ? "setter" : "no-setter") + "\t" + consts);
			String pkg = rc.getPackageName();
			if (!rc.isEnum() && (pkg.startsWith("ca.uhn.") || pkg.startsWith("org.opencds."))) walk(p, rt, depth + 1);
			if (elem != null && !raw(elem).isEnum() && raw(elem).getPackageName().startsWith("ca.uhn.")) walk(p + ".<key>", elem, depth + 1);
		}
	}

	static Class<?> raw(Type t) {
		if (t instanceof Class<?> c) return c;
		if (t instanceof ParameterizedType p) return (Class<?>) p.getRawType();
		return Object.class;
	}

	static String capitalize(String s) {
		return s.isEmpty() ? s : Character.toUpperCase(s.charAt(0)) + s.substring(1);
	}
}
