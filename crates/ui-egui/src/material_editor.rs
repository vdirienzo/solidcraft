//! Design-owned physical materials, edited inside the existing assignment panel.
//! Drafts never mutate the document; Save is one ordinary material command.
use serde_json::{Value, json};
use solidcraft_engine::doc::physical_material::{
    Conditions, Elasticity, Family, MaterialAxes, MaterialCard, Property, PropertyData, Source, SourceKind,
};
use solidcraft_engine::doc::{Document, UserMaterial, appearance::Look};

use crate::{SolveApp, i18n::tr, ui_kit};

const KINDS: [&str; 9] = [
    "density",
    "elasticity",
    "tensile_strength",
    "compressive_strength",
    "yield_strength",
    "shear_strength",
    "thermal_conductivity",
    "thermal_expansion",
    "specific_heat",
];
const FAMILIES: [Family; 5] = [Family::Other, Family::Metal, Family::Polymer, Family::Wood, Family::Glass];
const SOURCE_KINDS: [SourceKind; 4] = [SourceKind::UserSupplied, SourceKind::Measurement, SourceKind::PublicReference, SourceKind::Illustrative];

#[derive(Clone, Debug, Default)]
pub struct Editor {
    pub(crate) draft: Option<Draft>,
    error: Option<String>,
    note: Option<String>,
    chosen_material: Option<String>,
    observed_index: Option<usize>,
    mixed_materials: bool,
}

impl Editor {
    /// Follow the targets until the user deliberately chooses a material. Keep a
    /// choice by name: inserting, renaming or deleting a library row must not
    /// silently turn it into another material at the same ordinal.
    pub(crate) fn sync_selection(&mut self, doc: &Document, bodies: &[String], index: &mut usize) {
        if self.editing() {
            return;
        }
        let names = doc.material_names();
        if self.observed_index.is_some_and(|previous| previous != *index) {
            self.chosen_material = names.get(*index).cloned();
        }
        let materials: Vec<_> = bodies
            .iter()
            .map(|body| {
                let assigned = doc.materials.get(body).map(String::as_str).unwrap_or(solidcraft_engine::doc::DEFAULT_MATERIAL);
                doc.material_name(assigned).filter(|name| doc.material_density(name).is_some())
            })
            .collect();
        self.mixed_materials = materials.first().is_some_and(|first| materials.iter().any(|material| material != first));
        let common = materials.first().and_then(Option::as_ref).filter(|first| materials.iter().all(|material| material.as_ref() == Some(*first)));
        let selected = self.chosen_material.as_ref().or(common);
        *index = selected.and_then(|name| names.iter().position(|candidate| candidate == name)).unwrap_or(usize::MAX);
        self.observed_index = Some(*index);
    }

    pub fn editing(&self) -> bool {
        self.draft.is_some()
    }

    pub(crate) fn begin(&mut self, doc: &Document, source: Option<&str>, duplicate: bool) {
        self.draft = Some(Draft::new(doc, source, duplicate));
        self.error = None;
        self.note = None;
    }

    pub(crate) fn save(&mut self, app: &mut SolveApp, index: &mut usize) -> Result<(), String> {
        let draft = self.draft.as_ref().ok_or_else(|| tr("material.editor.no_draft").to_string())?;
        let name = draft.name.trim().to_string();
        if let Some((command, params)) = draft.command(&app.session.doc)? {
            app.run(command, params)?;
        }
        *index = app.session.doc.material_names().iter().position(|n| n == &name).unwrap_or(*index);
        self.chosen_material = Some(name);
        self.observed_index = Some(*index);
        self.draft = None;
        self.error = None;
        self.note = Some(tr("material.editor.saved").to_string());
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Draft {
    original: Option<UserMaterial>,
    copied_appearance: Option<Look>,
    pub(crate) name: String,
    pub(crate) density: String,
    pub(crate) card_enabled: bool,
    pub(crate) card: CardDraft,
}

impl Draft {
    fn new(doc: &Document, source: Option<&str>, duplicate: bool) -> Self {
        let user = source.and_then(|name| doc.user_material(name)).cloned();
        let original = (!duplicate).then(|| user.clone()).flatten();
        let name = match source {
            Some(name) if duplicate => crate::i18n::format("material.editor.copy_name", &[("name", name)]),
            Some(name) => name.to_string(),
            None => String::new(),
        };
        let saved = user.as_ref().and_then(|m| m.physical_card.as_ref()).map(|card| card.card());
        Self {
            original,
            copied_appearance: source.and_then(|name| doc.material_look(name)),
            density: source.and_then(|name| doc.material_density(name)).map(|n| n.to_string()).unwrap_or_default(),
            card_enabled: saved.is_some(),
            card: saved.map(CardDraft::from_card).unwrap_or_else(|| CardDraft::empty(&name)),
            name,
        }
    }

    /// Omit unchanged fields on edit. Appearance is never reconstructed or reset.
    pub(crate) fn command(&self, doc: &Document) -> Result<Option<(&'static str, Value)>, String> {
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > 128 {
            return Err(tr("material.editor.name_required").to_string());
        }
        let density = number(&self.density, "density_g_cm3")?;
        if !solidcraft_engine::doc::valid_density(density) {
            return Err(tr("material.editor.density_range").to_string());
        }
        let card = if self.card_enabled { Some(self.card.build(Some(density_kg_m3(density)))?) } else { None };
        if let Some(original) = &self.original {
            if doc.user_material(&original.name) != Some(original) {
                return Err(tr("material.editor.stale").to_string());
            }
            let mut params = json!({"material": original.name});
            if name != original.name {
                params["name"] = json!(name);
            }
            if density != original.density {
                params["density_g_cm3"] = json!(density);
            }
            if card.as_ref() != original.physical_card.as_ref().map(|c| c.card()) {
                params["physical_card"] = json!(card);
            }
            if params.as_object().is_some_and(|p| p.len() == 1) {
                return Ok(None);
            }
            Ok(Some(("material.edit", params)))
        } else {
            let mut params = json!({"name":name,"density_g_cm3":density});
            if let Some(card) = card {
                params["physical_card"] = json!(card);
            }
            if let Some(look) = &self.copied_appearance {
                // Duplicate optical values without retaining a live library link.
                params["color"] = json!(look.color);
                params["opacity"] = json!(look.opacity);
                if let Some(pbr) = look.pbr {
                    params["pbr"] = json!(pbr);
                }
            }
            Ok(Some(("material.create", params)))
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CardDraft {
    pub(crate) designation: String,
    pub(crate) family: Family,
    pub(crate) sources: Vec<Source>,
    pub(crate) properties: Vec<PropertyDraft>,
    /// The user wrote their own designation; until then it follows the material name.
    designation_owned: bool,
    add_kind: &'static str,
}

impl CardDraft {
    fn empty(name: &str) -> Self {
        Self {
            designation: name.into(),
            family: Family::Other,
            sources: Vec::new(),
            properties: Vec::new(),
            designation_owned: false,
            add_kind: "density",
        }
    }
    fn from_card(card: &MaterialCard) -> Self {
        Self {
            designation: card.designation.clone(),
            family: card.family,
            sources: card.sources.clone(),
            properties: card.properties.iter().map(PropertyDraft::from_property).collect(),
            designation_owned: true,
            add_kind: "density",
        }
    }
    /// The property types this card does not carry yet: a card holds each kind once.
    fn remaining_kinds(&self) -> Vec<&'static str> {
        KINDS.iter().copied().filter(|kind| !self.properties.iter().any(|p| p.kind == *kind)).collect()
    }
    /// A property names a source. A card that carries properties but no source gets
    /// an implicit one, so the user never has to author literature to save a card.
    fn normalize_sources(&mut self) {
        if self.sources.is_empty() && !self.properties.is_empty() {
            self.sources.push(implicit_source());
        }
        let first = self.sources.first().map(|s| s.id.clone()).unwrap_or_default();
        for property in &mut self.properties {
            if !self.sources.iter().any(|s| s.id == property.source_id) {
                property.source_id = first.clone();
            }
        }
    }
    fn add_property(&mut self, kind: &'static str, assigned_kg_m3: Option<f64>) {
        self.normalize_sources();
        let source_id = self.sources.first().map(|s| s.id.clone()).unwrap_or_default();
        let mut property = PropertyDraft::empty(kind, source_id);
        property.mirror_density(assigned_kg_m3);
        self.properties.push(property);
    }
    pub(crate) fn build(&self, assigned_kg_m3: Option<f64>) -> Result<MaterialCard, String> {
        let mut sources = self.sources.clone();
        if sources.is_empty() && !self.properties.is_empty() {
            sources.push(implicit_source());
        }
        let fallback = sources.first().map(|s| s.id.clone()).unwrap_or_default();
        let properties = self
            .properties
            .iter()
            .map(|property| {
                let source_id =
                    sources.iter().find(|s| s.id == property.source_id).map(|s| s.id.clone()).unwrap_or_else(|| fallback.clone());
                property.build(&source_id, assigned_kg_m3)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let card = MaterialCard {
            schema_version: 1,
            designation: self.designation.clone(),
            family: self.family,
            sources,
            properties,
        };
        card.validate().map_err(|e| e.to_string())?;
        Ok(card)
    }
}

/// The source a card carries when the user writes no literature of their own.
fn implicit_source() -> Source {
    Source {
        id: "source-1".into(),
        title: tr("material.editor.default_source_title").to_string(),
        kind: SourceKind::UserSupplied,
        license: None,
        reference: None,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PropertyDraft {
    pub(crate) kind: &'static str,
    pub(crate) values: Vec<String>,
    pub(crate) orthotropic: bool,
    pub(crate) axes: MaterialAxes,
    pub(crate) source_id: String,
    /// Temperature K, moisture kg/kg dry basis, strain rate s^-1, state.
    pub(crate) conditions: [String; 4],
    /// The user wrote a density of their own; until then it shows the assigned density.
    density_edited: bool,
}

impl PropertyDraft {
    pub(crate) fn empty(kind: &'static str, source_id: String) -> Self {
        let count = match kind {
            "elasticity" => 2,
            "thermal_conductivity" | "thermal_expansion" => 3,
            _ => 1,
        };
        Self {
            kind,
            values: vec![String::new(); count],
            orthotropic: false,
            axes: MaterialAxes::Material123,
            source_id,
            conditions: Default::default(),
            density_edited: false,
        }
    }
    fn from_property(property: &Property) -> Self {
        let (values, orthotropic, axes) = match &property.data {
            PropertyData::Density { kg_m3 } => (vec![*kg_m3], false, MaterialAxes::Material123),
            PropertyData::Elasticity { model: Elasticity::Isotropic { young_modulus_pa, poisson_ratio } } => {
                (vec![*young_modulus_pa, *poisson_ratio], false, MaterialAxes::Material123)
            }
            PropertyData::Elasticity { model: Elasticity::Orthotropic { axes, young_moduli_pa, shear_moduli_pa, poisson_ratios } } => {
                (young_moduli_pa.iter().chain(shear_moduli_pa).chain(poisson_ratios).copied().collect(), true, *axes)
            }
            PropertyData::TensileStrength { pa }
            | PropertyData::CompressiveStrength { pa }
            | PropertyData::YieldStrength { pa }
            | PropertyData::ShearStrength { pa } => (vec![*pa], false, MaterialAxes::Material123),
            PropertyData::ThermalConductivity { w_m_k, axes } => (w_m_k.to_vec(), false, *axes),
            PropertyData::ThermalExpansion { per_k, axes } => (per_k.to_vec(), false, *axes),
            PropertyData::SpecificHeat { j_kg_k } => (vec![*j_kg_k], false, MaterialAxes::Material123),
        };
        let c = &property.conditions;
        Self {
            kind: property.data.kind(),
            values: values.iter().map(|v| v.to_string()).collect(),
            orthotropic,
            axes,
            source_id: property.source_id.clone(),
            conditions: [
                c.temperature_k.map(|v| v.to_string()).unwrap_or_default(),
                c.moisture_dry_basis_kg_kg.map(|v| v.to_string()).unwrap_or_default(),
                c.strain_rate_per_s.map(|v| v.to_string()).unwrap_or_default(),
                c.state.clone().unwrap_or_default(),
            ],
            density_edited: true,
        }
    }
    /// The density property starts from the assigned density until the user writes
    /// a value of their own; a card never carries a density nobody entered.
    fn mirror_density(&mut self, assigned_kg_m3: Option<f64>) {
        if self.kind != "density" || self.density_edited {
            return;
        }
        if let Some(value) = self.values.first_mut() {
            *value = assigned_kg_m3.map(density_text).unwrap_or_default();
        }
    }
    /// The density the card carries: the user's own value, or the assigned density
    /// while the user has written none. A blank value never becomes an invented zero.
    fn density_value(&self, assigned_kg_m3: Option<f64>) -> Result<f64, String> {
        let text = self.values.first().map(String::as_str).unwrap_or("").trim();
        if text.is_empty() && !self.density_edited && let Some(kg_m3) = assigned_kg_m3 {
            return Ok(kg_m3);
        }
        number(text, self.kind)
    }
    fn build(&self, source_id: &str, assigned_kg_m3: Option<f64>) -> Result<Property, String> {
        let n = |i: usize| number(self.values.get(i).map(String::as_str).unwrap_or(""), self.kind);
        let vector = |offset| Ok::<_, String>([n(offset)?, n(offset + 1)?, n(offset + 2)?]);
        let data = match self.kind {
            "density" => PropertyData::Density { kg_m3: self.density_value(assigned_kg_m3)? },
            "elasticity" if self.orthotropic => PropertyData::Elasticity {
                model: Elasticity::Orthotropic {
                    axes: self.axes,
                    young_moduli_pa: vector(0)?,
                    shear_moduli_pa: vector(3)?,
                    poisson_ratios: vector(6)?,
                },
            },
            "elasticity" => PropertyData::Elasticity { model: Elasticity::Isotropic { young_modulus_pa: n(0)?, poisson_ratio: n(1)? } },
            "tensile_strength" => PropertyData::TensileStrength { pa: n(0)? },
            "compressive_strength" => PropertyData::CompressiveStrength { pa: n(0)? },
            "yield_strength" => PropertyData::YieldStrength { pa: n(0)? },
            "shear_strength" => PropertyData::ShearStrength { pa: n(0)? },
            "thermal_conductivity" => PropertyData::ThermalConductivity { w_m_k: vector(0)?, axes: self.axes },
            "thermal_expansion" => PropertyData::ThermalExpansion { per_k: vector(0)?, axes: self.axes },
            "specific_heat" => PropertyData::SpecificHeat { j_kg_k: n(0)? },
            _ => return Err(tr("material.editor.invalid_property").to_string()),
        };
        let optional = |i: usize| -> Result<Option<f64>, String> {
            let text = self.conditions.get(i).map(String::as_str).unwrap_or("");
            if text.trim().is_empty() { Ok(None) } else { number(text, "conditions").map(Some) }
        };
        Ok(Property {
            data,
            source_id: source_id.to_string(),
            conditions: Conditions {
                temperature_k: optional(0)?,
                moisture_dry_basis_kg_kg: optional(1)?,
                strain_rate_per_s: optional(2)?,
                state: self.conditions.get(3).filter(|s| !s.is_empty()).cloned(),
            },
        })
    }
}

fn number(value: &str, field: &str) -> Result<f64, String> {
    crate::prefs::parse_number(value).ok_or_else(|| format!("{field}: {}", tr("material.editor.number_required")))
}

/// A number without the artefact of a binary fraction: 7.85 g/cm³ reads as 7850
/// kg/m³, not as 7849.999999999999.
fn tidy(value: f64) -> f64 {
    format!("{value:.14e}").parse::<f64>().unwrap_or(value)
}

/// An assigned density in g/cm³ as kg/m³, the unit a card carries.
fn density_kg_m3(density_g_cm3: f64) -> f64 {
    tidy(density_g_cm3 * 1000.0)
}

/// A density in kg/m³ as the card writes it.
fn density_text(kg_m3: f64) -> String {
    tidy(kg_m3).to_string()
}

/// The assigned density in kg/m³, when the user wrote a valid one.
fn assigned_density_kg_m3(density_g_cm3: &str) -> Option<f64> {
    let grams = crate::prefs::parse_number(density_g_cm3)?;
    solidcraft_engine::doc::valid_density(grams).then(|| density_kg_m3(grams))
}

fn field(ui: &mut egui::Ui, label: &str, id: &str, value: &mut String, unit: &str) {
    let label = if unit.is_empty() { label.to_string() } else { format!("{label} ({unit})") };
    let response = ui_kit::row(ui, &label, |ui| {
        ui_kit::cell_editor(ui, egui::Id::unique(("physical_editor", id)), value, tr("ui.material_review.unknown"), false)
    });
    crate::scenario::publish_handle(&format!("material:field:{id}"), response.rect.center());
}

fn optional_text(ui: &mut egui::Ui, label: &str, id: &str, value: &mut Option<String>) {
    let mut text = value.clone().unwrap_or_default();
    field(ui, label, id, &mut text, "");
    if text != value.clone().unwrap_or_default() {
        *value = (!text.is_empty()).then_some(text);
    }
}

fn axes(ui: &mut egui::Ui, id: &str, axes: &mut MaterialAxes) {
    let mut index = usize::from(*axes == MaterialAxes::WoodLrt);
    ui_kit::choice_row(ui, tr("material.editor.axes"), id, &mut index, &[tr("material.editor.axes_numeric"), tr("material.editor.axes_wood")]);
    *axes = if index == 1 { MaterialAxes::WoodLrt } else { MaterialAxes::Material123 };
}

fn property_rows(ui: &mut egui::Ui, property: &mut PropertyDraft, sources: &[Source], assigned_kg_m3: Option<f64>) {
    let kind = property.kind;
    if kind == "elasticity" {
        let mut index = usize::from(property.orthotropic);
        ui_kit::choice_row(
            ui,
            tr("material.editor.symmetry"),
            &format!("{kind}:symmetry"),
            &mut index,
            &[tr("material.editor.isotropic"), tr("material.editor.orthotropic")],
        );
        if (index == 1) != property.orthotropic {
            property.orthotropic = index == 1;
            // No inferred anisotropic constants: a changed model requires explicit values.
            property.values = vec![String::new(); if property.orthotropic { 9 } else { 2 }];
        }
    }
    if property.orthotropic || matches!(kind, "thermal_conductivity" | "thermal_expansion") {
        axes(ui, &format!("{kind}:axes"), &mut property.axes);
    }
    for (i, value) in property.values.iter_mut().enumerate() {
        let (label, unit) = match kind {
            "density" => ("ρ".into(), "kg/m³"),
            "elasticity" if property.orthotropic => {
                let labels = ["E1", "E2", "E3", "G12", "G23", "G31", "ν12", "ν23", "ν31"];
                (labels.get(i).copied().unwrap_or("").into(), if i < 6 { "Pa" } else { "1" })
            }
            "elasticity" => (if i == 0 { "E" } else { "ν" }.into(), if i == 0 { "Pa" } else { "1" }),
            "thermal_conductivity" => (format!("k{}", i + 1), "W/(m·K)"),
            "thermal_expansion" => (format!("α{}", i + 1), "K⁻¹"),
            "specific_heat" => ("c".into(), "J/(kg·K)"),
            _ => ("σ".into(), "Pa"),
        };
        let before = value.clone();
        field(ui, &label, &format!("{kind}:value:{i}"), value, unit);
        if kind == "density" {
            if *value != before {
                // A density the user wrote replaces the mirrored one; clearing it returns to the mirror.
                property.density_edited = !value.trim().is_empty();
            }
            property.mirror_density(assigned_kg_m3);
        }
    }
    if kind == "density" {
        ui_kit::text(ui, tr("material.editor.density_hint"), ui_kit::Text::Note("physical_density_hint"));
    }
    if !sources.is_empty() {
        // A property always names a source the card carries.
        let mut selected = sources.iter().position(|s| s.id == property.source_id).unwrap_or(0);
        let labels: Vec<_> = sources.iter().map(|s| s.id.as_str()).collect();
        let before = selected;
        ui_kit::choice_row(ui, tr("ui.material_review.source"), &format!("{kind}:source"), &mut selected, &labels);
        if selected != before {
            property.source_id = sources.get(selected).map(|s| s.id.clone()).unwrap_or_default();
        }
    }
}

fn card_rows(ui: &mut egui::Ui, card: &mut CardDraft, assigned_kg_m3: Option<f64>, material_name: &str) {
    let before = card.designation.clone();
    field(ui, tr("material.editor.designation"), "designation", &mut card.designation, "");
    if card.designation != before {
        // A designation the user wrote is theirs; clearing it returns to the material name.
        card.designation_owned = !card.designation.trim().is_empty();
        if !card.designation_owned {
            card.designation = material_name.to_string();
        }
    }
    ui_kit::text(ui, tr("material.editor.designation_hint"), ui_kit::Text::Note("physical_designation"));
    let mut family = FAMILIES.iter().position(|v| v == &card.family).unwrap_or(0);
    let labels =
        ["material.editor.other", "material.editor.metal", "material.editor.polymer", "material.editor.wood", "material.editor.glass"].map(tr);
    ui_kit::choice_row(ui, tr("material.editor.family"), "physical:family", &mut family, &labels);
    card.family = FAMILIES.get(family).copied().unwrap_or(Family::Other);
    // A card with properties but no source carries an implicit one, so the user
    // never has to author literature to save a card.
    card.normalize_sources();
    ui_kit::section_count(ui, "physical:sources", tr("material.editor.sources"), false, Some(card.sources.len()), |ui| {
        let mut remove = None;
        for (i, source) in card.sources.iter_mut().enumerate() {
            ui_kit::section(ui, format!("physical:source:{i}"), &format!("{} {}", tr("ui.material_review.source"), i + 1), true, |ui| {
                // Stable local references remain intact while source claims are edited.
                ui_kit::row(ui, tr("material.editor.source_id"), |ui| ui_kit::text(ui, &source.id, ui_kit::Text::MonoCell));
                field(ui, tr("material.editor.source_title"), &format!("source:{i}:title"), &mut source.title, "");
                let mut selected = SOURCE_KINDS.iter().position(|k| k == &source.kind).unwrap_or(0);
                let labels = [
                    "material.editor.user_supplied",
                    "material.editor.measurement",
                    "material.editor.public_reference",
                    "material.editor.illustrative",
                ]
                .map(tr);
                ui_kit::choice_row(ui, tr("material.editor.source_kind"), &format!("source:{i}:kind"), &mut selected, &labels);
                source.kind = SOURCE_KINDS.get(selected).copied().unwrap_or(SourceKind::UserSupplied);
                optional_text(ui, tr("ui.design_properties.license"), &format!("source:{i}:license"), &mut source.license);
                optional_text(ui, tr("material.editor.reference"), &format!("source:{i}:reference"), &mut source.reference);
                let used = card.properties.iter().any(|p| p.source_id == source.id);
                if ui_kit::button(ui, tr("Remove"), ui_kit::Button::Enabled(!used)).clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            card.sources.remove(i);
        }
        if ui_kit::button(ui, tr("material.editor.add_source"), ui_kit::Button::Enabled(card.sources.len() < 32)).clicked() {
            let id = (1..=33).map(|i| format!("source-{i}")).find(|id| card.sources.iter().all(|s| &s.id != id)).unwrap_or_default();
            // The title is prefilled so a new source is usable; the user rewrites it.
            let title = tr("material.editor.default_source_title").to_string();
            card.sources.push(Source { id, title, kind: SourceKind::UserSupplied, license: None, reference: None });
        }
    });
    let mut remove = None;
    for (i, property) in card.properties.iter_mut().enumerate() {
        let label = format!("ui.material_review.{}", property.kind);
        ui_kit::section(ui, format!("physical:property:{}", property.kind), tr(&label), false, |ui| {
            property_rows(ui, property, &card.sources, assigned_kg_m3);
            if ui_kit::button(ui, tr("Remove"), ui_kit::Button::Normal).clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        card.properties.remove(i);
    }
    // A card holds each property kind once: the dropdown offers what is missing.
    let remaining = card.remaining_kinds();
    if remaining.is_empty() {
        ui_kit::text(ui, tr("material.editor.all_properties_added"), ui_kit::Text::Note("physical_add_kind"));
    } else {
        let labels: Vec<_> = remaining.iter().map(|kind| tr(&format!("ui.material_review.{kind}")).to_string()).collect();
        let refs: Vec<_> = labels.iter().map(String::as_str).collect();
        let mut index = remaining.iter().position(|kind| *kind == card.add_kind).unwrap_or(0);
        ui_kit::choice_row(ui, tr("material.editor.add_property"), "physical:add_kind", &mut index, &refs);
        card.add_kind = remaining.get(index).copied().unwrap_or_else(|| remaining.first().copied().unwrap_or("density"));
        if ui_kit::button(ui, tr("Add"), ui_kit::Button::Enabled(true)).clicked() {
            card.add_property(card.add_kind, assigned_kg_m3);
        }
    }
}

pub(crate) fn rows(app: &mut SolveApp, ui: &mut egui::Ui, index: &mut usize, editor: &mut Editor) {
    let names = app.session.doc.material_names();
    if editor.draft.is_none() {
        let placeholder = if editor.mixed_materials { "material.selection.mixed" } else { "material.selection.choose" };
        let current = names.get(*index).map(String::as_str).unwrap_or_else(|| tr(placeholder));
        ui_kit::row(ui, "Material", |ui| {
            ui_kit::dropdown_menu(ui, "mat", current, |ui| {
                for (i, name) in names.iter().enumerate() {
                    let option = ui_kit::list_picker(ui, &format!("physical_material_{i}"), name, *index == i);
                    crate::scenario::publish_handle(&format!("dialog:Material:{name}"), option.rect.center());
                    if option.clicked() {
                        *index = i;
                        editor.chosen_material = Some(name.clone());
                        editor.observed_index = Some(i);
                        ui.close();
                    }
                }
            });
        });
        let selected = names.get(*index).map(String::as_str);
        ui.horizontal_wrapped(|ui| {
            let new = ui_kit::button(ui, tr("New"), ui_kit::Button::Normal);
            crate::scenario::publish_handle("material:new", new.rect.center());
            if new.clicked() {
                editor.begin(&app.session.doc, None, false);
            }
            let duplicate = ui_kit::button(ui, tr("material.editor.duplicate"), ui_kit::Button::Enabled(selected.is_some()));
            crate::scenario::publish_handle("material:duplicate", duplicate.rect.center());
            if duplicate.clicked() {
                editor.begin(&app.session.doc, selected, true);
            }
            let edit =
                ui_kit::button(ui, tr("Edit"), ui_kit::Button::Enabled(selected.and_then(|name| app.session.doc.user_material(name)).is_some()));
            crate::scenario::publish_handle("material:edit", edit.rect.center());
            if edit.clicked() {
                editor.begin(&app.session.doc, selected, false);
            }
        });
        if editor.draft.is_none()
            && let Some(name) = selected
        {
            crate::material_card_review::show_saved_material(ui, &app.session.doc, name);
        }
    }
    let mut save = false;
    let mut discard = false;
    if let Some(draft) = &mut editor.draft {
        ui_kit::text(ui, tr("material.editor.scope"), ui_kit::Text::Hint("physical_edit"));
        field(ui, tr("Name"), "name", &mut draft.name, "");
        field(ui, tr("material.editor.assigned_density"), "density", &mut draft.density, "g/cm³");
        if let Some(look) = &draft.copied_appearance {
            ui_kit::text(ui, &format!("{}: {}", tr("material.editor.preserved_appearance"), look.name), ui_kit::Text::Note("physical_appearance"));
        }
        ui_kit::checkbox(ui, tr("material.editor.engineering_card"), &mut draft.card_enabled);
        if draft.card_enabled {
            // The designation follows the material name until the user writes their own.
            let material_name = draft.name.trim().to_string();
            if !draft.card.designation_owned {
                draft.card.designation = material_name.clone();
            }
            let assigned_kg_m3 = assigned_density_kg_m3(&draft.density);
            card_rows(ui, &mut draft.card, assigned_kg_m3, &material_name);
            ui_kit::text(ui, tr("ui.material_review.claims"), ui_kit::Text::Note("physical_claims"));
        } else {
            ui_kit::text(ui, tr("material.editor.unknown_card"), ui_kit::Text::Note("unknown_card"));
        }
        let validation = draft.command(&app.session.doc);
        if let Err(error) = &validation {
            ui_kit::text(ui, error, ui_kit::Text::Report("physical_validation", true));
        }
        ui.horizontal_wrapped(|ui| {
            let button = ui_kit::button(ui, tr("ui.save"), ui_kit::Button::Enabled(validation.is_ok()));
            crate::scenario::publish_handle("material:save", button.rect.center());
            save = button.clicked();
            let button = ui_kit::button(ui, tr("Discard"), ui_kit::Button::Normal);
            crate::scenario::publish_handle("material:discard", button.rect.center());
            discard = button.clicked();
        });
    }
    if discard {
        editor.draft = None;
        editor.error = None;
    }
    if save && let Err(error) = editor.save(app, index) {
        editor.error = Some(error);
    }
    if let Some(error) = &editor.error {
        ui_kit::text(ui, error, ui_kit::Text::Report("physical_save_error", true));
    }
    if let Some(note) = &editor.note {
        ui_kit::text(ui, note, ui_kit::Text::Report("physical_saved", false));
    }
}
