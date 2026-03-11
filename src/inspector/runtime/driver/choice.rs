use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(ChoiceInspectorDriver)
}

struct ChoiceInspectorDriver;

impl InspectorDriver for ChoiceInspectorDriver {
    fn id(&self) -> InspectorDriverId {
        INSPECTOR_DRIVER_CHOICE
    }

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let entity = DropdownBuilder::new()
            .width(px(144.0))
            .options(default_choice_options(theme))
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
        theme: Option<&Theme>,
    ) -> Option<String> {
        let (options, selected) = read_choice_field(field, theme)?;
        options.get(selected).cloned()
    }

    fn apply_serialized(
        &self,
        _editor: InspectorFieldEditor,
        field: &mut dyn PartialReflect,
        raw: &str,
        _numeric_min: Option<f32>,
        theme: Option<&Theme>,
    ) -> bool {
        let Some((options, _)) = read_choice_field(field, theme) else {
            return false;
        };
        let Some(selected) = options.iter().position(|option| option == raw) else {
            return false;
        };
        write_choice_field(field, selected, theme)
    }

    fn install_runtime(&self, builder: &mut InspectorDriverRuntimeBuilder) {
        builder.on_apply(apply_inspector_choice_driver_changes);
        builder.on_sync(sync_inspector_choice_controls);
    }
}

fn apply_inspector_choice_driver_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<DropdownChange>,
    controls: Query<&InspectorControlBinding>,
    val_unit_inputs: Query<(), With<InspectorValUnitInput>>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }
    for change in changes.read() {
        if val_unit_inputs.contains(change.entity) {
            continue;
        }
        let Ok(control) = controls.get(change.entity) else {
            continue;
        };
        if control.editor.driver_id != INSPECTOR_DRIVER_CHOICE {
            continue;
        }
        let _ = ctx.apply_to_binding(control, None, |field| write_choice_field(field, change.selected, None));
    }
}

fn sync_inspector_choice_controls(
    ctx: InspectorDriverSyncContext,
    mut dropdown_controls: Query<(&InspectorControlBinding, &mut Dropdown)>,
) {
    if !ctx.changed() {
        return;
    }
    let Some(selection) = ctx.selection() else {
        for (binding, mut dropdown) in dropdown_controls.iter_mut() {
            if binding.editor.driver_id != INSPECTOR_DRIVER_CHOICE {
                continue;
            }
            dropdown.options = default_choice_options(None);
            dropdown.selected = 0;
            dropdown.expanded = false;
            dropdown.disabled = true;
        }
        return;
    };

    for (binding, mut dropdown) in dropdown_controls.iter_mut() {
        let source = selection.binding_source(binding);
        let Some(style_field) = source else {
            dropdown.disabled = true;
            continue;
        };
        if binding.editor.driver_id != INSPECTOR_DRIVER_CHOICE {
            dropdown.disabled = true;
            continue;
        }
        if let Some((options, selected)) = read_choice_field(style_field, None) {
            dropdown.options = options;
            dropdown.selected = selected;
            dropdown.expanded = false;
            dropdown.disabled = false;
        } else {
            dropdown.disabled = true;
        }
    }
}
