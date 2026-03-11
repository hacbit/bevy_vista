use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::reflect::{DynamicEnum, PartialReflect, ReflectMut, ReflectRef, VariantType};

use crate::inspector::{
    INSPECTOR_DRIVER_BOOL, INSPECTOR_DRIVER_CHOICE, INSPECTOR_DRIVER_COLOR,
    INSPECTOR_DRIVER_NUMBER, INSPECTOR_DRIVER_STRING, INSPECTOR_DRIVER_VAL, INSPECTOR_DRIVER_VEC2,
    InspectorDriverId, InspectorEntryDescriptor, InspectorFieldEditor,
};
use crate::theme::Theme;
use crate::widget::input::{Number, NumberKind};

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

pub fn read_string_field(field: &dyn PartialReflect) -> Option<String> {
    field.try_downcast_ref::<String>().cloned()
}

pub fn write_string_field(field: &mut dyn PartialReflect, value: String) -> bool {
    let Some(target) = field.try_downcast_mut::<String>() else {
        return false;
    };
    *target = value;
    true
}

pub fn read_color_field(field: &dyn PartialReflect) -> Option<Color> {
    field.try_downcast_ref::<Color>().copied()
}

pub fn write_color_field(field: &mut dyn PartialReflect, color: Color) -> bool {
    let Some(target) = field.try_downcast_mut::<Color>() else {
        return false;
    };
    *target = color;
    true
}

pub fn number_kind_for_field(field: &dyn PartialReflect) -> Option<NumberKind> {
    if let Some(value) = field.try_downcast_ref::<Number>() {
        Some(value.kind())
    } else if field.try_downcast_ref::<i8>().is_some() {
        Some(NumberKind::I8)
    } else if field.try_downcast_ref::<i16>().is_some() {
        Some(NumberKind::I16)
    } else if field.try_downcast_ref::<i32>().is_some() {
        Some(NumberKind::I32)
    } else if field.try_downcast_ref::<i64>().is_some() {
        Some(NumberKind::I64)
    } else if field.try_downcast_ref::<isize>().is_some() {
        Some(NumberKind::Isize)
    } else if field.try_downcast_ref::<u8>().is_some() {
        Some(NumberKind::U8)
    } else if field.try_downcast_ref::<u16>().is_some() {
        Some(NumberKind::U16)
    } else if field.try_downcast_ref::<u32>().is_some() {
        Some(NumberKind::U32)
    } else if field.try_downcast_ref::<u64>().is_some() {
        Some(NumberKind::U64)
    } else if field.try_downcast_ref::<usize>().is_some() {
        Some(NumberKind::Usize)
    } else if field.try_downcast_ref::<f32>().is_some()
        || field.try_downcast_ref::<Rot2>().is_some()
    {
        Some(NumberKind::F32)
    } else if field.try_downcast_ref::<f64>().is_some() {
        Some(NumberKind::F64)
    } else {
        None
    }
}

pub fn read_number_field(field: &dyn PartialReflect) -> Option<Number> {
    if let Some(value) = field.try_downcast_ref::<Number>() {
        Some(*value)
    } else if let Some(value) = field.try_downcast_ref::<i8>() {
        Some(Number::I8(*value))
    } else if let Some(value) = field.try_downcast_ref::<i16>() {
        Some(Number::I16(*value))
    } else if let Some(value) = field.try_downcast_ref::<i32>() {
        Some(Number::I32(*value))
    } else if let Some(value) = field.try_downcast_ref::<i64>() {
        Some(Number::I64(*value))
    } else if let Some(value) = field.try_downcast_ref::<isize>() {
        Some(Number::Isize(*value))
    } else if let Some(value) = field.try_downcast_ref::<u8>() {
        Some(Number::U8(*value))
    } else if let Some(value) = field.try_downcast_ref::<u16>() {
        Some(Number::U16(*value))
    } else if let Some(value) = field.try_downcast_ref::<u32>() {
        Some(Number::U32(*value))
    } else if let Some(value) = field.try_downcast_ref::<u64>() {
        Some(Number::U64(*value))
    } else if let Some(value) = field.try_downcast_ref::<usize>() {
        Some(Number::Usize(*value))
    } else if let Some(value) = field.try_downcast_ref::<f32>() {
        Some(Number::F32(*value))
    } else if let Some(value) = field.try_downcast_ref::<f64>() {
        Some(Number::F64(*value))
    } else {
        Some(Number::F32(field.try_downcast_ref::<Rot2>()?.as_degrees()))
    }
}

pub fn parse_number_for_field(
    field: &dyn PartialReflect,
    raw: &str,
    numeric_min: Option<f32>,
) -> Option<Number> {
    let min = numeric_min.map(Number::F32);
    if let Some(value) = Number::deserialize(raw, min, None) {
        return Some(value);
    }
    let kind = number_kind_for_field(field)?;
    Number::parse_as(kind, raw, min, None)
}

pub fn write_number_field(
    field: &mut dyn PartialReflect,
    value: Number,
    numeric_min: Option<f32>,
) -> bool {
    let min = numeric_min.map(Number::F32);
    if let Some(target) = field.try_downcast_mut::<Number>() {
        *target = value.cast_to(value.kind(), min, None);
        return true;
    }

    let Some(kind) = number_kind_for_field(field) else {
        return false;
    };
    let value = value.cast_to(kind, min, None);
    if let Some(target) = field.try_downcast_mut::<i8>() {
        *target = match value {
            Number::I8(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<i16>() {
        *target = match value {
            Number::I16(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<i32>() {
        *target = match value {
            Number::I32(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<i64>() {
        *target = match value {
            Number::I64(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<isize>() {
        *target = match value {
            Number::Isize(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<u8>() {
        *target = match value {
            Number::U8(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<u16>() {
        *target = match value {
            Number::U16(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<u32>() {
        *target = match value {
            Number::U32(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<u64>() {
        *target = match value {
            Number::U64(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<usize>() {
        *target = match value {
            Number::Usize(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<f32>() {
        *target = match value {
            Number::F32(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<f64>() {
        *target = match value {
            Number::F64(value) => value,
            _ => unreachable!(),
        };
    } else if let Some(target) = field.try_downcast_mut::<Rot2>() {
        *target = match value {
            Number::F32(value) => Rot2::degrees(value),
            _ => unreachable!(),
        };
    } else {
        return false;
    }
    true
}

pub fn write_number_kind_field(
    field: &mut dyn PartialReflect,
    kind: NumberKind,
    numeric_min: Option<f32>,
) -> bool {
    let min = numeric_min.map(Number::F32);
    let Some(current) = read_number_field(field) else {
        return false;
    };
    if field.try_downcast_ref::<Number>().is_none() {
        return false;
    }
    write_number_field(field, current.cast_to(kind, min, None), numeric_min)
}

pub fn val_unit_options() -> Vec<String> {
    ["Auto", "Px", "%", "Vw", "Vh", "VMin", "VMax"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

pub fn read_val_field(field: &dyn PartialReflect) -> Option<(f32, usize, bool)> {
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

pub fn read_vec2_field(field: &dyn PartialReflect) -> Option<Vec2> {
    field.try_downcast_ref::<Vec2>().copied()
}

pub fn write_vec2_axis_field(field: &mut dyn PartialReflect, axis: usize, value: f32) -> bool {
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

pub fn read_bool_field(field: &dyn PartialReflect) -> Option<bool> {
    field.try_downcast_ref::<bool>().copied()
}

pub fn write_bool_field(field: &mut dyn PartialReflect, checked: bool) -> bool {
    let Some(target) = field.try_downcast_mut::<bool>() else {
        return false;
    };
    *target = checked;
    true
}

pub fn read_choice_field(
    field: &dyn PartialReflect,
    _theme: Option<&Theme>,
) -> Option<(Vec<String>, usize)> {
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

pub fn write_choice_field(
    field: &mut dyn PartialReflect,
    selected: usize,
    _theme: Option<&Theme>,
) -> bool {
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

pub fn serialize_val_field(field: &dyn PartialReflect) -> Option<String> {
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

pub fn apply_serialized_val_field(field: &mut dyn PartialReflect, raw: &str) -> bool {
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
    true
}

pub fn serialize_vec2_field(field: &dyn PartialReflect) -> Option<String> {
    let value = read_vec2_field(field)?;
    Some(format!("{},{}", value.x, value.y))
}

pub fn apply_serialized_vec2_field(field: &mut dyn PartialReflect, raw: &str) -> bool {
    let Some((x, y)) = raw.split_once(',') else {
        return false;
    };
    let (Ok(x), Ok(y)) = (x.parse::<f32>(), y.parse::<f32>()) else {
        return false;
    };
    write_vec2_axis_field(field, 0, x) && write_vec2_axis_field(field, 1, y)
}

pub fn default_choice_options(_theme: Option<&Theme>) -> Vec<String> {
    Vec::new()
}

pub fn serialize_editor_value(
    editor: InspectorFieldEditor,
    field: &dyn PartialReflect,
    theme: Option<&Theme>,
) -> Option<String> {
    match driver_id(editor) {
        INSPECTOR_DRIVER_NUMBER => Some(read_number_field(field)?.serialize()),
        INSPECTOR_DRIVER_STRING => read_string_field(field),
        INSPECTOR_DRIVER_BOOL => Some(read_bool_field(field)?.to_string()),
        INSPECTOR_DRIVER_CHOICE => {
            let (options, selected) = read_choice_field(field, theme)?;
            options.get(selected).cloned()
        }
        INSPECTOR_DRIVER_COLOR => {
            let color = read_color_field(field)?.to_srgba();
            Some(format!(
                "{},{},{},{}",
                color.red, color.green, color.blue, color.alpha
            ))
        }
        INSPECTOR_DRIVER_VAL => serialize_val_field(field),
        INSPECTOR_DRIVER_VEC2 => serialize_vec2_field(field),
        _ => None,
    }
}

pub fn apply_serialized_editor_value(
    editor: InspectorFieldEditor,
    field: &mut dyn PartialReflect,
    raw: &str,
    theme: Option<&Theme>,
) -> bool {
    match driver_id(editor) {
        INSPECTOR_DRIVER_NUMBER => parse_number_for_field(field, raw, None)
            .is_some_and(|value| write_number_field(field, value, None)),
        INSPECTOR_DRIVER_STRING => write_string_field(field, raw.to_owned()),
        INSPECTOR_DRIVER_BOOL => raw
            .parse::<bool>()
            .ok()
            .is_some_and(|checked| write_bool_field(field, checked)),
        INSPECTOR_DRIVER_CHOICE => {
            let Some((options, _)) = read_choice_field(field, theme) else {
                return false;
            };
            let Some(selected) = options.iter().position(|option| option == raw) else {
                return false;
            };
            write_choice_field(field, selected, theme)
        }
        INSPECTOR_DRIVER_COLOR => {
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
            write_color_field(field, Color::srgba(r, g, b, a))
        }
        INSPECTOR_DRIVER_VAL => apply_serialized_val_field(field, raw),
        INSPECTOR_DRIVER_VEC2 => apply_serialized_vec2_field(field, raw),
        _ => false,
    }
}

fn driver_id(editor: InspectorFieldEditor) -> InspectorDriverId {
    editor.driver_id
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
