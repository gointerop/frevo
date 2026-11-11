//! The binding the `codegen-r4` oracle drives.
//!
//! `crates/frevo-codegen/tests/codegen_r4.rs` defines the seam it grades: a
//! `Codegen` trait and the data model its assertions read. Those are *test*
//! types, so the `impl Codegen for ...` that points them at this crate has to
//! live in the test crate; no library can name them.
//!
//! [`bind_oracle!`] is how it gets there without the oracle file growing a
//! conversion layer. The macro expands, at the call site, to the struct the
//! oracle constructs and the whole `impl`, so the oracle file carries two
//! lines and this crate carries the mapping. Tasks 8 to 14 extend the macro;
//! none of them needs to touch the oracle, which is the point — a file that
//! grades an implementation should not be edited by the implementation.

/// Bind the oracle's `Codegen` trait to this crate.
///
/// Expands to `struct <name>;` and `impl Codegen for <name>`, using the trait
/// and the data model in scope at the call site.
///
/// ```ignore
/// fn generator_under_test() -> Box<dyn Codegen> {
///     Box::new(BoundGenerator)
/// }
///
/// frevo_codegen::bind_oracle!(BoundGenerator);
/// ```
///
/// Methods whose behaviour has not landed yet answer
/// `CodegenError::NotImplemented`, which the oracle treats as an error value
/// and never as a verdict — including on the scenarios that require a refusal.
#[macro_export]
macro_rules! bind_oracle {
    ($name:ident) => {
        /// The generator under test, bound to `frevo-codegen` by
        /// `frevo_codegen::bind_oracle!`.
        struct $name;

        impl $name {
            fn failure_kind(kind: $crate::error::FailureKind) -> FailureKind {
                match kind {
                    $crate::error::FailureKind::DigestMismatch => FailureKind::DigestMismatch,
                    $crate::error::FailureKind::PackageMissing => FailureKind::PackageMissing,
                    $crate::error::FailureKind::InventoryMismatch => FailureKind::InventoryMismatch,
                    $crate::error::FailureKind::NameCollision => FailureKind::NameCollision,
                    $crate::error::FailureKind::Codec => FailureKind::Codec,
                    $crate::error::FailureKind::Other => FailureKind::Other,
                }
            }

            fn error(error: $crate::error::CodegenError) -> CodegenError {
                match error {
                    $crate::error::CodegenError::Failed {
                        exit_code,
                        kind,
                        message,
                    } => CodegenError::Failed {
                        exit_code,
                        kind: Self::failure_kind(kind),
                        message,
                    },
                    $crate::error::CodegenError::NotImplemented => CodegenError::NotImplemented,
                }
            }

            fn request(request: &GenerateRequest) -> $crate::run::GenerateRequest {
                $crate::run::GenerateRequest {
                    fhir_version: request.fhir_version.clone(),
                    package_dir: request.package_dir.clone(),
                    expected_sha256: request.expected_sha256.clone(),
                    out_dir: request.out_dir.clone(),
                    reference_dir: request.reference_dir.clone(),
                }
            }

            fn synthetic_request(request: &SyntheticRequest) -> $crate::synthetic::Request {
                $crate::synthetic::Request {
                    definitions: request
                        .definitions
                        .iter()
                        .map(|definition| $crate::synthetic::Definition {
                            url: definition.url.clone(),
                            name: definition.name.clone(),
                            kind: match definition.kind {
                                DefinitionKind::PrimitiveType => {
                                    $crate::synthetic::Kind::PrimitiveType
                                }
                                DefinitionKind::ComplexType => $crate::synthetic::Kind::ComplexType,
                                DefinitionKind::Resource => $crate::synthetic::Kind::Resource,
                                DefinitionKind::Logical => $crate::synthetic::Kind::Logical,
                            },
                            base: definition.base.clone(),
                            is_abstract: definition.is_abstract,
                            derivation: definition.derivation.clone(),
                            elements: definition
                                .elements
                                .iter()
                                .map(|element| $crate::synthetic::Element {
                                    path: element.path.clone(),
                                    min: element.min,
                                    max: element.max.clone(),
                                    types: element.types.clone(),
                                    content_reference: element.content_reference.clone(),
                                    binding: element.binding.as_ref().map(|binding| {
                                        $crate::synthetic::Binding {
                                            strength: binding.strength.clone(),
                                            value_set: binding.value_set.clone(),
                                        }
                                    }),
                                })
                                .collect(),
                        })
                        .collect(),
                    value_sets: request
                        .value_sets
                        .iter()
                        .map(|value_set| $crate::synthetic::ValueSet {
                            url: value_set.url.clone(),
                            codes: value_set.codes.clone(),
                        })
                        .collect(),
                    out_dir: request.out_dir.clone(),
                    assert_pinned_inventory: request.assert_pinned_inventory,
                }
            }

            fn variant(variant: $crate::bindings::Variant) -> EnumVariant {
                EnumVariant {
                    code: variant.code,
                    rust_name: variant.rust_name,
                }
            }

            fn enumeration(generated: $crate::bindings::Enumeration) -> GeneratedEnum {
                GeneratedEnum {
                    rust_name: generated.rust_name,
                    value_set: generated.value_set,
                    element_paths: generated.element_paths,
                    variants: generated.variants.into_iter().map(Self::variant).collect(),
                }
            }

            fn unresolved_binding(
                binding: $crate::bindings::UnresolvedBinding,
            ) -> UnresolvedBinding {
                UnresolvedBinding {
                    element_path: binding.element_path,
                    value_set_url: binding.value_set_url,
                    strength: binding.strength,
                }
            }

            fn search_param_row(
                $crate::searchparam::Row {
                    id,
                    code,
                    url,
                    search_type,
                    base,
                    declared_bases,
                    expression,
                    components,
                    targets,
                    multiple_or,
                    multiple_and,
                    comparator,
                    modifier,
                    chain,
                }: $crate::searchparam::Row,
            ) -> SearchParamRow {
                SearchParamRow {
                    id,
                    code,
                    url,
                    search_type,
                    base,
                    declared_bases,
                    expression,
                    components: components
                        .into_iter()
                        .map(|component| SearchParamComponent {
                            definition: component.definition,
                            expression: component.expression,
                        })
                        .collect(),
                    targets,
                    multiple_or,
                    multiple_and,
                    comparator,
                    modifier,
                    chain,
                }
            }

            fn max(max: $crate::model::Max) -> Max {
                match max {
                    $crate::model::Max::Zero => Max::Zero,
                    $crate::model::Max::One => Max::One,
                    $crate::model::Max::Many => Max::Many,
                }
            }

            fn type_kind(kind: $crate::model::TypeKind) -> TypeKind {
                match kind {
                    $crate::model::TypeKind::Primitive => TypeKind::Primitive,
                    $crate::model::TypeKind::Complex => TypeKind::Complex,
                    $crate::model::TypeKind::Resource => TypeKind::Resource,
                    $crate::model::TypeKind::Backbone => TypeKind::Backbone,
                    $crate::model::TypeKind::AbstractElementSet => TypeKind::AbstractElementSet,
                    $crate::model::TypeKind::Logical => TypeKind::Logical,
                }
            }

            fn element(element: $crate::model::Element) -> GeneratedElement {
                GeneratedElement {
                    fhir_name: element.fhir_name,
                    rust_name: element.rust_name,
                    path: element.path,
                    types: element.types,
                    is_choice: element.is_choice,
                    json_property_names: element.json_property_names,
                    min: element.min,
                    max: Self::max(element.max),
                    binding: element.binding.map(|binding| BindingInfo {
                        strength: binding.strength,
                        value_set: binding.value_set,
                        enumeration: binding.enumeration,
                    }),
                    content_reference: element.content_reference,
                }
            }

            fn generated_type(ty: $crate::model::Type) -> GeneratedType {
                GeneratedType {
                    element_order: ty.element_order(),
                    fhir_name: ty.fhir_name,
                    rust_name: ty.rust_name,
                    kind: Self::type_kind(ty.kind),
                    base: ty.base,
                    instantiable: ty.instantiable,
                    elements: ty.elements.into_iter().map(Self::element).collect(),
                }
            }

            fn report(report: $crate::run::GenerateReport) -> GenerateReport {
                GenerateReport {
                    exit_code: report.exit_code,
                    pin: PackagePin {
                        name: report.pin.name,
                        version: report.pin.version,
                        sha256: report.pin.sha256,
                    },
                    summary: report.summary,
                    inventory: Inventory {
                        datatype_definitions: report.inventory.datatype_definitions,
                        resource_definitions: report.inventory.resource_definitions,
                        concrete_resources: report.inventory.concrete_resources,
                        abstract_resources: report.inventory.abstract_resources,
                        logical_resources: report.inventory.logical_resources,
                        search_parameters: report.inventory.search_parameters,
                    },
                    inputs_read: report.inputs_read,
                    types: report
                        .model
                        .types
                        .into_iter()
                        .map(Self::generated_type)
                        .collect(),
                    resource_registry: report
                        .model
                        .resource_registry
                        .into_iter()
                        .map(|(fhir_name, rust_name)| RegistryEntry {
                            fhir_name,
                            rust_name,
                        })
                        .collect(),
                    enumerations: report
                        .model
                        .enumerations
                        .into_iter()
                        .map(Self::enumeration)
                        .collect(),
                    unresolved_bindings: report
                        .model
                        .unresolved_bindings
                        .into_iter()
                        .map(Self::unresolved_binding)
                        .collect(),
                    search_parameters: report
                        .search_parameters
                        .into_iter()
                        .map(Self::search_param_row)
                        .collect(),
                    files: report
                        .files
                        .into_iter()
                        .map(|file| GeneratedFile {
                            path: file.path,
                            bytes: file.bytes,
                            sha256: file.sha256,
                        })
                        .collect(),
                }
            }

            fn cross_check_report(
                report: $crate::crosscheck::CrossCheckReport,
            ) -> CrossCheckReport {
                CrossCheckReport {
                    scope: report.scope,
                    package_fhir_version: report.package_fhir_version,
                    hapi_fhir_version: report.hapi_fhir_version,
                    hapi_definition_count: report.hapi_definition_count,
                    only_in_package: report.only_in_package,
                    only_in_hapi: report.only_in_hapi,
                    element_disagreements: report
                        .element_disagreements
                        .into_iter()
                        .map(|disagreement| ElementDisagreement {
                            url: disagreement.url,
                            path: disagreement.path,
                            detail: disagreement.detail,
                        })
                        .collect(),
                }
            }

            fn drift_report(report: $crate::run::DriftReport) -> DriftReport {
                DriftReport {
                    in_sync: report.in_sync,
                    differing_files: report.differing_files,
                    remediation: report.remediation,
                }
            }
        }

        impl Codegen for $name {
            fn recorded_pin(&self) -> Result<PackagePin, CodegenError> {
                let pin = $crate::pin::recorded_pin();
                Ok(PackagePin {
                    name: pin.name,
                    version: pin.version,
                    sha256: pin.sha256,
                })
            }

            fn generate(&self, request: &GenerateRequest) -> Result<GenerateReport, CodegenError> {
                $crate::run::generate(&Self::request(request))
                    .map(Self::report)
                    .map_err(Self::error)
            }

            fn generate_synthetic(
                &self,
                request: &SyntheticRequest,
            ) -> Result<GenerateReport, CodegenError> {
                $crate::run::generate_synthetic(&Self::synthetic_request(request))
                    .map(Self::report)
                    .map_err(Self::error)
            }

            fn cross_check(
                &self,
                package_dir: Option<&Path>,
                hapi_profile_dir: &Path,
            ) -> Result<CrossCheckReport, CodegenError> {
                $crate::run::cross_check(package_dir, hapi_profile_dir)
                    .map(Self::cross_check_report)
                    .map_err(Self::error)
            }

            fn drift_check(
                &self,
                request: &GenerateRequest,
                checked_in_dir: &Path,
            ) -> Result<DriftReport, CodegenError> {
                $crate::run::drift_check(&Self::request(request), checked_in_dir)
                    .map(Self::drift_report)
                    .map_err(Self::error)
            }

            fn rust_field_name(&self, element_path: &str) -> Result<String, CodegenError> {
                // Total by construction: the rule is applied to the last
                // segment of the path, so it answers for a path the package
                // does not contain.
                Ok($crate::model::rust_field_name_of_path(element_path))
            }

            fn code_variants(&self, codes: &[&str]) -> Result<Vec<EnumVariant>, CodegenError> {
                Ok($crate::bindings::code_variants(codes)
                    .into_iter()
                    .map(Self::variant)
                    .collect())
            }

            fn parameters_for(
                &self,
                resource_type: &str,
            ) -> Result<Vec<SearchParamRow>, CodegenError> {
                $crate::run::parameters_for(resource_type)
                    .map(|rows| rows.into_iter().map(Self::search_param_row).collect())
                    .map_err(Self::error)
            }

            // --- the codec, and the FHIRPath parser ----------------------
            // Each of these is bound by the task that implements it: the
            // codec is tasks 19 to 23 and the parser is tasks 28 to 30. They
            // answer NotImplemented rather than a plausible-looking default,
            // so no scenario can read as satisfied by silence.

            fn round_trip(&self, _examples: &[PathBuf]) -> Result<RoundTripReport, CodegenError> {
                Err(CodegenError::NotImplemented)
            }

            fn decode(&self, _json: &str, _mode: DecodeMode) -> Result<Decoded, CodegenError> {
                Err(CodegenError::NotImplemented)
            }

            fn encode(&self, _decoded: &Decoded) -> Result<String, CodegenError> {
                Err(CodegenError::NotImplemented)
            }

            fn encode_canonical(&self, _decoded: &Decoded) -> Result<String, CodegenError> {
                Err(CodegenError::NotImplemented)
            }

            fn parse_search_expression(&self, _expression: &str) -> Result<(), CodegenError> {
                Err(CodegenError::NotImplemented)
            }
        }
    };
}
