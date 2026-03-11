use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(BoolInspectorDriver)
}

struct BoolInspectorDriver;

impl InspectorDriver for BoolInspectorDriver {
    fn id(&self) -> InspectorDriverId {
        INSPECTOR_DRIVER_BOOL
    }

    fn build(
        &self,
        commands: &mut Commands,
        _field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        CheckboxBuilder::new().disabled(true).build(commands, theme)
    }

    fn serialize(&self, field: &dyn PartialReflect) -> Option<String> {
        Some(read_bool_field(field)?.to_string())
    }

    fn apply_serialized(&self, field: &mut dyn PartialReflect, raw: &str) -> bool {
        raw.parse::<bool>()
            .ok()
            .is_some_and(|checked| write_bool_field(field, checked))
    }

    fn install_runtime(&self, builder: &mut InspectorDriverRuntimeBuilder) {
        builder.on_apply(apply_inspector_bool_driver_changes);
        builder.on_sync(sync_inspector_bool_controls);
    }
}

fn apply_inspector_bool_driver_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<CheckboxChange>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }

    for change in changes.read() {
        if !ctx.is_control(change.entity, INSPECTOR_DRIVER_BOOL) {
            continue;
        }
        let _ = ctx.write_for(change.entity, |field| {
            write_bool_field(field, change.checked)
        });
    }
}

fn sync_inspector_bool_controls(
    ctx: InspectorDriverSyncContext,
    mut checkbox_controls: Query<(Entity, &mut Checkbox)>,
) {
    if !ctx.changed() {
        return;
    }

    for (entity, mut checkbox) in checkbox_controls.iter_mut() {
        if !ctx.is_control(entity, INSPECTOR_DRIVER_BOOL) {
            continue;
        }
        if let Some(checked) = ctx.read_for(entity, read_bool_field) {
            checkbox.checked = checked;
            checkbox.disabled = false;
        } else {
            checkbox.checked = false;
            checkbox.disabled = true;
        }
    }
}
