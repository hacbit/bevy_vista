use std::any::TypeId;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::reflect::{DynamicEnum, PartialReflect, ReflectRef, VariantType};

use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorResolvedEditor {
    Number(InspectorNumberAdapter),
    Bool(InspectorBoolAdapter),
    Choice(InspectorChoiceAdapter),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorNumberAdapter {
    F32,
    ValPx,
    UiRectAll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorBoolAdapter {
    Bool,
    Visibility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorChoiceAdapter {
    UnitEnum,
    ColorPreset,
}

#[derive(Clone, Debug, Default)]
pub struct InspectorFieldOptions {
    pub label: Option<String>,
    pub editor: Option<InspectorResolvedEditor>,
    pub hidden: bool,
    pub numeric_min: Option<f32>,
}

impl InspectorFieldOptions {
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn editor(mut self, editor: InspectorResolvedEditor) -> Self {
        self.editor = Some(editor);
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn numeric_min(mut self, min: f32) -> Self {
        self.numeric_min = Some(min);
        self
    }
}

#[derive(Clone, Debug)]
pub struct InspectorFieldMetadata {
    pub field_name: &'static str,
    pub options: InspectorFieldOptions,
}

#[derive(Clone, Debug)]
pub struct InspectorFieldDescriptor {
    pub name: String,
    pub label: String,
    pub editor: InspectorResolvedEditor,
    pub numeric_min: Option<f32>,
}

#[derive(Resource)]
pub struct InspectorEditorRegistry {
    type_editors: HashMap<TypeId, InspectorResolvedEditor>,
}

impl Default for InspectorEditorRegistry {
    fn default() -> Self {
        let mut registry = Self {
            type_editors: HashMap::default(),
        };

        registry.register_type_editor::<f32>(InspectorResolvedEditor::Number(
            InspectorNumberAdapter::F32,
        ));
        registry.register_type_editor::<Val>(InspectorResolvedEditor::Number(
            InspectorNumberAdapter::ValPx,
        ));
        registry.register_type_editor::<UiRect>(InspectorResolvedEditor::Number(
            InspectorNumberAdapter::UiRectAll,
        ));
        registry.register_type_editor::<bool>(InspectorResolvedEditor::Bool(
            InspectorBoolAdapter::Bool,
        ));

        registry
    }
}

impl InspectorEditorRegistry {
    pub fn register_type_editor<T: 'static>(&mut self, editor: InspectorResolvedEditor) {
        self.type_editors.insert(TypeId::of::<T>(), editor);
    }

    pub fn fields_for<T>(&self) -> Vec<InspectorFieldDescriptor>
    where
        T: Reflect + Default + ShowInInspector,
    {
        let value = T::default();
        let ReflectRef::Struct(reflected) = value.reflect_ref() else {
            return Vec::new();
        };
        let metadata = T::inspector_fields()
            .into_iter()
            .map(|field| (field.field_name, field.options))
            .collect::<HashMap<_, _>>();

        let mut fields = Vec::new();
        for index in 0..reflected.field_len() {
            let Some(name) = reflected.name_at(index) else {
                continue;
            };
            let Some(field) = reflected.field_at(index) else {
                continue;
            };
            let Some(descriptor) = self.resolve_field_descriptor(name, field, metadata.get(name))
            else {
                continue;
            };
            fields.push(descriptor);
        }
        fields
    }

    fn resolve_field_descriptor(
        &self,
        field_name: &str,
        field: &dyn PartialReflect,
        metadata: Option<&InspectorFieldOptions>,
    ) -> Option<InspectorFieldDescriptor> {
        if metadata.is_some_and(|value| value.hidden) {
            return None;
        }

        let editor = metadata
            .and_then(|value| value.editor)
            .or_else(|| self.editor_for_type(field))
            .or_else(|| self.editor_for_unit_enum(field))?;

        Some(InspectorFieldDescriptor {
            name: field_name.to_owned(),
            label: metadata
                .and_then(|value| value.label.clone())
                .unwrap_or_else(|| humanize_field_name(field_name)),
            editor,
            numeric_min: metadata.and_then(|value| value.numeric_min),
        })
    }

    fn editor_for_type(&self, field: &dyn PartialReflect) -> Option<InspectorResolvedEditor> {
        let type_id = field.get_represented_type_info()?.type_id();
        self.type_editors.get(&type_id).copied()
    }

    fn editor_for_unit_enum(&self, field: &dyn PartialReflect) -> Option<InspectorResolvedEditor> {
        let ReflectRef::Enum(value) = field.reflect_ref() else {
            return None;
        };
        let info = value.get_represented_enum_info()?;
        if info
            .iter()
            .all(|variant| variant.variant_type() == VariantType::Unit)
        {
            Some(InspectorResolvedEditor::Choice(
                InspectorChoiceAdapter::UnitEnum,
            ))
        } else {
            None
        }
    }
}

pub trait ShowInInspector {
    fn inspector_fields() -> Vec<InspectorFieldMetadata> {
        Vec::new()
    }
}

pub fn editor_from_key(key: &str) -> Option<InspectorResolvedEditor> {
    match key {
        "f32" => Some(InspectorResolvedEditor::Number(InspectorNumberAdapter::F32)),
        "val_px" => Some(InspectorResolvedEditor::Number(
            InspectorNumberAdapter::ValPx,
        )),
        "ui_rect_all" => Some(InspectorResolvedEditor::Number(
            InspectorNumberAdapter::UiRectAll,
        )),
        "bool" => Some(InspectorResolvedEditor::Bool(InspectorBoolAdapter::Bool)),
        "visibility" => Some(InspectorResolvedEditor::Bool(
            InspectorBoolAdapter::Visibility,
        )),
        "unit_enum" => Some(InspectorResolvedEditor::Choice(
            InspectorChoiceAdapter::UnitEnum,
        )),
        "color_preset" => Some(InspectorResolvedEditor::Choice(
            InspectorChoiceAdapter::ColorPreset,
        )),
        _ => None,
    }
}

pub fn read_number_field(
    adapter: InspectorNumberAdapter,
    field: &dyn PartialReflect,
) -> Option<f32> {
    match adapter {
        InspectorNumberAdapter::F32 => field.try_downcast_ref::<f32>().copied(),
        InspectorNumberAdapter::ValPx => match field.try_downcast_ref::<Val>()? {
            Val::Px(value) => Some(*value),
            _ => Some(0.0),
        },
        InspectorNumberAdapter::UiRectAll => rect_uniform_px(*field.try_downcast_ref::<UiRect>()?),
    }
}

pub fn write_number_field(
    adapter: InspectorNumberAdapter,
    field: &mut dyn PartialReflect,
    value: f32,
    numeric_min: Option<f32>,
) -> bool {
    let value = numeric_min.map_or(value, |min| value.max(min));
    match adapter {
        InspectorNumberAdapter::F32 => {
            let Some(target) = field.try_downcast_mut::<f32>() else {
                return false;
            };
            *target = value;
            true
        }
        InspectorNumberAdapter::ValPx => {
            let Some(target) = field.try_downcast_mut::<Val>() else {
                return false;
            };
            *target = Val::Px(value);
            true
        }
        InspectorNumberAdapter::UiRectAll => {
            let Some(target) = field.try_downcast_mut::<UiRect>() else {
                return false;
            };
            *target = UiRect::all(Val::Px(value));
            true
        }
    }
}

pub fn read_bool_field(adapter: InspectorBoolAdapter, field: &dyn PartialReflect) -> Option<bool> {
    match adapter {
        InspectorBoolAdapter::Bool => field.try_downcast_ref::<bool>().copied(),
        InspectorBoolAdapter::Visibility => {
            let visibility = field.try_downcast_ref::<Visibility>()?;
            Some(*visibility != Visibility::Hidden)
        }
    }
}

pub fn write_bool_field(
    adapter: InspectorBoolAdapter,
    field: &mut dyn PartialReflect,
    checked: bool,
) -> bool {
    match adapter {
        InspectorBoolAdapter::Bool => {
            let Some(target) = field.try_downcast_mut::<bool>() else {
                return false;
            };
            *target = checked;
            true
        }
        InspectorBoolAdapter::Visibility => {
            let Some(target) = field.try_downcast_mut::<Visibility>() else {
                return false;
            };
            *target = if checked {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            true
        }
    }
}

pub fn read_choice_field(
    adapter: InspectorChoiceAdapter,
    field: &dyn PartialReflect,
    theme: Option<&Theme>,
) -> Option<(Vec<String>, usize)> {
    match adapter {
        InspectorChoiceAdapter::UnitEnum => {
            let ReflectRef::Enum(value) = field.reflect_ref() else {
                return None;
            };
            let info = value.get_represented_enum_info()?;
            if !info
                .iter()
                .all(|variant| variant.variant_type() == VariantType::Unit)
            {
                return None;
            }
            Some((
                info.variant_names()
                    .iter()
                    .map(|name| humanize_variant_name(name))
                    .collect(),
                value.variant_index(),
            ))
        }
        InspectorChoiceAdapter::ColorPreset => {
            let color = field.try_downcast_ref::<Color>()?;
            let presets = background_presets(theme);
            Some((
                presets
                    .iter()
                    .map(|(label, _)| (*label).to_owned())
                    .collect(),
                presets
                    .iter()
                    .position(|(_, preset)| preset == color)
                    .unwrap_or(0),
            ))
        }
    }
}

pub fn write_choice_field(
    adapter: InspectorChoiceAdapter,
    field: &mut dyn PartialReflect,
    selected: usize,
    theme: Option<&Theme>,
) -> bool {
    match adapter {
        InspectorChoiceAdapter::UnitEnum => {
            let Some(type_info) = field.get_represented_type_info() else {
                return false;
            };
            let Ok(info) = type_info.as_enum() else {
                return false;
            };
            let Some(variant) = info.variant_at(selected) else {
                return false;
            };
            if variant.variant_type() != VariantType::Unit {
                return false;
            }
            let mut dynamic = DynamicEnum::new_with_index(selected, variant.name(), ());
            dynamic.set_represented_type(Some(type_info));
            field.apply(dynamic.as_partial_reflect());
            true
        }
        InspectorChoiceAdapter::ColorPreset => {
            let Some(target) = field.try_downcast_mut::<Color>() else {
                return false;
            };
            let presets = background_presets(theme);
            let index = selected.min(presets.len().saturating_sub(1));
            *target = presets[index].1;
            true
        }
    }
}

pub fn default_choice_options(
    adapter: InspectorChoiceAdapter,
    theme: Option<&Theme>,
) -> Vec<String> {
    match adapter {
        InspectorChoiceAdapter::UnitEnum => Vec::new(),
        InspectorChoiceAdapter::ColorPreset => background_presets(theme)
            .iter()
            .map(|(label, _)| (*label).to_owned())
            .collect(),
    }
}

fn rect_uniform_px(rect: UiRect) -> Option<f32> {
    match (rect.left, rect.right, rect.top, rect.bottom) {
        (Val::Px(left), Val::Px(right), Val::Px(top), Val::Px(bottom))
            if left == right && left == top && left == bottom =>
        {
            Some(left)
        }
        _ => Some(0.0),
    }
}

fn humanize_field_name(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut uppercase_next = true;
    for ch in name.chars() {
        if ch == '_' {
            result.push(' ');
            uppercase_next = true;
            continue;
        }
        if uppercase_next {
            result.extend(ch.to_uppercase());
            uppercase_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

fn humanize_variant_name(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for (index, ch) in name.chars().enumerate() {
        if index > 0 && ch.is_uppercase() {
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

fn background_presets(theme: Option<&Theme>) -> [(&'static str, Color); 5] {
    match theme {
        Some(t) => [
            ("None", Color::NONE),
            ("Surface", t.palette.surface),
            ("Variant", t.palette.surface_variant),
            ("PrimaryBox", t.palette.primary_container),
            ("Primary", t.palette.primary),
        ],
        None => [
            ("None", Color::NONE),
            ("Dark", Color::srgb(0.12, 0.12, 0.12)),
            ("Panel", Color::srgb(0.2, 0.2, 0.2)),
            ("AccentBox", Color::srgb(0.22, 0.35, 0.52)),
            ("Accent", Color::srgb(0.2, 0.6, 0.95)),
        ],
    }
}
