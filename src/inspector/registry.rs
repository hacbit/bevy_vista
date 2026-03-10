use std::any::TypeId;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::reflect::{PartialReflect, Reflect, ReflectRef, VariantType};

use crate::inspector::{
    inspector_metadata_for, InspectorEntryDescriptor, InspectorFieldDescriptor,
    InspectorFieldEditor, InspectorFieldOptions, InspectorHeaderDescriptor,
    InspectorTypeEditorResolver,
    INSPECTOR_DRIVER_BOOL, INSPECTOR_DRIVER_CHOICE, INSPECTOR_DRIVER_COLOR,
    INSPECTOR_DRIVER_NUMBER, INSPECTOR_DRIVER_STRING, INSPECTOR_DRIVER_VAL,
    INSPECTOR_DRIVER_VEC2,
};

#[derive(Resource)]
pub struct InspectorEditorRegistry {
    type_editors: HashMap<TypeId, InspectorFieldEditor>,
    type_resolvers: HashMap<TypeId, InspectorTypeEditorResolver>,
}

impl Default for InspectorEditorRegistry {
    fn default() -> Self {
        let mut registry = Self {
            type_editors: HashMap::default(),
            type_resolvers: HashMap::default(),
        };

        registry.register_type_driver::<i8>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<i16>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<i32>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<i64>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<isize>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<u8>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<u16>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<u32>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<u64>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<usize>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<f32>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<f64>(INSPECTOR_DRIVER_NUMBER);
        registry.register_type_driver::<String>(INSPECTOR_DRIVER_STRING);
        registry.register_type_driver::<Val>(INSPECTOR_DRIVER_VAL);
        registry.register_type_driver::<bool>(INSPECTOR_DRIVER_BOOL);
        registry.register_type_driver::<Color>(INSPECTOR_DRIVER_COLOR);
        registry.register_type_driver::<Vec2>(INSPECTOR_DRIVER_VEC2);
        registry.register_type_driver::<Rot2>(INSPECTOR_DRIVER_NUMBER);

        registry
    }
}

impl InspectorEditorRegistry {
    pub fn register_type_driver<T: 'static>(&mut self, driver_id: &'static str) {
        let type_id = TypeId::of::<T>();
        self.type_resolvers.remove(&type_id);
        self.type_editors
            .insert(type_id, InspectorFieldEditor::new(driver_id));
    }

    pub fn register_type_resolver<T: 'static>(
        &mut self,
        resolver: InspectorTypeEditorResolver,
    ) {
        let type_id = TypeId::of::<T>();
        self.type_editors.remove(&type_id);
        self.type_resolvers.insert(type_id, resolver);
    }

    pub fn entries_for<T>(&self) -> Vec<InspectorEntryDescriptor>
    where
        T: Reflect + Default + 'static,
    {
        let Some(field_metadata) = inspector_metadata_for::<T>() else {
            return Vec::new();
        };
        let value = T::default();
        let ReflectRef::Struct(reflected) = value.reflect_ref() else {
            return Vec::new();
        };
        let metadata = field_metadata
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

        let editor = self
            .editor_for_type(field)
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
            entries.extend(self.resolve_field_entries(&child_path, &child_label, child, None, true));
        }
        if auto_group_struct {
            entries.push(InspectorEntryDescriptor::EndHeader);
        }
        entries
    }

    fn editor_for_type(&self, field: &dyn PartialReflect) -> Option<InspectorFieldEditor> {
        let type_id = field.get_represented_type_info()?.type_id();
        if let Some(resolver) = self.type_resolvers.get(&type_id) {
            if let Some(editor) = resolver(field) {
                return Some(editor);
            }
        }
        self.type_editors.get(&type_id).copied()
    }

    fn editor_for_unit_enum(&self, field: &dyn PartialReflect) -> Option<InspectorFieldEditor> {
        let ReflectRef::Enum(value) = field.reflect_ref() else {
            return None;
        };
        let info = value.get_represented_enum_info()?;
        if info
            .iter()
            .all(|variant| variant.variant_type() == VariantType::Unit)
        {
            Some(InspectorFieldEditor::new(INSPECTOR_DRIVER_CHOICE))
        } else {
            None
        }
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
