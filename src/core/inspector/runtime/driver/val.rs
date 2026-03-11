use std::sync::Arc;

use bevy::prelude::*;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(ValInspectorDriver)
}

struct ValInspectorDriver;

#[derive(Component)]
struct ValControlRoot {
    numeric_min: Option<f32>,
    value_input: Entity,
    unit_input: Entity,
}

#[derive(Component)]
struct ValValuePart;

#[derive(Component)]
struct ValUnitPart;

fn build_val_control(
    commands: &mut Commands,
    field: &InspectorFieldDescriptor,
    theme: Option<&Theme>,
) -> Entity {
    let value_input = NumberFieldBuilder::new()
        .kind(NumberKind::F32)
        .width(px(84.0))
        .height(px(28.0))
        .disabled(true)
        .build(commands, theme);
    let unit_input = DropdownBuilder::new()
        .width(px(72.0))
        .options(val_unit_options())
        .disabled(true)
        .build(commands, theme);
    let owner = commands
        .spawn((
            Name::new(format!("Inspector {} Val Control", field.label)),
            Node {
                width: px(162.0),
                min_width: px(0.0),
                align_items: AlignItems::Center,
                column_gap: px(6.0),
                ..default()
            },
            ValControlRoot {
                numeric_min: field.numeric_min,
                value_input,
                unit_input,
            },
        ))
        .add_children(&[value_input, unit_input])
        .id();
    commands
        .entity(value_input)
        .insert((InspectorControlOwner { owner }, ValValuePart));
    commands
        .entity(unit_input)
        .insert((InspectorControlOwner { owner }, ValUnitPart));
    owner
}

impl InspectorDriver for ValInspectorDriver {
    fn id(&self) -> InspectorDriverId {
        INSPECTOR_DRIVER_VAL
    }

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        build_val_control(commands, field, theme)
    }
    fn install_runtime(&self, builder: &mut InspectorDriverRuntimeBuilder) {
        builder.on_apply(apply_inspector_val_numeric_changes);
        builder.on_apply(apply_inspector_val_dropdown_changes);
        builder.on_sync(sync_inspector_val_controls);
    }
}

fn apply_inspector_val_numeric_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<NumberFieldChange>,
    value_inputs: Query<&InspectorControlOwner, With<ValValuePart>>,
    val_controls: Query<&ValControlRoot>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }
    for change in changes.read() {
        let Ok(input) = value_inputs.get(change.entity) else {
            continue;
        };
        let Ok(control) = val_controls.get(input.owner) else {
            continue;
        };
        let Some(value) = change.value.cast::<f32>() else {
            continue;
        };
        let _ = ctx.write_for(change.entity, |field| {
            write_val_number_field(field, value, control.numeric_min)
        });
    }
}

fn apply_inspector_val_dropdown_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<DropdownChange>,
    unit_inputs: Query<&InspectorControlOwner, With<ValUnitPart>>,
    val_controls: Query<&ValControlRoot>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }
    for change in changes.read() {
        let Ok(input) = unit_inputs.get(change.entity) else {
            continue;
        };
        let Ok(control) = val_controls.get(input.owner) else {
            continue;
        };
        let _ = ctx.write_for(change.entity, |field| {
            write_val_unit_field(field, change.selected, control.numeric_min)
        });
    }
}

fn sync_inspector_val_controls(
    ctx: InspectorDriverSyncContext,
    val_controls: Query<(Entity, &ValControlRoot)>,
    mut value_fields: Query<&mut NumberField>,
    mut unit_dropdowns: Query<&mut Dropdown>,
) {
    if !ctx.changed() {
        return;
    }
    for (entity, control) in val_controls.iter() {
        if !ctx.is_control(entity, INSPECTOR_DRIVER_VAL) {
            continue;
        }

        let Some((value, selected, number_enabled)) = ctx.read_for(entity, read_val_field) else {
            if let Ok(mut field) = value_fields.get_mut(control.value_input) {
                field.value = Number::F32(0.0);
                field.disabled = true;
            }
            if let Ok(mut dropdown) = unit_dropdowns.get_mut(control.unit_input) {
                dropdown.options = val_unit_options();
                dropdown.selected = 0;
                dropdown.expanded = false;
                dropdown.disabled = true;
            }
            continue;
        };
        if let Ok(mut field) = value_fields.get_mut(control.value_input) {
            field.value = Number::F32(value);
            field.disabled = !number_enabled;
        }
        if let Ok(mut dropdown) = unit_dropdowns.get_mut(control.unit_input) {
            dropdown.options = val_unit_options();
            dropdown.selected = selected;
            dropdown.expanded = false;
            dropdown.disabled = false;
        }
    }
}
