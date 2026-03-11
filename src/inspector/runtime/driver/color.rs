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
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let entity = ColorFieldBuilder::new()
            .width(px(180.0))
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
        let color = read_color_field(field)?.to_srgba();
        Some(format!(
            "{},{},{},{}",
            color.red, color.green, color.blue, color.alpha
        ))
    }

    fn apply_serialized(
        &self,
        _editor: InspectorFieldEditor,
        field: &mut dyn PartialReflect,
        raw: &str,
        _numeric_min: Option<f32>,
        _theme: Option<&Theme>,
    ) -> bool {
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
    controls: Query<&InspectorControlBinding>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }
    for change in changes.read() {
        let Ok(control) = controls.get(change.entity) else {
            continue;
        };
        if control.editor.driver_id != INSPECTOR_DRIVER_COLOR {
            continue;
        }
        let _ = ctx.apply_to_binding(control, None, |field| write_color_field(field, change.color));
    }
}

fn sync_inspector_color_controls(
    ctx: InspectorDriverSyncContext,
    mut color_controls: Query<(&InspectorControlBinding, &mut ColorField)>,
) {
    if !ctx.changed() {
        return;
    }
    let Some(selection) = ctx.selection() else {
        for (_, mut field) in color_controls.iter_mut() {
            field.disabled = true;
        }
        return;
    };

    for (binding, mut field) in color_controls.iter_mut() {
        let source = selection.binding_source(binding);
        let Some(style_field) = source else {
            field.disabled = true;
            continue;
        };
        if binding.editor.driver_id != INSPECTOR_DRIVER_COLOR {
            field.disabled = true;
            continue;
        }
        if let Some(color) = read_color_field(style_field) {
            field.color = color;
            field.disabled = false;
        } else {
            field.disabled = true;
        }
    }
}
