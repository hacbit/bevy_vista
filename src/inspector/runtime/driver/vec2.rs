use std::sync::Arc;

use bevy::prelude::*;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(Vec2InspectorDriver)
}

struct Vec2InspectorDriver;

impl InspectorDriver for Vec2InspectorDriver {
    fn id(&self) -> InspectorDriverId {
        INSPECTOR_DRIVER_VEC2
    }

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let x_input = NumberFieldBuilder::new()
            .kind(NumberKind::F32)
            .width(px(72.0))
            .height(px(28.0))
            .disabled(true)
            .build(commands, theme);
        let y_input = NumberFieldBuilder::new()
            .kind(NumberKind::F32)
            .width(px(72.0))
            .height(px(28.0))
            .disabled(true)
            .build(commands, theme);
        let owner = commands
            .spawn((
                Name::new(format!("Inspector {} Vec2 Control", field.label)),
                Node {
                    width: px(154.0),
                    min_width: px(0.0),
                    align_items: AlignItems::Center,
                    column_gap: px(6.0),
                    ..default()
                },
                InspectorVec2Control {
                    field_path: field.field_path.clone(),
                    target: InspectorBindingTarget::Style,
                    x_input,
                    y_input,
                },
            ))
            .add_children(&[x_input, y_input])
            .id();
        commands
            .entity(x_input)
            .insert(InspectorVec2AxisInput { owner, axis: 0 });
        commands
            .entity(y_input)
            .insert(InspectorVec2AxisInput { owner, axis: 1 });
        owner
    }

    fn retarget_control(
        &self,
        commands: &mut Commands,
        control: Entity,
        target: InspectorBindingTarget,
    ) {
        commands
            .entity(control)
            .entry::<InspectorVec2Control>()
            .and_modify(move |mut binding| {
                binding.target = target.clone();
            });
    }

    fn install_runtime(&self, builder: &mut InspectorDriverRuntimeBuilder) {
        builder.on_apply(apply_inspector_vec2_numeric_changes);
        builder.on_sync(sync_inspector_vec2_controls);
    }
}

fn apply_inspector_vec2_numeric_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<NumberFieldChange>,
    axis_inputs: Query<&InspectorVec2AxisInput>,
    vec2_controls: Query<&InspectorVec2Control>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }
    for change in changes.read() {
        let Ok(input) = axis_inputs.get(change.entity) else {
            continue;
        };
        let Ok(control) = vec2_controls.get(input.owner) else {
            continue;
        };
        let Some(value) = change.value.cast::<f32>() else {
            continue;
        };
        let _ = ctx.apply_to_field(
            &control.target,
            &control.field_path,
            InspectorFieldEditor::new(INSPECTOR_DRIVER_VEC2),
            None,
            |field| write_vec2_axis_field(field, input.axis, value),
        );
    }
}

fn sync_inspector_vec2_controls(
    ctx: InspectorDriverSyncContext,
    vec2_controls: Query<&InspectorVec2Control>,
    mut value_fields: Query<&mut NumberField>,
) {
    if !ctx.changed() {
        return;
    }
    let Some(selection) = ctx.selection() else {
        for control in vec2_controls.iter() {
            if let Ok(mut field) = value_fields.get_mut(control.x_input) {
                field.value = Number::F32(0.0);
                field.disabled = true;
            }
            if let Ok(mut field) = value_fields.get_mut(control.y_input) {
                field.value = Number::F32(0.0);
                field.disabled = true;
            }
        }
        return;
    };

    for control in vec2_controls.iter() {
        let source = selection.source(&control.target, &control.field_path);
        let Some(style_field) = source else {
            if let Ok(mut field) = value_fields.get_mut(control.x_input) {
                field.disabled = true;
            }
            if let Ok(mut field) = value_fields.get_mut(control.y_input) {
                field.disabled = true;
            }
            continue;
        };
        if let Some(value) = read_vec2_field(style_field) {
            if let Ok(mut field) = value_fields.get_mut(control.x_input) {
                field.value = Number::F32(value.x);
                field.disabled = false;
            }
            if let Ok(mut field) = value_fields.get_mut(control.y_input) {
                field.value = Number::F32(value.y);
                field.disabled = false;
            }
        }
    }
}
