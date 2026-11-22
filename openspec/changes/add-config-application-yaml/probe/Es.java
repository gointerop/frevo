package probe;

import org.springframework.boot.autoconfigure.elasticsearch.ElasticsearchProperties;
import org.springframework.boot.context.properties.bind.Binder;
import org.springframework.boot.context.properties.source.MapConfigurationPropertySource;

import java.util.Map;

public class Es {
	public static void main(String[] a) {
		ElasticsearchProperties d = new ElasticsearchProperties();
		System.out.println("defaults uris=" + d.getUris() + " username=" + d.getUsername() + " password=" + d.getPassword()
				+ " connectionTimeout=" + d.getConnectionTimeout() + " socketTimeout=" + d.getSocketTimeout()
				+ " socketKeepAlive=" + d.isSocketKeepAlive() + " pathPrefix=" + d.getPathPrefix()
				+ " sniffer.interval=" + d.getRestclient().getSniffer().getInterval()
				+ " sniffer.delayAfterFailure=" + d.getRestclient().getSniffer().getDelayAfterFailure()
				+ " ssl.bundle=" + d.getRestclient().getSsl().getBundle());
		for (String v : new String[] {"5s", "10", "PT5S", "1m", "2h", "3d", "500ms", "7us", "9ns", "-5s", "5S", "5 s", "1.5s", "pt5s", "abc"}) {
			try {
				ElasticsearchProperties p = new Binder(new MapConfigurationPropertySource(Map.of("spring.elasticsearch.connection-timeout", v)))
						.bind("spring.elasticsearch", ElasticsearchProperties.class).get();
				System.out.println("[" + v + "] -> " + p.getConnectionTimeout());
			} catch (Exception e) {
				Throwable c = e; while (c.getCause() != null) c = c.getCause();
				System.out.println("[" + v + "] -> FAIL " + c.getMessage());
			}
		}
	}
}
