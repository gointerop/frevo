package dev.frevo.harness.config;

import org.springframework.beans.BeanUtils;
import org.springframework.beans.factory.BeanClassLoaderAware;
import org.springframework.beans.factory.config.ConfigurableListableBeanFactory;
import org.springframework.beans.factory.support.BeanDefinitionRegistry;
import org.springframework.beans.factory.support.BeanDefinitionRegistryPostProcessor;
import org.springframework.context.EnvironmentAware;
import org.springframework.context.ResourceLoaderAware;
import org.springframework.context.annotation.Condition;
import org.springframework.context.annotation.ConditionContext;
import org.springframework.context.annotation.Conditional;
import org.springframework.core.annotation.AnnotationAwareOrderComparator;
import org.springframework.core.env.Environment;
import org.springframework.core.io.ResourceLoader;
import org.springframework.core.type.AnnotatedTypeMetadata;
import org.springframework.core.type.AnnotationMetadata;
import org.springframework.core.type.MethodMetadata;
import org.springframework.util.ClassUtils;
import org.springframework.util.MultiValueMap;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * The twenty-five gates of config.gate.001, each evaluated by calling the starter's own
 * condition classes on the annotation metadata of the starter class (or {@code @Bean} method)
 * that carries them, the way Spring's {@code ConditionEvaluator.shouldSkip} does during
 * configuration-class parsing. Nothing of the starter's configuration is imported or
 * instantiated: only the conditions run.
 *
 * <p>Evaluated in {@link #postProcessBeanDefinitionRegistry}, i.e. during context refresh
 * before any bean is created, which is where HAPI evaluates them. A condition that throws
 * (HAPI's {@code NullPointerException} when no version is set, a strict placeholder, a
 * binding failure inside {@code OnCorsPresent}) fails the refresh exactly as it does in HAPI,
 * and the exception carries the starter class name ({@code Error processing condition on
 * ca.uhn.fhir.jpa.starter.web.WebAppFilesConfigurer}, config.placeholder.011).
 *
 * <p>Order: the version gates first, then the rest in config.gate.001's order. The version
 * gates are plain {@code Condition}s, so when one throws no condition evaluation report exists
 * yet and Spring prints no {@code ConditionEvaluationReportLogger} line (config.error.008).
 */
public class GateEvaluator
		implements BeanDefinitionRegistryPostProcessor, EnvironmentAware, ResourceLoaderAware, BeanClassLoaderAware {

	static final Map<String, Boolean> GATES = new LinkedHashMap<>();

	private static final String S = "ca.uhn.fhir.jpa.starter.";
	private static final String ON_PROPERTY = "org.springframework.boot.autoconfigure.condition.OnPropertyCondition";

	/** name, starter class, {@code @Bean} method or null, condition class to keep or null for all. */
	private static final String[][] GATE_SITES = {
		{"fhir-dstu2", S + "common.FhirServerConfigDstu2", null, null},
		{"fhir-dstu3", S + "common.FhirServerConfigDstu3", null, null},
		{"fhir-r4", S + "common.FhirServerConfigR4", null, null},
		{"fhir-r4b", S + "common.FhirServerConfigR4B", null, null},
		{"fhir-r5", S + "common.FhirServerConfigR5", null, null},
		{"fhir-any", S + "Application", "hapiServletRegistration", null},
		{"cr", S + "cr.CrCommonConfig", null, null},
		{"cdshooks", S + "cdshooks.StarterCdsHooksConfig", null, S + "cdshooks.CdsHooksConfigCondition"},
		{"mdm", S + "mdm.MdmConfig", null, null},
		{"ips", S + "ips.StarterIpsConfig", null, null},
		{"ig-runtime-upload", S + "ig.ImplementationGuideR4OperationProvider", null, ON_PROPERTY},
		{"repository-validating-interceptor", S + "common.StarterJpaConfig", "repositoryValidatingInterceptor", null},
		{"binary-storage-filesystem", S + "common.FhirServerConfigCommon", "filesystemBinaryStorageSvc", null},
		{"binary-storage-database", S + "common.FhirServerConfigCommon", "databaseBinaryStorageSvc", null},
		{"custom-content", S + "web.CustomContentFilesConfigurer", null, null},
		{"app-content", S + "web.WebAppFilesConfigurer", null, null},
		{"mcp", S + "mcp.McpServerConfig", null, null},
		{"mcp-cdshooks", S + "mcp.McpServerConfig", "mcpCdsBridge", null},
		{"cors", S + "common.StarterJpaConfig", "corsInterceptor", null},
		{"implementation-guides", S + "common.StarterJpaConfig", "packageInstaller", null},
		{"partitioning", S + "common.PartitionModeConfigurer", null, null},
		{"remote-terminology", S + "terminology.TerminologyConfig", null, null},
		{"tester", S + "common.FhirTesterConfig", null, null},
		{"elasticsearch", S + "elastic.ElasticsearchConfig", null, null},
	};

	private Environment environment;
	private ResourceLoader resourceLoader;
	private ClassLoader classLoader;

	@Override
	public void setEnvironment(Environment environment) {
		this.environment = environment;
	}

	@Override
	public void setResourceLoader(ResourceLoader resourceLoader) {
		this.resourceLoader = resourceLoader;
	}

	@Override
	public void setBeanClassLoader(ClassLoader classLoader) {
		this.classLoader = classLoader;
	}

	@Override
	public void postProcessBeanDefinitionRegistry(BeanDefinitionRegistry registry) {
		ConditionContext context = new Context(registry, environment, resourceLoader, classLoader);
		for (String[] site : GATE_SITES) {
			GATES.put(site[0], evaluate(context, site[1], site[2], site[3]));
		}
		// HAPI's own test, not a Spring condition: BaseSchedulerServiceImpl.java:319,
		// SubscriptionMatcherInterceptorConfig.java:47 ("true".equals(...)).
		GATES.put("scheduling-disabled", "true".equals(environment.getProperty("scheduling_disabled")));
	}

	@Override
	public void postProcessBeanFactory(ConfigurableListableBeanFactory beanFactory) {}

	private boolean evaluate(ConditionContext context, String className, String method, String only) {
		AnnotatedTypeMetadata metadata;
		try {
			AnnotationMetadata type = AnnotationMetadata.introspect(ClassUtils.forName(className, classLoader));
			metadata = type;
			if (method != null) {
				metadata = null;
				for (MethodMetadata m : type.getDeclaredMethods()) {
					if (m.getMethodName().equals(method)) metadata = m;
				}
				if (metadata == null) throw new IllegalStateException("no method " + className + "." + method);
			}
		} catch (ClassNotFoundException e) {
			throw new IllegalStateException("gate site missing from the image: " + className, e);
		}
		List<Condition> conditions = new ArrayList<>();
		MultiValueMap<String, Object> attrs = metadata.getAllAnnotationAttributes(Conditional.class.getName(), true);
		if (attrs == null) throw new IllegalStateException("no @Conditional on " + className + " " + method);
		for (Object value : attrs.get("value")) {
			for (String conditionClass : (String[]) value) {
				if (only != null && !only.equals(conditionClass)) continue;
				try {
					conditions.add((Condition) BeanUtils.instantiateClass(ClassUtils.forName(conditionClass, classLoader)));
				} catch (ClassNotFoundException e) {
					throw new IllegalStateException(e);
				}
			}
		}
		if (conditions.isEmpty()) throw new IllegalStateException("no condition kept on " + className + " " + method);
		AnnotationAwareOrderComparator.sort(conditions);
		for (Condition c : conditions) {
			if (!c.matches(context, metadata)) return false;
		}
		return true;
	}

	private record Context(
			BeanDefinitionRegistry registry, Environment environment, ResourceLoader resourceLoader, ClassLoader classLoader)
			implements ConditionContext {
		@Override
		public BeanDefinitionRegistry getRegistry() {
			return registry;
		}

		@Override
		public ConfigurableListableBeanFactory getBeanFactory() {
			return registry instanceof ConfigurableListableBeanFactory f ? f : null;
		}

		@Override
		public Environment getEnvironment() {
			return environment;
		}

		@Override
		public ResourceLoader getResourceLoader() {
			return resourceLoader;
		}

		@Override
		public ClassLoader getClassLoader() {
			return classLoader;
		}
	}
}
