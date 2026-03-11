use std::sync::Arc;

use bevy::prelude::*;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(ValInspectorDriver)
}

struct ValInspectorDriver;

fn retarget_val_control(commands: &mut Commands, control: Entity, target: InspectorBindingTarget) {
    commands
        .entity(control)
        .entry::<InspectorValControl>()
        .and_modify(move |mut binding| {
            binding.target = target.clone();
        });
}

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
            InspectorValControl {
                field_path: field.field_path.clone(),
                numeric_min: field.numeric_min,
                target: InspectorBindingTarget::Style,
                value_input,
                unit_input,
            },
        ))
        .add_children(&[value_input, unit_input])
        .id();
    commands
        .entity(value_input)
        .insert(InspectorValValueInput { owner });
    commands
        .entity(unit_input)
        .insert(InspectorValUnitInput { owner });
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

    fn retarget_control(
        &self,
        commands: &mut Commands,
        control: Entity,
        target: InspectorBindingTarget,
    ) {
        retarget_val_control(commands, control, target);
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
    value_inputs: Query<&InspectorValValueInput>,
    val_controls: Query<&InspectorValControl>,
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
        let _ = ctx.apply_to_field(
            &control.target,
            &control.field_path,
            InspectorFieldEditor::new(INSPECTOR_DRIVER_VAL),
            None,
            |field| write_val_number_field(field, value, control.numeric_min),
        );
    }
}

fn apply_inspector_val_dropdown_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<DropdownChange>,
    unit_inputs: Query<&InspectorValUnitInput>,
    val_controls: Query<&InspectorValControl>,
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
        let _ = ctx.apply_to_field(
            &control.target,
            &control.field_path,
            InspectorFieldEditor::new(INSPECTOR_DRIVER_VAL),
            None,
            |field| write_val_unit_field(field, change.selected, control.numeric_min),
        );
    }
}

fn sync_inspector_val_controls(
    ctx: InspectorDriverSyncContext,
    val_controls: Query<&InspectorValControl>,
    mut value_fields: Query<&mut NumberField>,
    mut unit_dropdowns: Query<&mut Dropdown>,
) {
    if !ctx.changed() {
        return;
    }
    let Some(selection) = ctx.selection() else {
        for control in val_controls.iter() {
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
        }
        return;
    };

    for control in val_controls.iter() {
        let source = selection.source(&control.target, &control.field_path);
        let Some(style_field) = source else {
            if let Ok(mut field) = value_fields.get_mut(control.value_input) {
                field.disabled = true;
            }
            if let Ok(mut dropdown) = unit_dropdowns.get_mut(control.unit_input) {
                dropdown.disabled = true;
            }
            continue;
        };
        if let Some((value, selected, number_enabled)) = read_val_field(style_field) {
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
}
