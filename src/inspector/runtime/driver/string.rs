use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(StringInspectorDriver)
}

struct StringInspectorDriver;

impl InspectorDriver for StringInspectorDriver {
    fn id(&self) -> InspectorDriverId {
        INSPECTOR_DRIVER_STRING
    }

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let entity = TextFieldBuilder::new()
            .width(px(180.0))
            .height(px(28.0))
            .disabled(true)
            .build(commands, theme);
        commands.entity(entity).insert(InspectorControlBinding {
            field_path: field.field_path.clone(),
            editor: field.editor,
            target: InspectorBindingTarget::Style,
        });
        entity
    }

    fn serialize(
        &self,
        _editor: InspectorFieldEditor,
        field: &dyn PartialReflect,
        _theme: Option<&Theme>,
    ) -> Option<String> {
        read_string_field(field)
    }

    fn apply_serialized(
        &self,
        _editor: InspectorFieldEditor,
        field: &mut dyn PartialReflect,
        raw: &str,
        _numeric_min: Option<f32>,
        _theme: Option<&Theme>,
    ) -> bool {
        write_string_field(field, raw.to_owned())
    }

    fn install_runtime(&self, builder: &mut InspectorDriverRuntimeBuilder) {
        builder.on_apply(apply_inspector_string_changes);
        builder.on_sync(sync_inspector_string_controls);
    }
}

fn apply_inspector_string_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<TextInputChange>,
    mut submits: MessageReader<TextInputSubmit>,
    controls: Query<&InspectorControlBinding>,
) {
    if !ctx.can_edit() {
        changes.clear();
        submits.clear();
        return;
    }
    let events = changes
        .read()
        .map(|event| (event.entity, event.value.clone()))
        .chain(submits.read().map(|event| (event.entity, event.value.clone())));

    for (entity, value) in events {
        let Ok(control) = controls.get(entity) else {
            continue;
        };
        if control.editor.driver_id != INSPECTOR_DRIVER_STRING {
            continue;
        }
        let _ = ctx.apply_to_binding(control, None, |field| write_string_field(field, value.clone()));
    }
}

fn sync_inspector_string_controls(
    ctx: InspectorDriverSyncContext,
    mut string_controls: Query<(&InspectorControlBinding, &mut TextField)>,
) {
    if !ctx.changed() {
        return;
    }
    let Some(selection) = ctx.selection() else {
        for (_, mut field) in string_controls.iter_mut() {
            field.disabled = true;
        }
        return;
    };

    for (binding, mut field) in string_controls.iter_mut() {
        let source = selection.binding_source(binding);
        let Some(style_field) = source else {
            field.disabled = true;
            continue;
        };
        if binding.editor.driver_id != INSPECTOR_DRIVER_STRING {
            field.disabled = true;
            continue;
        }
        if let Some(value) = read_string_field(style_field) {
            if field.value != value {
                field.value = value;
                field.cursor_pos = field.value.chars().count();
                field.selection = None;
            }
            field.disabled = false;
        } else {
            field.disabled = true;
        }
    }
}
