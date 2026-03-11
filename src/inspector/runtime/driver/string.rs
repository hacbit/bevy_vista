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
        _field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        TextFieldBuilder::new()
            .width(px(180.0))
            .height(px(28.0))
            .disabled(true)
            .build(commands, theme)
    }

    fn serialize(&self, field: &dyn PartialReflect) -> Option<String> {
        read_string_field(field)
    }

    fn apply_serialized(&self, field: &mut dyn PartialReflect, raw: &str) -> bool {
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
) {
    if !ctx.can_edit() {
        changes.clear();
        submits.clear();
        return;
    }
    let events = changes
        .read()
        .map(|event| (event.entity, event.value.clone()))
        .chain(
            submits
                .read()
                .map(|event| (event.entity, event.value.clone())),
        );

    for (entity, value) in events {
        if !ctx.is_control(entity, INSPECTOR_DRIVER_STRING) {
            continue;
        }
        let _ = ctx.write_for(entity, |field| write_string_field(field, value.clone()));
    }
}

fn sync_inspector_string_controls(
    ctx: InspectorDriverSyncContext,
    mut string_controls: Query<(Entity, &mut TextField)>,
) {
    if !ctx.changed() {
        return;
    }

    for (entity, mut field) in string_controls.iter_mut() {
        if !ctx.is_control(entity, INSPECTOR_DRIVER_STRING) {
            continue;
        }
        if let Some(value) = ctx.read_for(entity, read_string_field) {
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
