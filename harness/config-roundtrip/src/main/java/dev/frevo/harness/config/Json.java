package dev.frevo.harness.config;

import java.util.List;
import java.util.Map;

/** A minimal, dependency-free JSON writer: maps, lists, strings, longs, booleans, null. */
final class Json {
	private Json() {}

	static String write(Object v) {
		StringBuilder sb = new StringBuilder();
		write(sb, v, 0);
		sb.append('\n');
		return sb.toString();
	}

	private static void write(StringBuilder sb, Object v, int depth) {
		if (v == null) {
			sb.append("null");
		} else if (v instanceof Boolean || v instanceof Long || v instanceof Integer) {
			sb.append(v);
		} else if (v instanceof String s) {
			string(sb, s);
		} else if (v instanceof Map<?, ?> m) {
			if (m.isEmpty()) {
				sb.append("{}");
				return;
			}
			sb.append("{\n");
			int i = 0;
			for (Map.Entry<?, ?> e : m.entrySet()) {
				indent(sb, depth + 1);
				string(sb, String.valueOf(e.getKey()));
				sb.append(": ");
				write(sb, e.getValue(), depth + 1);
				if (++i < m.size()) sb.append(',');
				sb.append('\n');
			}
			indent(sb, depth);
			sb.append('}');
		} else if (v instanceof List<?> l) {
			if (l.isEmpty()) {
				sb.append("[]");
				return;
			}
			sb.append('[');
			for (int i = 0; i < l.size(); i++) {
				if (i > 0) sb.append(", ");
				write(sb, l.get(i), depth + 1);
			}
			sb.append(']');
		} else {
			throw new IllegalArgumentException("not a JSON value: " + v.getClass());
		}
	}

	private static void indent(StringBuilder sb, int depth) {
		for (int i = 0; i < depth; i++) sb.append("  ");
	}

	private static void string(StringBuilder sb, String s) {
		sb.append('"');
		for (int i = 0; i < s.length(); i++) {
			char c = s.charAt(i);
			switch (c) {
				case '"' -> sb.append("\\\"");
				case '\\' -> sb.append("\\\\");
				case '\n' -> sb.append("\\n");
				case '\r' -> sb.append("\\r");
				case '\t' -> sb.append("\\t");
				default -> {
					if (c < 0x20) sb.append(String.format("\\u%04x", (int) c));
					else sb.append(c);
				}
			}
		}
		sb.append('"');
	}
}
