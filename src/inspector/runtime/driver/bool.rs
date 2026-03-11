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
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let entity = CheckboxBuilder::new().disabled(true).build(commands, theme);
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
        Some(read_bool_field(field)?.to_string())
    }

    fn apply_serialized(
        &self,
        _editor: InspectorFieldEditor,
        field: &mut dyn PartialReflect,
        raw: &str,
        _numeric_min: Option<f32>,
        _theme: Option<&Theme>,
    ) -> bool {
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
        if control.editor.driver_id != INSPECTOR_DRIVER_BOOL {
            continue;
        }
        let _ = ctx.apply_to_binding(control, None, |field| write_bool_field(field, change.checked));
    }
}

fn sync_inspector_bool_controls(
    ctx: InspectorDriverSyncContext,
    mut checkbox_controls: Query<(&InspectorControlBinding, &mut Checkbox)>,
) {
    if !ctx.changed() {
        return;
    }
    let Some(selection) = ctx.selection() else {
        for (_, mut checkbox) in checkbox_controls.iter_mut() {
            checkbox.checked = false;
            checkbox.disabled = true;
        }
        return;
    };

    for (binding, mut checkbox) in checkbox_controls.iter_mut() {
        let source = selection.binding_source(binding);
        let Some(style_field) = source else {
            checkbox.disabled = true;
            continue;
        };
        if binding.editor.driver_id != INSPECTOR_DRIVER_BOOL {
            checkbox.disabled = true;
            continue;
        }
        if let Some(checked) = read_bool_field(style_field) {
            checkbox.checked = checked;
            checkbox.disabled = false;
        } else {
            checkbox.disabled = true;
        }
    }
}
