//! The element-order and type tables the codec reads.
//!
//! This module defines the **shape** of the tables. The data itself is
//! `frevo-codegen`'s output: tasks 7 to 14 of `add-fhirpath-and-codegen-r4`
//! generate `crates/frevo-model/src/r4/` from `hl7.fhir.r4.core#4.0.1`, and
//! `codegen.roundtrip.004` is the scenario that grades the element order.
//!
//! Until that output exists, [`crate::codec::provisional_r4::TABLES`] supplies
//! the subset the Phase C oracles exercise. See that module's header.

/// What a type is, which decides how its values are encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    /// A FHIR primitive: a JSON string, number or boolean.
    Primitive,
    /// A FHIR complex datatype or backbone element: a JSON object.
    Complex,
    /// A resource: a JSON object carrying `resourceType`.
    Resource,
    /// The abstract `Resource` placeholder a `contained` entry or a
    /// `Bundle.entry.resource` declares. The concrete type is read from the
    /// instance's `resourceType`.
    AnyResource,
}

/// How a primitive's lexical form is spelled in JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonForm {
    /// Quoted: `string`, `code`, `date`, `uri`, `xhtml`, ...
    Quoted,
    /// Bare `true`/`false`.
    Boolean,
    /// A bare JSON number, emitted from its lexical form so precision survives
    /// (`model.canonical.003`).
    Number,
}

/// One element of one type, in the position the StructureDefinition snapshot
/// declares it. Position in [`TypeDef::elements`] *is* the generated element
/// order.
#[derive(Debug, Clone, Copy)]
pub struct ElementDef {
    /// The base element name, with no `[x]` suffix and no typed spelling.
    pub name: &'static str,
    /// Whether this is a `value[x]`-style choice element.
    pub choice: bool,
    /// Whether the element may repeat (`max` other than `1`).
    pub repeating: bool,
    /// The declared types, in declared order. For a choice element this is the
    /// full choice set that `model.nav.011` grades.
    pub types: &'static [&'static str],
}

/// One type, with its base type and its elements in generated element order.
#[derive(Debug, Clone, Copy)]
pub struct TypeDef {
    pub name: &'static str,
    /// The immediate base type: `code` -> `string`, `Age` -> `Quantity`.
    pub base: Option<&'static str>,
    pub kind: TypeKind,
    /// Only meaningful when `kind` is [`TypeKind::Primitive`].
    pub json_form: JsonForm,
    pub elements: &'static [ElementDef],
}

/// A lookup over a set of [`TypeDef`]s.
#[derive(Debug, Clone, Copy)]
pub struct Tables {
    pub types: &'static [TypeDef],
}

impl Tables {
    pub fn type_def(&self, name: &str) -> Option<&'static TypeDef> {
        self.types.iter().find(|t| t.name == name)
    }

    pub fn is_primitive(&self, name: &str) -> bool {
        self.type_def(name)
            .is_some_and(|t| t.kind == TypeKind::Primitive)
    }

    pub fn is_resource_type(&self, name: &str) -> bool {
        self.type_def(name)
            .is_some_and(|t| matches!(t.kind, TypeKind::Resource | TypeKind::AnyResource))
    }

    pub fn base_type(&self, name: &str) -> Option<&'static str> {
        self.type_def(name).and_then(|t| t.base)
    }

    pub fn derives_from(&self, name: &str, ancestor: &str) -> bool {
        let mut current = name;
        for _ in 0..64 {
            match self.base_type(current) {
                Some(base) if base == ancestor => return true,
                Some(base) => current = base,
                None => return false,
            }
        }
        false
    }

    pub fn element(&self, type_name: &str, element: &str) -> Option<&'static ElementDef> {
        let mut current = Some(type_name);
        while let Some(name) = current {
            let def = self.type_def(name)?;
            if let Some(found) = def.elements.iter().find(|e| e.name == element) {
                return Some(found);
            }
            current = def.base;
        }
        None
    }

    /// The JSON property name a choice element takes for `type_name`:
    /// `value` plus `Quantity` is `valueQuantity`.
    pub fn choice_property(base: &str, type_name: &str) -> String {
        let mut out = String::with_capacity(base.len() + type_name.len());
        Self::write_choice_property(base, type_name, &mut out);
        out
    }

    /// [`Tables::choice_property`] into a buffer the caller reuses. Decoding
    /// touches this once per candidate type per property, and `Extension`'s
    /// `value[x]` has forty of them, so allocating here is the difference
    /// between a decode budget met and missed.
    pub fn write_choice_property(base: &str, type_name: &str, out: &mut String) {
        out.clear();
        out.push_str(base);
        let mut chars = type_name.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }

    /// Whether `property` is `base` followed by `type_name` with its first
    /// character upper-cased. FHIR element and type names are ASCII, so this
    /// compares bytes and allocates nothing.
    pub fn is_choice_property(base: &str, type_name: &str, property: &[u8]) -> bool {
        let Some(rest) = property.strip_prefix(base.as_bytes()) else {
            return false;
        };
        let suffix = type_name.as_bytes();
        let Some((first, tail)) = suffix.split_first() else {
            return false;
        };
        let Some((head, rest)) = rest.split_first() else {
            return false;
        };
        *head == first.to_ascii_uppercase() && rest == tail
    }

    /// Resolve a JSON property name of `type_name` to the element it belongs
    /// to, the concrete type the property carries, and whether the property is
    /// the `_field` sibling rather than the value.
    pub fn resolve_property(
        &self,
        type_name: &str,
        property: &str,
    ) -> Option<(&'static ElementDef, &'static str, bool)> {
        self.resolve_property_bytes(type_name, property.as_bytes())
    }

    /// [`Tables::resolve_property`] over the raw property-name bytes, so a
    /// decode never has to turn a property name into a `String` to look it up.
    pub fn resolve_property_bytes(
        &self,
        type_name: &str,
        property: &[u8],
    ) -> Option<(&'static ElementDef, &'static str, bool)> {
        if property == b"resourceType" {
            return None;
        }
        let (name, sibling) = match property.strip_prefix(b"_") {
            Some(rest) => (rest, true),
            None => (property, false),
        };
        let mut current = Some(type_name);
        while let Some(owner) = current {
            let def = self.type_def(owner)?;
            for element in def.elements {
                if !element.choice {
                    if element.name.as_bytes() == name {
                        return Some((element, element.types[0], sibling));
                    }
                    continue;
                }
                for candidate in element.types {
                    if Self::is_choice_property(element.name, candidate, name) {
                        return Some((element, candidate, sibling));
                    }
                }
            }
            current = def.base;
        }
        None
    }

    /// Every JSON property name `element` can take, in declared order: one for
    /// an ordinary element, one per choice type for a choice element.
    pub fn property_names(element: &ElementDef) -> Vec<String> {
        if element.choice {
            element
                .types
                .iter()
                .map(|t| Self::choice_property(element.name, t))
                .collect()
        } else {
            vec![element.name.to_string()]
        }
    }

    pub fn json_form(&self, type_name: &str) -> JsonForm {
        let mut current = Some(type_name);
        while let Some(name) = current {
            let Some(def) = self.type_def(name) else {
                return JsonForm::Quoted;
            };
            if def.kind == TypeKind::Primitive {
                return def.json_form;
            }
            current = def.base;
        }
        JsonForm::Quoted
    }
}
