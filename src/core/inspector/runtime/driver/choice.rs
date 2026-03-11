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
        _field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        DropdownBuilder::new()
            .width(px(144.0))
            .options(default_choice_options(theme))
            .disabled(true)
            .build(commands, theme)
    }

    fn serialize(&self, field: &dyn PartialReflect) -> Option<String> {
        let (options, selected) = read_choice_field(field, None)?;
        options.get(selected).cloned()
    }

    fn apply_serialized(&self, field: &mut dyn PartialReflect, raw: &str) -> bool {
        let Some((options, _)) = read_choice_field(field, None) else {
            return false;
        };
        let Some(selected) = options.iter().position(|option| option == raw) else {
            return false;
        };
        write_choice_field(field, selected, None)
    }

    fn install_runtime(&self, builder: &mut InspectorDriverRuntimeBuilder) {
        builder.on_apply(apply_inspector_choice_driver_changes);
        builder.on_sync(sync_inspector_choice_controls);
    }
}

fn apply_inspector_choice_driver_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<DropdownChange>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }
    for change in changes.read() {
        if !ctx.is_control(change.entity, INSPECTOR_DRIVER_CHOICE) {
            continue;
        }
        let _ = ctx.write_for(change.entity, |field| {
            write_choice_field(field, change.selected, None)
        });
    }
}

fn sync_inspector_choice_controls(
    ctx: InspectorDriverSyncContext,
    mut dropdown_controls: Query<(Entity, &mut Dropdown)>,
) {
    if !ctx.changed() {
        return;
    }

    for (entity, mut dropdown) in dropdown_controls.iter_mut() {
        if !ctx.is_control(entity, INSPECTOR_DRIVER_CHOICE) {
            continue;
        }
        if let Some((options, selected)) =
            ctx.read_for(entity, |field| read_choice_field(field, None))
        {
            dropdown.options = options;
            dropdown.selected = selected;
            dropdown.expanded = false;
            dropdown.disabled = false;
        } else {
            dropdown.options = default_choice_options(None);
            dropdown.selected = 0;
            dropdown.expanded = false;
            dropdown.disabled = true;
        }
    }
}
