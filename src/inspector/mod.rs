use std::any::TypeId;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::reflect::{DynamicEnum, PartialReflect, ReflectMut, ReflectRef, VariantType};

use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InspectorResolvedEditor {
    Number(InspectorNumberAdapter),
    String(InspectorStringAdapter),
    Bool(InspectorBoolAdapter),
    Choice(InspectorChoiceAdapter),
    Color(InspectorColorAdapter),
    Val(InspectorValAdapter),
    Vec2(InspectorVec2Adapter),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InspectorNumberAdapter {
    F32,
    Rot2Degrees,
    UiRectAll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InspectorStringAdapter {
    Text,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InspectorValAdapter {
    Val,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InspectorVec2Adapter {
    Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InspectorBoolAdapter {
    Bool,
    Visibility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InspectorChoiceAdapter {
    UnitEnum,
    ColorPreset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InspectorColorAdapter {
    Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InspectorDriverKey {
    Number,
    String,
    Bool,
    Choice,
    Color,
    Val,
    Vec2,
}

impl InspectorResolvedEditor {
    pub fn driver_key(self) -> InspectorDriverKey {
        match self {
            InspectorResolvedEditor::Number(_) => InspectorDriverKey::Number,
            InspectorResolvedEditor::String(_) => InspectorDriverKey::String,
            InspectorResolvedEditor::Bool(_) => InspectorDriverKey::Bool,
            InspectorResolvedEditor::Choice(_) => InspectorDriverKey::Choice,
            InspectorResolvedEditor::Color(_) => InspectorDriverKey::Color,
            InspectorResolvedEditor::Val(_) => InspectorDriverKey::Val,
            InspectorResolvedEditor::Vec2(_) => InspectorDriverKey::Vec2,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct InspectorFieldOptions {
    pub label: Option<String>,
    pub editor: Option<InspectorResolvedEditor>,
    pub hidden: bool,
    pub numeric_min: Option<f32>,
    pub header: Option<InspectorHeaderOptions>,
    pub end_header: bool,
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

    pub fn header(mut self, title: impl Into<String>) -> Self {
        self.header = Some(InspectorHeaderOptions::new(title));
        self
    }

    pub fn header_with_options(mut self, options: InspectorHeaderOptions) -> Self {
        self.header = Some(options);
        self
    }

    pub fn end_header(mut self, end_header: bool) -> Self {
        self.end_header = end_header;
        self
    }
}

#[derive(Clone, Debug)]
pub struct InspectorFieldMetadata {
    pub field_name: &'static str,
    pub options: InspectorFieldOptions,
}

#[derive(Clone, Debug)]
pub struct InspectorHeaderOptions {
    pub title: String,
    pub default_open: bool,
}

impl InspectorHeaderOptions {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            default_open: true,
        }
    }

    pub fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }
}

#[derive(Clone, Debug)]
pub struct InspectorHeaderDescriptor {
    pub title: String,
    pub default_open: bool,
    pub implicit_close_previous: bool,
}

#[derive(Clone, Debug)]
pub struct InspectorFieldDescriptor {
    pub field_path: String,
    pub label: String,
    pub editor: InspectorResolvedEditor,
    pub numeric_min: Option<f32>,
}

#[derive(Clone, Debug)]
pub enum InspectorEntryDescriptor {
    Header(InspectorHeaderDescriptor),
    Field(InspectorFieldDescriptor),
    EndHeader,
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
        registry.register_type_editor::<String>(InspectorResolvedEditor::String(
            InspectorStringAdapter::Text,
        ));
        registry
            .register_type_editor::<Val>(InspectorResolvedEditor::Val(InspectorValAdapter::Val));
        registry.register_type_editor::<bool>(InspectorResolvedEditor::Bool(
            InspectorBoolAdapter::Bool,
        ));
        registry.register_type_editor::<Color>(InspectorResolvedEditor::Color(
            InspectorColorAdapter::Color,
        ));
        registry.register_type_editor::<Vec2>(InspectorResolvedEditor::Vec2(
            InspectorVec2Adapter::Vec2,
        ));
        registry.register_type_editor::<Rot2>(InspectorResolvedEditor::Number(
            InspectorNumberAdapter::Rot2Degrees,
        ));

        registry
    }
}

impl InspectorEditorRegistry {
    pub fn register_type_editor<T: 'static>(&mut self, editor: InspectorResolvedEditor) {
        self.type_editors.insert(TypeId::of::<T>(), editor);
    }

    pub fn entries_for<T>(&self) -> Vec<InspectorEntryDescriptor>
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

        let mut entries = Vec::new();
        for index in 0..reflected.field_len() {
            let Some(name) = reflected.name_at(index) else {
                continue;
            };
            let Some(field) = reflected.field_at(index) else {
                continue;
            };
            let field_metadata = metadata.get(name);
            if let Some(header) = field_metadata.and_then(|value| value.header.as_ref()) {
                entries.push(InspectorEntryDescriptor::Header(
                    InspectorHeaderDescriptor {
                        title: header.title.clone(),
                        default_open: header.default_open,
                        implicit_close_previous: true,
                    },
                ));
            }

            let label = field_metadata
                .and_then(|value| value.label.clone())
                .unwrap_or_else(|| humanize_field_name(name));
            entries.extend(
                self.resolve_field_entries(
                    name,
                    &label,
                    field,
                    field_metadata,
                    field_metadata
                        .and_then(|value| value.header.as_ref())
                        .is_none(),
                ),
            );

            if field_metadata.is_some_and(|value| value.end_header) {
                entries.push(InspectorEntryDescriptor::EndHeader);
            }
        }
        entries
    }

    fn resolve_field_entries(
        &self,
        field_path: &str,
        label: &str,
        field: &dyn PartialReflect,
        metadata: Option<&InspectorFieldOptions>,
        auto_group_struct: bool,
    ) -> Vec<InspectorEntryDescriptor> {
        if metadata.is_some_and(|value| value.hidden) {
            return Vec::new();
        }

        let editor = metadata
            .and_then(|value| value.editor)
            .or_else(|| self.editor_for_type(field))
            .or_else(|| self.editor_for_unit_enum(field));

        if let Some(editor) = editor {
            return vec![InspectorEntryDescriptor::Field(InspectorFieldDescriptor {
                field_path: field_path.to_owned(),
                label: label.to_owned(),
                editor,
                numeric_min: metadata.and_then(|value| value.numeric_min),
            })];
        }

        let ReflectRef::Struct(value) = field.reflect_ref() else {
            return Vec::new();
        };

        let mut entries = Vec::new();
        if auto_group_struct {
            entries.push(InspectorEntryDescriptor::Header(
                InspectorHeaderDescriptor {
                    title: label.to_owned(),
                    default_open: false,
                    implicit_close_previous: false,
                },
            ));
        }
        for index in 0..value.field_len() {
            let Some(child_name) = value.name_at(index) else {
                continue;
            };
            let Some(child) = value.field_at(index) else {
                continue;
            };
            let child_path = format!("{field_path}.{child_name}");
            let child_label = humanize_field_name(child_name);
            entries.extend(self.resolve_field_entries(
                &child_path,
                &child_label,
                child,
                None,
                true,
            ));
        }
        if auto_group_struct {
            entries.push(InspectorEntryDescriptor::EndHeader);
        }
        entries
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
        "string" | "text" => Some(InspectorResolvedEditor::String(
            InspectorStringAdapter::Text,
        )),
        "val_px" | "val" => Some(InspectorResolvedEditor::Val(InspectorValAdapter::Val)),
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
        "color_preset" | "color" => {
            Some(InspectorResolvedEditor::Color(InspectorColorAdapter::Color))
        }
        _ => None,
    }
}

pub fn read_string_field(
    _adapter: InspectorStringAdapter,
    field: &dyn PartialReflect,
) -> Option<String> {
    field.try_downcast_ref::<String>().cloned()
}

pub fn write_string_field(
    _adapter: InspectorStringAdapter,
    field: &mut dyn PartialReflect,
    value: String,
) -> bool {
    let Some(target) = field.try_downcast_mut::<String>() else {
        return false;
    };
    *target = value;
    true
}

pub fn read_color_field(
    _adapter: InspectorColorAdapter,
    field: &dyn PartialReflect,
) -> Option<Color> {
    field.try_downcast_ref::<Color>().copied()
}

pub fn write_color_field(
    _adapter: InspectorColorAdapter,
    field: &mut dyn PartialReflect,
    color: Color,
) -> bool {
    let Some(target) = field.try_downcast_mut::<Color>() else {
        return false;
    };
    *target = color;
    true
}

pub fn read_number_field(
    adapter: InspectorNumberAdapter,
    field: &dyn PartialReflect,
) -> Option<f32> {
    match adapter {
        InspectorNumberAdapter::F32 => field.try_downcast_ref::<f32>().copied(),
        InspectorNumberAdapter::Rot2Degrees => Some(field.try_downcast_ref::<Rot2>()?.as_degrees()),
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
        InspectorNumberAdapter::Rot2Degrees => {
            let Some(target) = field.try_downcast_mut::<Rot2>() else {
                return false;
            };
            *target = Rot2::degrees(value);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorValUnit {
    Auto,
    Px,
    Percent,
    Vw,
    Vh,
    VMin,
    VMax,
}

pub fn val_unit_options() -> Vec<String> {
    ["Auto", "Px", "%", "Vw", "Vh", "VMin", "VMax"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

pub fn read_val_field(
    _adapter: InspectorValAdapter,
    field: &dyn PartialReflect,
) -> Option<(f32, usize, bool)> {
    let (value, unit) = match field.try_downcast_ref::<Val>()? {
        Val::Auto => (0.0, InspectorValUnit::Auto),
        Val::Px(value) => (*value, InspectorValUnit::Px),
        Val::Percent(value) => (*value, InspectorValUnit::Percent),
        Val::Vw(value) => (*value, InspectorValUnit::Vw),
        Val::Vh(value) => (*value, InspectorValUnit::Vh),
        Val::VMin(value) => (*value, InspectorValUnit::VMin),
        Val::VMax(value) => (*value, InspectorValUnit::VMax),
    };
    Some((value, val_unit_index(unit), unit != InspectorValUnit::Auto))
}

pub fn write_val_number_field(
    _adapter: InspectorValAdapter,
    field: &mut dyn PartialReflect,
    value: f32,
    numeric_min: Option<f32>,
) -> bool {
    let value = numeric_min.map_or(value, |min| value.max(min));
    let Some(target) = field.try_downcast_mut::<Val>() else {
        return false;
    };
    *target = match *target {
        Val::Auto => Val::Auto,
        Val::Px(_) => Val::Px(value),
        Val::Percent(_) => Val::Percent(value),
        Val::Vw(_) => Val::Vw(value),
        Val::Vh(_) => Val::Vh(value),
        Val::VMin(_) => Val::VMin(value),
        Val::VMax(_) => Val::VMax(value),
    };
    true
}

pub fn write_val_unit_field(
    _adapter: InspectorValAdapter,
    field: &mut dyn PartialReflect,
    selected: usize,
    numeric_min: Option<f32>,
) -> bool {
    let Some(target) = field.try_downcast_mut::<Val>() else {
        return false;
    };
    let value = match *target {
        Val::Auto => numeric_min.unwrap_or(0.0).max(0.0),
        Val::Px(value)
        | Val::Percent(value)
        | Val::Vw(value)
        | Val::Vh(value)
        | Val::VMin(value)
        | Val::VMax(value) => numeric_min.map_or(value, |min| value.max(min)),
    };
    *target = match val_unit_from_index(selected) {
        InspectorValUnit::Auto => Val::Auto,
        InspectorValUnit::Px => Val::Px(value),
        InspectorValUnit::Percent => Val::Percent(value),
        InspectorValUnit::Vw => Val::Vw(value),
        InspectorValUnit::Vh => Val::Vh(value),
        InspectorValUnit::VMin => Val::VMin(value),
        InspectorValUnit::VMax => Val::VMax(value),
    };
    true
}

pub fn read_vec2_field(_adapter: InspectorVec2Adapter, field: &dyn PartialReflect) -> Option<Vec2> {
    field.try_downcast_ref::<Vec2>().copied()
}

pub fn write_vec2_axis_field(
    _adapter: InspectorVec2Adapter,
    field: &mut dyn PartialReflect,
    axis: usize,
    value: f32,
) -> bool {
    let Some(target) = field.try_downcast_mut::<Vec2>() else {
        return false;
    };
    match axis {
        0 => target.x = value,
        1 => target.y = value,
        _ => return false,
    }
    true
}

pub fn read_reflect_path<'a>(
    value: &'a dyn PartialReflect,
    path: &str,
) -> Option<&'a dyn PartialReflect> {
    let mut current = value;
    for segment in path.split('.') {
        let ReflectRef::Struct(struct_value) = current.reflect_ref() else {
            return None;
        };
        current = struct_value.field(segment)?;
    }
    Some(current)
}

pub fn read_reflect_path_mut<'a>(
    value: &'a mut dyn PartialReflect,
    path: &str,
) -> Option<&'a mut dyn PartialReflect> {
    fn descend<'a>(
        current: &'a mut dyn PartialReflect,
        segments: &[&str],
    ) -> Option<&'a mut dyn PartialReflect> {
        if segments.is_empty() {
            return Some(current);
        }
        let ReflectMut::Struct(struct_value) = current.reflect_mut() else {
            return None;
        };
        let child = struct_value.field_mut(segments[0])?;
        descend(child, &segments[1..])
    }

    let segments = path.split('.').collect::<Vec<_>>();
    descend(value, &segments)
}

pub fn reflect_path_differs_from_default(
    value: &dyn PartialReflect,
    default_value: &dyn PartialReflect,
    path: &str,
) -> bool {
    let Some(current) = read_reflect_path(value, path) else {
        return false;
    };
    let Some(default_field) = read_reflect_path(default_value, path) else {
        return false;
    };
    !current.reflect_partial_eq(default_field).unwrap_or(false)
}

pub fn collect_non_default_serialized_fields(
    value: &dyn PartialReflect,
    default_value: &dyn PartialReflect,
    entries: &[InspectorEntryDescriptor],
    theme: Option<&Theme>,
) -> HashMap<String, String> {
    let mut result = HashMap::default();
    for entry in entries {
        let InspectorEntryDescriptor::Field(field) = entry else {
            continue;
        };
        if !reflect_path_differs_from_default(value, default_value, &field.field_path) {
            continue;
        }
        let Some(current) = read_reflect_path(value, &field.field_path) else {
            continue;
        };
        let Some(serialized) = serialize_editor_value(field.editor, current, theme) else {
            continue;
        };
        result.insert(field.field_path.clone(), serialized);
    }
    result
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

pub fn serialize_editor_value(
    editor: InspectorResolvedEditor,
    field: &dyn PartialReflect,
    theme: Option<&Theme>,
) -> Option<String> {
    match editor {
        InspectorResolvedEditor::Number(adapter) => {
            Some(read_number_field(adapter, field)?.to_string())
        }
        InspectorResolvedEditor::String(adapter) => read_string_field(adapter, field),
        InspectorResolvedEditor::Bool(adapter) => {
            Some(read_bool_field(adapter, field)?.to_string())
        }
        InspectorResolvedEditor::Choice(adapter) => {
            let (options, selected) = read_choice_field(adapter, field, theme)?;
            options.get(selected).cloned()
        }
        InspectorResolvedEditor::Color(adapter) => {
            let color = read_color_field(adapter, field)?.to_srgba();
            Some(format!(
                "{},{},{},{}",
                color.red, color.green, color.blue, color.alpha
            ))
        }
        InspectorResolvedEditor::Val(_adapter) => {
            let value = field.try_downcast_ref::<Val>()?;
            Some(match value {
                Val::Auto => "Auto".to_owned(),
                Val::Px(v) => format!("Px:{v}"),
                Val::Percent(v) => format!("Percent:{v}"),
                Val::Vw(v) => format!("Vw:{v}"),
                Val::Vh(v) => format!("Vh:{v}"),
                Val::VMin(v) => format!("VMin:{v}"),
                Val::VMax(v) => format!("VMax:{v}"),
            })
        }
        InspectorResolvedEditor::Vec2(adapter) => {
            let value = read_vec2_field(adapter, field)?;
            Some(format!("{},{}", value.x, value.y))
        }
    }
}

pub fn apply_serialized_editor_value(
    editor: InspectorResolvedEditor,
    field: &mut dyn PartialReflect,
    raw: &str,
    numeric_min: Option<f32>,
    theme: Option<&Theme>,
) -> bool {
    match editor {
        InspectorResolvedEditor::Number(adapter) => raw
            .parse::<f32>()
            .ok()
            .is_some_and(|value| write_number_field(adapter, field, value, numeric_min)),
        InspectorResolvedEditor::String(adapter) => {
            write_string_field(adapter, field, raw.to_owned())
        }
        InspectorResolvedEditor::Bool(adapter) => raw
            .parse::<bool>()
            .ok()
            .is_some_and(|checked| write_bool_field(adapter, field, checked)),
        InspectorResolvedEditor::Choice(adapter) => {
            let Some((options, _)) = read_choice_field(adapter, field, theme) else {
                return false;
            };
            let Some(selected) = options.iter().position(|option| option == raw) else {
                return false;
            };
            write_choice_field(adapter, field, selected, theme)
        }
        InspectorResolvedEditor::Color(adapter) => {
            let parts = raw.split(',').collect::<Vec<_>>();
            if parts.len() != 4 {
                return false;
            }
            let Ok(r) = parts[0].parse::<f32>() else {
                return false;
            };
            let Ok(g) = parts[1].parse::<f32>() else {
                return false;
            };
            let Ok(b) = parts[2].parse::<f32>() else {
                return false;
            };
            let Ok(a) = parts[3].parse::<f32>() else {
                return false;
            };
            write_color_field(adapter, field, Color::srgba(r, g, b, a))
        }
        InspectorResolvedEditor::Val(adapter) => {
            let Some(target) = field.try_downcast_mut::<Val>() else {
                return false;
            };
            if raw == "Auto" {
                *target = Val::Auto;
                return true;
            }
            let Some((unit, value)) = raw.split_once(':') else {
                return false;
            };
            let Ok(value) = value.parse::<f32>() else {
                return false;
            };
            *target = match unit {
                "Px" => Val::Px(value),
                "Percent" => Val::Percent(value),
                "Vw" => Val::Vw(value),
                "Vh" => Val::Vh(value),
                "VMin" => Val::VMin(value),
                "VMax" => Val::VMax(value),
                _ => return false,
            };
            if let Some(min) = numeric_min {
                let _ = write_val_number_field(adapter, field, value, Some(min));
            }
            true
        }
        InspectorResolvedEditor::Vec2(adapter) => {
            let Some((x, y)) = raw.split_once(',') else {
                return false;
            };
            let (Ok(x), Ok(y)) = (x.parse::<f32>(), y.parse::<f32>()) else {
                return false;
            };
            write_vec2_axis_field(adapter, field, 0, x)
                && write_vec2_axis_field(adapter, field, 1, y)
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

fn val_unit_index(unit: InspectorValUnit) -> usize {
    match unit {
        InspectorValUnit::Auto => 0,
        InspectorValUnit::Px => 1,
        InspectorValUnit::Percent => 2,
        InspectorValUnit::Vw => 3,
        InspectorValUnit::Vh => 4,
        InspectorValUnit::VMin => 5,
        InspectorValUnit::VMax => 6,
    }
}

fn val_unit_from_index(index: usize) -> InspectorValUnit {
    match index {
        0 => InspectorValUnit::Auto,
        1 => InspectorValUnit::Px,
        2 => InspectorValUnit::Percent,
        3 => InspectorValUnit::Vw,
        4 => InspectorValUnit::Vh,
        5 => InspectorValUnit::VMin,
        6 => InspectorValUnit::VMax,
        _ => InspectorValUnit::Px,
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
