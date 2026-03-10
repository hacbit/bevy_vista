use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

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
        let x_input = F32FieldBuilder::new()
            .kind(NumberKind::F32)
            .width(px(72.0))
            .height(px(28.0))
            .disabled(true)
            .build(commands, theme);
        let y_input = F32FieldBuilder::new()
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

    fn install_systems(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_inspector_vec2_numeric_changes.before(refresh_inspector_panel),
                sync_inspector_vec2_controls.after(sync_widget_property_section),
            )
                .run_if(in_state(crate::editor::VistaEditorInitPhase::Finalize)),
        );
    }
}

fn apply_inspector_vec2_numeric_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    mut changes: MessageReader<F32FieldChange>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    vec2_axis_inputs: Query<&InspectorVec2AxisInput>,
    vec2_controls: Query<&InspectorVec2Control>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
) {
    if options.is_preview_mode {
        changes.clear();
        return;
    }
    let Some(node_id) = panel_state.selected_node else {
        return;
    };

    for change in changes.read() {
        let Ok(input) = vec2_axis_inputs.get(change.entity) else {
            continue;
        };
        let Ok(control) = vec2_controls.get(input.owner) else {
            continue;
        };
        if matches!(control.target, InspectorBindingTarget::WidgetProp) {
            let Some(mut value) = selected_node_widget_reflect(
                &panel_state,
                &document,
                &widget_registry,
                &inspector_registry,
                &control_registry,
                None,
            ) else {
                continue;
            };
            let Some(field) = read_reflect_path_mut(value.as_mut(), &control.field_path) else {
                continue;
            };
            if !write_vec2_axis_field(field, input.axis, change.value as f32) {
                continue;
            }
            store_widget_prop_change(
                node_id,
                &control.field_path,
                InspectorFieldEditor::new(INSPECTOR_DRIVER_VEC2),
                field,
                &mut document,
                &widget_registry,
                &control_registry,
                None,
            );
            continue;
        }
        let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone()) else {
            continue;
        };
        let style_reflect: &mut dyn PartialReflect = &mut style;
        let Some(field) = read_reflect_path_mut(style_reflect, &control.field_path) else {
            continue;
        };
        if !write_vec2_axis_field(field, input.axis, change.value as f32) {
            continue;
        }
        apply_style_change(node_id, style, &mut document, &widget_registry);
    }
}

fn sync_inspector_vec2_controls(
    panel_state: Res<InspectorPanelState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    vec2_controls: Query<&InspectorVec2Control>,
    mut value_fields: Query<&mut F32Field>,
) {
    if !panel_state.is_changed() && !document.is_changed() {
        return;
    }

    let Some(style) = selected_node_style(&panel_state, &document) else {
        for control in vec2_controls.iter() {
            if let Ok(mut field) = value_fields.get_mut(control.x_input) {
                field.value = 0.0;
                field.disabled = true;
            }
            if let Ok(mut field) = value_fields.get_mut(control.y_input) {
                field.value = 0.0;
                field.disabled = true;
            }
        }
        return;
    };
    let widget_reflect = selected_node_widget_reflect(
        &panel_state,
        &document,
        &widget_registry,
        &inspector_registry,
        &control_registry,
        None,
    );

    for control in vec2_controls.iter() {
        let source: Option<&dyn PartialReflect> = match control.target {
            InspectorBindingTarget::Style => {
                read_reflect_path(style as &dyn PartialReflect, &control.field_path)
            }
            InspectorBindingTarget::WidgetProp => widget_reflect
                .as_deref()
                .and_then(|value| read_reflect_path(value, &control.field_path)),
        };
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
                field.kind = NumberKind::F32;
                field.value = value.x as f64;
                field.disabled = false;
            }
            if let Ok(mut field) = value_fields.get_mut(control.y_input) {
                field.kind = NumberKind::F32;
                field.value = value.y as f64;
                field.disabled = false;
            }
        }
    }
}
