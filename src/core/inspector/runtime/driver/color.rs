use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(ColorInspectorDriver)
}

struct ColorInspectorDriver;

impl InspectorDriver for ColorInspectorDriver {
    fn id(&self) -> InspectorDriverId {
        INSPECTOR_DRIVER_COLOR
    }

    fn build(
        &self,
        commands: &mut Commands,
        _field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        ColorFieldBuilder::new()
            .width(px(180.0))
            .disabled(true)
            .build(commands, theme)
    }

    fn serialize(&self, field: &dyn PartialReflect) -> Option<String> {
        let color = read_color_field(field)?.to_srgba();
        Some(format!(
            "{},{},{},{}",
            color.red, color.green, color.blue, color.alpha
        ))
    }

    fn apply_serialized(&self, field: &mut dyn PartialReflect, raw: &str) -> bool {
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

    fn install_runtime(&self, builder: &mut InspectorDriverRuntimeBuilder) {
        builder.on_apply(apply_inspector_color_driver_changes);
        builder.on_sync(sync_inspector_color_controls);
    }
}

fn apply_inspector_color_driver_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<ColorFieldChange>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }
    for change in changes.read() {
        if !ctx.is_control(change.entity, INSPECTOR_DRIVER_COLOR) {
            continue;
        }
        let _ = ctx.write_for(change.entity, |field| {
            write_color_field(field, change.color)
        });
    }
}

fn sync_inspector_color_controls(
    ctx: InspectorDriverSyncContext,
    mut color_controls: Query<(Entity, &mut ColorField)>,
) {
    if !ctx.changed() {
        return;
    }

    for (entity, mut field) in color_controls.iter_mut() {
        if !ctx.is_control(entity, INSPECTOR_DRIVER_COLOR) {
            continue;
        }
        if let Some(color) = ctx.read_for(entity, read_color_field) {
            field.color = color;
            field.disabled = false;
        } else {
            field.disabled = true;
        }
    }
}
