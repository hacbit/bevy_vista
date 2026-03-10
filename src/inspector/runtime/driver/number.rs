use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(NumberInspectorDriver)
}

struct NumberInspectorDriver;

impl InspectorDriver for NumberInspectorDriver {
    fn id(&self) -> InspectorDriverId {
        INSPECTOR_DRIVER_NUMBER
    }

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let value_input = NumberFieldBuilder::new()
            .kind(NumberKind::F32)
            .width(px(96.0))
            .height(px(28.0))
            .disabled(true)
            .build(commands, theme);
        let kind_input = DropdownBuilder::new()
            .width(px(84.0))
            .options(number_kind_options())
            .disabled(true)
            .build(commands, theme);
        let owner = commands
            .spawn((
                Name::new(format!("Inspector {} Number Control", field.label)),
                Node {
                    min_width: px(0.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexEnd,
                    column_gap: px(6.0),
                    ..default()
                },
                InspectorNumberControl {
                    field_path: field.field_path.clone(),
                    numeric_min: field.numeric_min,
                    target: InspectorBindingTarget::Style,
                    value_input,
                    kind_input,
                },
            ))
            .add_children(&[value_input, kind_input])
            .id();
        commands
            .entity(value_input)
            .insert(InspectorNumberValueInput { owner });
        commands
            .entity(kind_input)
            .insert(InspectorNumberKindInput { owner });
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
            .entry::<InspectorNumberControl>()
            .and_modify(move |mut binding| {
                binding.target = target.clone();
            });
    }

    fn serialize(
        &self,
        _editor: InspectorFieldEditor,
        field: &dyn PartialReflect,
        _theme: Option<&Theme>,
    ) -> Option<String> {
        Some(read_number_field(field)?.serialize())
    }

    fn apply_serialized(
        &self,
        _editor: InspectorFieldEditor,
        field: &mut dyn PartialReflect,
        raw: &str,
        numeric_min: Option<f32>,
        _theme: Option<&Theme>,
    ) -> bool {
        parse_number_for_field(field, raw, numeric_min)
            .is_some_and(|value| write_number_field(field, value, numeric_min))
    }

    fn install_systems(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_inspector_number_value_changes.before(refresh_inspector_panel),
                apply_inspector_number_kind_changes.before(refresh_inspector_panel),
                sync_inspector_number_controls.after(sync_widget_property_section),
            )
                .run_if(in_state(crate::editor::VistaEditorInitPhase::Finalize)),
        );
    }
}

fn apply_inspector_number_value_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    mut changes: MessageReader<NumberFieldChange>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    value_inputs: Query<&InspectorNumberValueInput>,
    number_controls: Query<&InspectorNumberControl>,
    val_value_inputs: Query<(), With<InspectorValValueInput>>,
    vec2_axis_inputs: Query<(), With<InspectorVec2AxisInput>>,
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
        if val_value_inputs.contains(change.entity) || vec2_axis_inputs.contains(change.entity) {
            continue;
        }
        let Ok(input) = value_inputs.get(change.entity) else {
            continue;
        };
        let Ok(control) = number_controls.get(input.owner) else {
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
            if !write_number_field(field, change.value, control.numeric_min) {
                continue;
            }
            store_widget_prop_change(
                node_id,
                &control.field_path,
                InspectorFieldEditor::new(INSPECTOR_DRIVER_NUMBER),
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
        if !write_number_field(field, change.value, control.numeric_min) {
            continue;
        }
        apply_style_change(node_id, style, &mut document, &widget_registry);
    }
}

fn apply_inspector_number_kind_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    mut changes: MessageReader<DropdownChange>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    kind_inputs: Query<&InspectorNumberKindInput>,
    number_controls: Query<&InspectorNumberControl>,
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
        let Ok(input) = kind_inputs.get(change.entity) else {
            continue;
        };
        let Ok(control) = number_controls.get(input.owner) else {
            continue;
        };
        let Some(kind) = number_kind_from_index(change.selected) else {
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
            if !write_number_kind_field(field, kind, control.numeric_min) {
                continue;
            }
            store_widget_prop_change(
                node_id,
                &control.field_path,
                InspectorFieldEditor::new(INSPECTOR_DRIVER_NUMBER),
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
        if !write_number_kind_field(field, kind, control.numeric_min) {
            continue;
        }
        apply_style_change(node_id, style, &mut document, &widget_registry);
    }
}

fn sync_inspector_number_controls(
    panel_state: Res<InspectorPanelState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    number_controls: Query<&InspectorNumberControl>,
    mut value_fields: Query<&mut NumberField>,
    mut kind_dropdowns: Query<(&mut Dropdown, &mut Node)>,
) {
    if !panel_state.is_changed() && !document.is_changed() {
        return;
    }

    let Some(style) = selected_node_style(&panel_state, &document) else {
        for control in number_controls.iter() {
            if let Ok(mut field) = value_fields.get_mut(control.value_input) {
                field.disabled = true;
            }
            if let Ok((mut dropdown, mut node)) = kind_dropdowns.get_mut(control.kind_input) {
                dropdown.disabled = true;
                node.display = Display::None;
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

    for control in number_controls.iter() {
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
            if let Ok((mut dropdown, mut node)) = kind_dropdowns.get_mut(control.kind_input) {
                dropdown.disabled = true;
                node.display = Display::None;
            }
            continue;
        };
        let Some(value) = read_number_field(style_field) else {
            if let Ok(mut field) = value_fields.get_mut(control.value_input) {
                field.disabled = true;
            }
            if let Ok((mut dropdown, mut node)) = kind_dropdowns.get_mut(control.kind_input) {
                dropdown.disabled = true;
                node.display = Display::None;
            }
            continue;
        };

        if let Ok(mut field) = value_fields.get_mut(control.value_input) {
            field.value = value;
            field.disabled = false;
        }
        if let Ok((mut dropdown, mut node)) = kind_dropdowns.get_mut(control.kind_input) {
            dropdown.options = number_kind_options();
            dropdown.selected = number_kind_index(value.kind());
            dropdown.expanded = false;
            let is_number = style_field.try_downcast_ref::<Number>().is_some();
            dropdown.disabled = !is_number;
            node.display = if is_number {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

fn number_kind_options() -> Vec<String> {
    [
        NumberKind::I8,
        NumberKind::I16,
        NumberKind::I32,
        NumberKind::I64,
        NumberKind::Isize,
        NumberKind::U8,
        NumberKind::U16,
        NumberKind::U32,
        NumberKind::U64,
        NumberKind::Usize,
        NumberKind::F32,
        NumberKind::F64,
    ]
    .into_iter()
    .map(|kind| kind.name().to_owned())
    .collect()
}

fn number_kind_index(kind: NumberKind) -> usize {
    match kind {
        NumberKind::I8 => 0,
        NumberKind::I16 => 1,
        NumberKind::I32 => 2,
        NumberKind::I64 => 3,
        NumberKind::Isize => 4,
        NumberKind::U8 => 5,
        NumberKind::U16 => 6,
        NumberKind::U32 => 7,
        NumberKind::U64 => 8,
        NumberKind::Usize => 9,
        NumberKind::F32 => 10,
        NumberKind::F64 => 11,
    }
}

fn number_kind_from_index(index: usize) -> Option<NumberKind> {
    match index {
        0 => Some(NumberKind::I8),
        1 => Some(NumberKind::I16),
        2 => Some(NumberKind::I32),
        3 => Some(NumberKind::I64),
        4 => Some(NumberKind::Isize),
        5 => Some(NumberKind::U8),
        6 => Some(NumberKind::U16),
        7 => Some(NumberKind::U32),
        8 => Some(NumberKind::U64),
        9 => Some(NumberKind::Usize),
        10 => Some(NumberKind::F32),
        11 => Some(NumberKind::F64),
        _ => None,
    }
}
