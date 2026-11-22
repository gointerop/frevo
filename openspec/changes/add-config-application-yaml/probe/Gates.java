package probe;

import ca.uhn.fhir.jpa.starter.annotations.*;
import ca.uhn.fhir.jpa.starter.cdshooks.CdsHooksConfigCondition;
import ca.uhn.fhir.jpa.starter.common.FhirTesterConfigCondition;
import ca.uhn.fhir.jpa.starter.common.OnPartitionModeEnabled;
import ca.uhn.fhir.jpa.starter.common.validation.OnRemoteTerminologyPresent;
import ca.uhn.fhir.jpa.starter.cr.CrConfigCondition;
import ca.uhn.fhir.jpa.starter.elastic.ElasticConfigCondition;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Conditional;
import org.springframework.context.annotation.Configuration;

/** One marker bean per starter gate, guarded by the starter's own condition. */
@Configuration
public class Gates {
	public static class Marker {
		public final String name;
		Marker(String n) { name = n; }
	}

	@Bean @Conditional(OnDSTU2Condition.class) Marker fhirDstu2() { return new Marker("fhir-dstu2"); }
	@Bean @Conditional(OnDSTU3Condition.class) Marker fhirDstu3() { return new Marker("fhir-dstu3"); }
	@Bean @Conditional(OnR4Condition.class) Marker fhirR4() { return new Marker("fhir-r4"); }
	@Bean @Conditional(OnR4BCondition.class) Marker fhirR4b() { return new Marker("fhir-r4b"); }
	@Bean @Conditional(OnR5Condition.class) Marker fhirR5() { return new Marker("fhir-r5"); }
	@Bean @Conditional(OnEitherVersion.class) Marker fhirAny() { return new Marker("fhir-any"); }
	@Bean @Conditional(CrConfigCondition.class) Marker cr() { return new Marker("cr"); }
	@Bean @Conditional(CdsHooksConfigCondition.class) Marker cdshooks() { return new Marker("cdshooks"); }
	@Bean @ConditionalOnProperty(prefix = "hapi.fhir", name = "mdm_enabled") Marker mdm() { return new Marker("mdm"); }
	@Bean @ConditionalOnProperty(name = "hapi.fhir.ips_enabled", havingValue = "true") Marker ips() { return new Marker("ips"); }
	@Bean @ConditionalOnProperty(name = "hapi.fhir.ig_runtime_upload_enabled", havingValue = "true") Marker igUpload() { return new Marker("ig-runtime-upload"); }
	@Bean @ConditionalOnProperty(prefix = "hapi.fhir", name = "enable_repository_validating_interceptor", havingValue = "true") Marker rvi() { return new Marker("repository-validating-interceptor"); }
	@Bean @ConditionalOnProperty(prefix = "hapi.fhir", name = "binary_storage_mode", havingValue = "FILESYSTEM") Marker binFs() { return new Marker("binary-storage-filesystem"); }
	@Bean @ConditionalOnProperty(prefix = "hapi.fhir", name = "binary_storage_mode", havingValue = "DATABASE", matchIfMissing = true) Marker binDb() { return new Marker("binary-storage-database"); }
	@Bean @ConditionalOnProperty(prefix = "hapi.fhir", name = "custom_content_path") Marker customContent() { return new Marker("custom-content"); }
	@Bean @ConditionalOnProperty(prefix = "hapi.fhir", name = "app_content_path") Marker appContent() { return new Marker("app-content"); }
	@Bean @ConditionalOnProperty(prefix = "spring.ai.mcp.server", name = {"enabled"}, havingValue = "true") Marker mcp() { return new Marker("mcp"); }
	@Bean @ConditionalOnProperty(prefix = "hapi.fhir.cdshooks", name = {"enabled"}, havingValue = "true") Marker mcpCds() { return new Marker("mcp-cdshooks"); }
	@Bean @Conditional(OnCorsPresent.class) Marker cors() { return new Marker("cors"); }
	@Bean @Conditional(OnImplementationGuidesPresent.class) Marker igs() { return new Marker("implementation-guides"); }
	@Bean @Conditional(OnPartitionModeEnabled.class) Marker part() { return new Marker("partitioning"); }
	@Bean @Conditional(OnRemoteTerminologyPresent.class) Marker remote() { return new Marker("remote-terminology"); }
	@Bean @Conditional(FhirTesterConfigCondition.class) Marker tester() { return new Marker("tester"); }
	@Bean @Conditional(ElasticConfigCondition.class) Marker elastic() { return new Marker("elasticsearch"); }
}
