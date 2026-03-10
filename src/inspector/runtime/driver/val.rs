use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

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

    fn install_systems(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_inspector_val_numeric_changes.before(refresh_inspector_panel),
                apply_inspector_val_dropdown_changes.before(refresh_inspector_panel),
                sync_inspector_val_controls.after(sync_widget_property_section),
            )
                .run_if(in_state(crate::editor::VistaEditorInitPhase::Finalize)),
        );
    }
}

fn apply_inspector_val_numeric_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    mut changes: MessageReader<NumberFieldChange>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    val_value_inputs: Query<&InspectorValValueInput>,
    val_controls: Query<&InspectorValControl>,
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
        let Ok(input) = val_value_inputs.get(change.entity) else {
            continue;
        };
        let Ok(control) = val_controls.get(input.owner) else {
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
            let Some(value) = change.value.cast::<f32>() else {
                continue;
            };
            if !write_val_number_field(field, value, control.numeric_min) {
                continue;
            }
            store_widget_prop_change(
                node_id,
                &control.field_path,
                InspectorFieldEditor::new(INSPECTOR_DRIVER_VAL),
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
        let Some(value) = change.value.cast::<f32>() else {
            continue;
        };
        if !write_val_number_field(field, value, control.numeric_min) {
            continue;
        }
        apply_style_change(node_id, style, &mut document, &widget_registry);
    }
}

fn apply_inspector_val_dropdown_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    editor_theme: Res<EditorTheme>,
    mut changes: MessageReader<DropdownChange>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    val_unit_inputs: Query<&InspectorValUnitInput>,
    val_controls: Query<&InspectorValControl>,
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
    let theme = Some(&editor_theme.0);

    for change in changes.read() {
        let Ok(input) = val_unit_inputs.get(change.entity) else {
            continue;
        };
        let Ok(control) = val_controls.get(input.owner) else {
            continue;
        };
        if matches!(control.target, InspectorBindingTarget::WidgetProp) {
            let Some(mut value) = selected_node_widget_reflect(
                &panel_state,
                &document,
                &widget_registry,
                &inspector_registry,
                &control_registry,
                theme,
            ) else {
                continue;
            };
            let Some(field) = read_reflect_path_mut(value.as_mut(), &control.field_path) else {
                continue;
            };
            if !write_val_unit_field(field, change.selected, control.numeric_min) {
                continue;
            }
            store_widget_prop_change(
                node_id,
                &control.field_path,
                InspectorFieldEditor::new(INSPECTOR_DRIVER_VAL),
                field,
                &mut document,
                &widget_registry,
                &control_registry,
                theme,
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
        if !write_val_unit_field(field, change.selected, control.numeric_min) {
            continue;
        }
        apply_style_change(node_id, style, &mut document, &widget_registry);
    }
}

fn sync_inspector_val_controls(
    panel_state: Res<InspectorPanelState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    val_controls: Query<&InspectorValControl>,
    mut value_fields: Query<&mut NumberField>,
    mut unit_dropdowns: Query<&mut Dropdown>,
) {
    if !panel_state.is_changed() && !document.is_changed() {
        return;
    }

    let Some(style) = selected_node_style(&panel_state, &document) else {
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
    let widget_reflect = selected_node_widget_reflect(
        &panel_state,
        &document,
        &widget_registry,
        &inspector_registry,
        &control_registry,
        None,
    );

    for control in val_controls.iter() {
        let source: Option<&dyn PartialReflect> = match control.target {
            InspectorBindingTarget::Style => {
                read_reflect_path(style as &dyn PartialReflect, &control.field_path)
            }
            InspectorBindingTarget::WidgetProp => widget_reflect
                .as_deref()
                .and_then(|value| read_reflect_path(value, &control.field_path)),
        };
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
