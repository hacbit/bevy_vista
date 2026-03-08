use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use crate::inspector::{
    InspectorDriverKey, InspectorEditorRegistry, InspectorEntryDescriptor,
    InspectorFieldDescriptor, InspectorHeaderDescriptor, InspectorResolvedEditor,
    InspectorValAdapter,
    InspectorVec2Adapter,
    default_choice_options, read_bool_field, read_choice_field, read_number_field,
    read_reflect_path, read_reflect_path_mut, read_val_field, read_vec2_field, val_unit_options,
    write_bool_field, write_choice_field, write_number_field, write_val_number_field,
    write_val_unit_field, write_vec2_axis_field,
};
use crate::theme::Theme;
use crate::widget::{
    Checkbox, CheckboxBuilder, CheckboxChange, Dropdown, DropdownBuilder, DropdownChange, F32Field,
    F32FieldBuilder, F32FieldChange, FoldoutBuilder, LabelBuilder, ScrollViewBuilder, TextField,
    TextFieldBuilder, TextInputChange, TextInputSubmit, WidgetRegistry, WidgetStyle,
};

use super::*;

#[derive(Resource, Default)]
pub(super) struct InspectorPanelState {
    selected_node: Option<blueprint::BlueprintNodeId>,
    visible: bool,
}

type InspectorControlBuilder = fn(&mut Commands, &InspectorFieldDescriptor, Option<&Theme>) -> Entity;

#[derive(Resource)]
pub(super) struct InspectorControlRegistry {
    builders: bevy::platform::collections::HashMap<InspectorDriverKey, InspectorControlBuilder>,
}

impl Default for InspectorControlRegistry {
    fn default() -> Self {
        let mut builders = bevy::platform::collections::HashMap::default();
        builders.insert(
            InspectorDriverKey::Number,
            build_numeric_control as InspectorControlBuilder,
        );
        builders.insert(
            InspectorDriverKey::Choice,
            build_choice_control as InspectorControlBuilder,
        );
        builders.insert(
            InspectorDriverKey::Bool,
            build_bool_control as InspectorControlBuilder,
        );
        builders.insert(
            InspectorDriverKey::Val,
            build_val_control as InspectorControlBuilder,
        );
        builders.insert(
            InspectorDriverKey::Vec2,
            build_vec2_control as InspectorControlBuilder,
        );
        Self { builders }
    }
}

impl InspectorControlRegistry {
    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let key = field.editor.driver_key();
        let Some(builder) = self.builders.get(&key).copied() else {
            panic!("missing inspector control builder for {:?}", key);
        };
        builder(commands, field, theme)
    }
}

#[derive(Component)]
pub(super) struct InspectorContentRoot;

#[derive(Component)]
pub(super) struct InspectorNameField;

#[derive(Component, Clone)]
pub(super) struct InspectorControlBinding {
    field_path: String,
    editor: InspectorResolvedEditor,
    numeric_min: Option<f32>,
}

#[derive(Component, Clone)]
pub(super) struct InspectorValControl {
    field_path: String,
    numeric_min: Option<f32>,
    adapter: InspectorValAdapter,
    value_input: Entity,
    unit_input: Entity,
}

#[derive(Component, Copy, Clone)]
pub(super) struct InspectorValValueInput {
    owner: Entity,
}

#[derive(Component, Copy, Clone)]
pub(super) struct InspectorValUnitInput {
    owner: Entity,
}

#[derive(Component, Clone)]
pub(super) struct InspectorVec2Control {
    field_path: String,
    x_input: Entity,
    y_input: Entity,
}

#[derive(Component, Copy, Clone)]
pub(super) struct InspectorVec2AxisInput {
    owner: Entity,
    axis: usize,
}

pub(super) fn init_inspector_panel(
    mut commands: Commands,
    inspector: Single<Entity, With<Inspector>>,
    editor_theme: Res<EditorTheme>,
    registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
) {
    let theme = Some(&editor_theme.0);
    let panel_bg = editor_theme.0.palette.surface;
    let text_color = editor_theme.0.palette.on_surface;
    let font_size = editor_theme.0.typography.body_medium.font.font_size;

    commands
        .entity(*inspector)
        .insert(BackgroundColor(panel_bg));

    let name_label = commands
        .spawn((
            Name::new("Inspector Name Label"),
            LabelBuilder::new()
                .text("Name")
                .font_size(font_size)
                .color(text_color)
                .build(),
        ))
        .id();
    let name_field = TextFieldBuilder::new()
        .width(percent(100.0))
        .height(px(28.0))
        .disabled(true)
        .build(&mut commands, theme);
    commands.entity(name_field).insert(InspectorNameField);

    let name_row = commands
        .spawn((
            Name::new("Inspector Name Row"),
            Node {
                width: percent(100.0),
                min_width: px(0.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: px(8.0),
                ..default()
            },
        ))
        .add_children(&[name_label, name_field])
        .id();

    let property_list_content = build_property_entries(
        &mut commands,
        registry.entries_for::<WidgetStyle>(),
        &control_registry,
        theme,
        font_size,
        text_color,
    );
    let property_list = ScrollViewBuilder::new()
        .width(percent(100.0))
        .height(percent(100.0))
        .show_horizontal(false)
        .build_with_entities(&mut commands, [property_list_content]);
    commands.entity(property_list).entry::<Node>().and_modify(|mut node| {
        node.min_width = px(0.0);
        node.min_height = px(0.0);
        node.flex_grow = 1.0;
        node.flex_shrink = 1.0;
    });

    let content_root = commands
        .spawn((
            Name::new("Inspector Content Root"),
            Node {
                width: percent(100.0),
                height: percent(100.0),
                padding: UiRect::all(px(8.0)),
                min_width: px(0.0),
                min_height: px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(8.0),
                display: Display::None,
                ..default()
            },
            InspectorContentRoot,
        ))
        .add_children(&[name_row, property_list])
        .id();

    commands.entity(*inspector).add_child(content_root);
}

fn build_property_entries(
    commands: &mut Commands,
    entries: Vec<InspectorEntryDescriptor>,
    control_registry: &InspectorControlRegistry,
    theme: Option<&Theme>,
    font_size: f32,
    text_color: Color,
) -> Entity {
    struct GroupFrame {
        header: InspectorHeaderDescriptor,
        children: Vec<Entity>,
    }

    let mut root_children = Vec::new();
    let mut group_stack: Vec<GroupFrame> = Vec::new();

    let finish_group =
        |commands: &mut Commands,
         target_children: &mut Vec<Entity>,
         frame: GroupFrame| {
            let header = frame.header;
            let group_children = frame.children;
        let content = commands
            .spawn((
                Name::new(format!("Inspector {} Group Content", header.title)),
                Node {
                    width: percent(100.0),
                    min_width: px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4.0),
                    ..default()
                },
            ))
            .id();
        if !group_children.is_empty() {
            commands.entity(content).add_children(&group_children);
        }

        let foldout = FoldoutBuilder::new(header.title)
            .expanded(header.default_open)
            .width(percent(100.0))
            .build_with_entity(commands, content, theme);
        target_children.push(foldout);
    };

    for entry in entries {
        match entry {
            InspectorEntryDescriptor::Header(header) => {
                if header.implicit_close_previous
                    && group_stack
                        .last()
                        .is_some_and(|frame| frame.header.implicit_close_previous)
                {
                    let frame = group_stack.pop().expect("checked group_stack is not empty");
                    if let Some(parent) = group_stack.last_mut() {
                        finish_group(commands, &mut parent.children, frame);
                    } else {
                        finish_group(commands, &mut root_children, frame);
                    }
                }
                group_stack.push(GroupFrame {
                    header,
                    children: Vec::new(),
                });
            }
            InspectorEntryDescriptor::Field(field) => {
                let row = spawn_property_row(
                    commands,
                    &field,
                    control_registry,
                    theme,
                    font_size,
                    text_color,
                );
                if let Some(group) = group_stack.last_mut() {
                    group.children.push(row);
                } else {
                    root_children.push(row);
                }
            }
            InspectorEntryDescriptor::EndHeader => {
                let Some(frame) = group_stack.pop() else {
                    continue;
                };
                if let Some(parent) = group_stack.last_mut() {
                    finish_group(commands, &mut parent.children, frame);
                } else {
                    finish_group(commands, &mut root_children, frame);
                }
            }
        }
    }

    while let Some(frame) = group_stack.pop() {
        if let Some(parent) = group_stack.last_mut() {
            finish_group(commands, &mut parent.children, frame);
        } else {
            finish_group(commands, &mut root_children, frame);
        }
    }

    let root = commands
        .spawn((
            Name::new("Inspector Property List"),
            Node {
                width: percent(100.0),
                min_width: px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.0),
                ..default()
            },
        ))
        .id();
    if !root_children.is_empty() {
        commands.entity(root).add_children(&root_children);
    }
    root
}

pub(super) fn refresh_inspector_panel(
    options: Res<VistaEditorViewOptions>,
    selection: Res<VistaEditorSelection>,
    runtime_map: Res<blueprint::BlueprintRuntimeMap>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    mut panel_state: ResMut<InspectorPanelState>,
    mut content_root: Single<&mut Node, With<InspectorContentRoot>>,
    mut name_field: Single<&mut TextField, With<InspectorNameField>>,
) {
    if !options.is_changed() && !selection.is_changed() && !document.is_changed() {
        return;
    }

    if options.is_preview_mode {
        content_root.display = Display::None;
        name_field.disabled = true;
        panel_state.visible = false;
        panel_state.selected_node = None;
        return;
    }

    let selected_node = selection
        .selected_entity
        .and_then(|entity| runtime_map.entity_to_node.get(&entity).copied());
    let Some(node_id) = selected_node else {
        content_root.display = Display::None;
        name_field.disabled = true;
        panel_state.visible = false;
        panel_state.selected_node = None;
        return;
    };
    let Some(node) = document.nodes.get(&node_id) else {
        content_root.display = Display::None;
        name_field.disabled = true;
        panel_state.visible = false;
        panel_state.selected_node = None;
        return;
    };

    content_root.display = Display::Flex;
    name_field.disabled = false;
    panel_state.visible = true;
    panel_state.selected_node = Some(node_id);
    if name_field.value != node.name {
        name_field.value = node.name.clone();
        name_field.cursor_pos = name_field.value.chars().count();
        name_field.selection = None;
    }
}

pub(super) fn apply_inspector_name_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    mut changes: MessageReader<TextInputChange>,
    mut submits: MessageReader<TextInputSubmit>,
    name_fields: Query<(), With<InspectorNameField>>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    schemas: Res<blueprint::WidgetSchemaRegistry>,
    mut hierarchy: ResMut<hierarchy::HierarchyState>,
) {
    if options.is_preview_mode {
        changes.clear();
        submits.clear();
        return;
    }
    let Some(node_id) = panel_state.selected_node else {
        return;
    };

    let mut pending_name = None;
    for event in changes.read() {
        if name_fields.contains(event.entity) {
            pending_name = Some(event.value.clone());
        }
    }
    for event in submits.read() {
        if name_fields.contains(event.entity) {
            pending_name = Some(event.value.clone());
        }
    }

    let Some(name) = pending_name else {
        return;
    };
    let _ = blueprint::apply_blueprint_command(
        blueprint::BlueprintCommand::SetNodeName {
            node: node_id,
            name,
        },
        &mut document,
        &schemas,
        &widget_registry,
    );
    hierarchy.dirty = true;
}

pub(super) fn apply_inspector_numeric_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    mut changes: MessageReader<F32FieldChange>,
    controls: Query<&InspectorControlBinding>,
    val_value_inputs: Query<&InspectorValValueInput>,
    val_controls: Query<&InspectorValControl>,
    vec2_axis_inputs: Query<&InspectorVec2AxisInput>,
    vec2_controls: Query<&InspectorVec2Control>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    schemas: Res<blueprint::WidgetSchemaRegistry>,
) {
    if options.is_preview_mode {
        changes.clear();
        return;
    }
    let Some(node_id) = panel_state.selected_node else {
        return;
    };

    for change in changes.read() {
        if let Ok(input) = val_value_inputs.get(change.entity) {
            let Ok(control) = val_controls.get(input.owner) else {
                continue;
            };
            let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone()) else {
                continue;
            };
            let style_reflect: &mut dyn PartialReflect = &mut style;
            let Some(field) = read_reflect_path_mut(style_reflect, &control.field_path) else {
                continue;
            };
            if !write_val_number_field(control.adapter, field, change.value, control.numeric_min) {
                continue;
            }
            apply_style_change(node_id, style, &mut document, &schemas, &widget_registry);
            continue;
        }

        if let Ok(input) = vec2_axis_inputs.get(change.entity) {
            let Ok(control) = vec2_controls.get(input.owner) else {
                continue;
            };
            let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone()) else {
                continue;
            };
            let style_reflect: &mut dyn PartialReflect = &mut style;
            let Some(field) = read_reflect_path_mut(style_reflect, &control.field_path) else {
                continue;
            };
            if !write_vec2_axis_field(
                InspectorVec2Adapter::Vec2,
                field,
                input.axis,
                change.value,
            ) {
                continue;
            }
            apply_style_change(node_id, style, &mut document, &schemas, &widget_registry);
            continue;
        }

        let Ok(control) = controls.get(change.entity) else {
            continue;
        };
        let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone()) else {
            continue;
        };
        let style_reflect: &mut dyn PartialReflect = &mut style;
        let Some(field) = read_reflect_path_mut(style_reflect, &control.field_path) else {
            continue;
        };
        let InspectorResolvedEditor::Number(adapter) = control.editor else {
            continue;
        };
        if !write_number_field(adapter, field, change.value, control.numeric_min) {
            continue;
        }
        apply_style_change(node_id, style, &mut document, &schemas, &widget_registry);
    }
}

pub(super) fn apply_inspector_dropdown_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    editor_theme: Res<EditorTheme>,
    mut changes: MessageReader<DropdownChange>,
    controls: Query<&InspectorControlBinding>,
    val_unit_inputs: Query<&InspectorValUnitInput>,
    val_controls: Query<&InspectorValControl>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    schemas: Res<blueprint::WidgetSchemaRegistry>,
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
        if let Ok(input) = val_unit_inputs.get(change.entity) {
            let Ok(control) = val_controls.get(input.owner) else {
                continue;
            };
            let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone()) else {
                continue;
            };
            let style_reflect: &mut dyn PartialReflect = &mut style;
            let Some(field) = read_reflect_path_mut(style_reflect, &control.field_path) else {
                continue;
            };
            if !write_val_unit_field(control.adapter, field, change.selected, control.numeric_min) {
                continue;
            }
            apply_style_change(node_id, style, &mut document, &schemas, &widget_registry);
            continue;
        }

        let Ok(control) = controls.get(change.entity) else {
            continue;
        };
        let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone()) else {
            continue;
        };
        let style_reflect: &mut dyn PartialReflect = &mut style;
        let Some(field) = read_reflect_path_mut(style_reflect, &control.field_path) else {
            continue;
        };
        let InspectorResolvedEditor::Choice(adapter) = control.editor else {
            continue;
        };
        if !write_choice_field(adapter, field, change.selected, theme) {
            continue;
        }
        apply_style_change(node_id, style, &mut document, &schemas, &widget_registry);
    }
}

pub(super) fn apply_inspector_checkbox_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    mut changes: MessageReader<CheckboxChange>,
    controls: Query<&InspectorControlBinding>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    schemas: Res<blueprint::WidgetSchemaRegistry>,
) {
    if options.is_preview_mode {
        changes.clear();
        return;
    }
    let Some(node_id) = panel_state.selected_node else {
        return;
    };

    for change in changes.read() {
        let Ok(control) = controls.get(change.entity) else {
            continue;
        };
        let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone()) else {
            continue;
        };
        let style_reflect: &mut dyn PartialReflect = &mut style;
        let Some(field) = read_reflect_path_mut(style_reflect, &control.field_path) else {
            continue;
        };
        let InspectorResolvedEditor::Bool(adapter) = control.editor else {
            continue;
        };
        if !write_bool_field(adapter, field, change.checked) {
            continue;
        }
        apply_style_change(node_id, style, &mut document, &schemas, &widget_registry);
    }
}

fn spawn_property_row(
    commands: &mut Commands,
    field: &InspectorFieldDescriptor,
    control_registry: &InspectorControlRegistry,
    theme: Option<&Theme>,
    font_size: f32,
    text_color: Color,
) -> Entity {
    let label = commands
        .spawn((
            Name::new(format!("Inspector {} Label", field.label)),
            LabelBuilder::new()
                .text(field.label.clone())
                .font_size(font_size)
                .color(text_color)
                .build(),
        ))
        .id();
    let control = control_registry.build(commands, field, theme);
    commands
        .spawn((
            Name::new(format!("Inspector {} Row", field.label)),
            Node {
                width: percent(100.0),
                min_width: px(0.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: px(8.0),
                ..default()
            },
        ))
        .add_children(&[label, control])
        .id()
}

fn build_numeric_control(
    commands: &mut Commands,
    field: &InspectorFieldDescriptor,
    theme: Option<&Theme>,
) -> Entity {
    let InspectorResolvedEditor::Number(_) = field.editor else {
        panic!("numeric builder received non-number editor");
    };
    let entity = F32FieldBuilder::new()
        .width(px(132.0))
        .height(px(28.0))
        .disabled(true)
        .build(commands, theme);
    if matches!(
        field.editor,
        InspectorResolvedEditor::Number(_)
            | InspectorResolvedEditor::Choice(_)
            | InspectorResolvedEditor::Bool(_)
    ) {
        commands.entity(entity).insert(InspectorControlBinding {
            field_path: field.field_path.clone(),
            editor: field.editor,
            numeric_min: field.numeric_min,
        });
    }
    entity
}

fn build_choice_control(
    commands: &mut Commands,
    field: &InspectorFieldDescriptor,
    theme: Option<&Theme>,
) -> Entity {
    let InspectorResolvedEditor::Choice(adapter) = field.editor else {
        panic!("choice builder received non-choice editor");
    };
    let entity = DropdownBuilder::new()
        .width(px(144.0))
        .options(default_choice_options(adapter, theme))
        .disabled(true)
        .build(commands, theme);
    commands.entity(entity).insert(InspectorControlBinding {
        field_path: field.field_path.clone(),
        editor: field.editor,
        numeric_min: field.numeric_min,
    });
    entity
}

fn build_bool_control(
    commands: &mut Commands,
    field: &InspectorFieldDescriptor,
    theme: Option<&Theme>,
) -> Entity {
    let InspectorResolvedEditor::Bool(_) = field.editor else {
        panic!("bool builder received non-bool editor");
    };
    let entity = CheckboxBuilder::new().disabled(true).build(commands, theme);
    commands.entity(entity).insert(InspectorControlBinding {
        field_path: field.field_path.clone(),
        editor: field.editor,
        numeric_min: field.numeric_min,
    });
    entity
}

fn build_val_control(
    commands: &mut Commands,
    field: &InspectorFieldDescriptor,
    theme: Option<&Theme>,
) -> Entity {
    let InspectorResolvedEditor::Val(adapter) = field.editor else {
        panic!("val builder received non-val editor");
    };
    let value_input = F32FieldBuilder::new()
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
                adapter,
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

fn build_vec2_control(
    commands: &mut Commands,
    field: &InspectorFieldDescriptor,
    theme: Option<&Theme>,
) -> Entity {
    let InspectorResolvedEditor::Vec2(_) = field.editor else {
        panic!("vec2 builder received non-vec2 editor");
    };
    let x_input = F32FieldBuilder::new()
        .width(px(72.0))
        .height(px(28.0))
        .disabled(true)
        .build(commands, theme);
    let y_input = F32FieldBuilder::new()
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

pub(super) fn sync_inspector_numeric_controls(
    panel_state: Res<InspectorPanelState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    mut numeric_controls: Query<(&InspectorControlBinding, &mut F32Field)>,
) {
    if !panel_state.is_changed() && !document.is_changed() {
        return;
    }

    let Some(style) = selected_node_style(&panel_state, &document) else {
        for (_, mut field) in numeric_controls.iter_mut() {
            field.disabled = true;
        }
        return;
    };

    for (binding, mut field) in numeric_controls.iter_mut() {
        let Some(style_field) = read_reflect_path(style as &dyn PartialReflect, &binding.field_path)
        else {
            field.disabled = true;
            continue;
        };
        let InspectorResolvedEditor::Number(adapter) = binding.editor else {
            field.disabled = true;
            continue;
        };
        if let Some(value) = read_number_field(adapter, style_field) {
            field.value = value;
            field.disabled = false;
        } else {
            field.disabled = true;
        }
    }
}

pub(super) fn sync_inspector_dropdown_controls(
    panel_state: Res<InspectorPanelState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    editor_theme: Res<EditorTheme>,
    mut dropdown_controls: Query<(&InspectorControlBinding, &mut Dropdown)>,
) {
    let theme = Some(&editor_theme.0);
    if !panel_state.is_changed() && !document.is_changed() && !editor_theme.is_changed() {
        return;
    }

    let Some(style) = selected_node_style(&panel_state, &document) else {
        for (binding, mut dropdown) in dropdown_controls.iter_mut() {
            let InspectorResolvedEditor::Choice(adapter) = binding.editor else {
                continue;
            };
            dropdown.options = default_choice_options(adapter, theme);
            dropdown.selected = 0;
            dropdown.expanded = false;
            dropdown.disabled = true;
        }
        return;
    };

    for (binding, mut dropdown) in dropdown_controls.iter_mut() {
        let Some(style_field) = read_reflect_path(style as &dyn PartialReflect, &binding.field_path)
        else {
            dropdown.disabled = true;
            continue;
        };
        let InspectorResolvedEditor::Choice(adapter) = binding.editor else {
            dropdown.disabled = true;
            continue;
        };
        if let Some((options, selected)) = read_choice_field(adapter, style_field, theme) {
            dropdown.options = options;
            dropdown.selected = selected;
            dropdown.expanded = false;
            dropdown.disabled = false;
        } else {
            dropdown.disabled = true;
        }
    }
}

pub(super) fn sync_inspector_checkbox_controls(
    panel_state: Res<InspectorPanelState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    mut checkbox_controls: Query<(&InspectorControlBinding, &mut Checkbox)>,
) {
    if !panel_state.is_changed() && !document.is_changed() {
        return;
    }

    let Some(style) = selected_node_style(&panel_state, &document) else {
        for (_, mut checkbox) in checkbox_controls.iter_mut() {
            checkbox.checked = false;
            checkbox.disabled = true;
        }
        return;
    };

    for (binding, mut checkbox) in checkbox_controls.iter_mut() {
        let Some(style_field) = read_reflect_path(style as &dyn PartialReflect, &binding.field_path)
        else {
            checkbox.disabled = true;
            continue;
        };
        let InspectorResolvedEditor::Bool(adapter) = binding.editor else {
            checkbox.disabled = true;
            continue;
        };
        if let Some(checked) = read_bool_field(adapter, style_field) {
            checkbox.checked = checked;
            checkbox.disabled = false;
        } else {
            checkbox.disabled = true;
        }
    }
}

pub(super) fn sync_inspector_val_controls(
    panel_state: Res<InspectorPanelState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    val_controls: Query<&InspectorValControl>,
    mut value_fields: Query<&mut F32Field>,
    mut unit_dropdowns: Query<&mut Dropdown>,
) {
    if !panel_state.is_changed() && !document.is_changed() {
        return;
    }

    let Some(style) = selected_node_style(&panel_state, &document) else {
        for control in val_controls.iter() {
            if let Ok(mut field) = value_fields.get_mut(control.value_input) {
                field.value = 0.0;
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
        let Some(style_field) = read_reflect_path(style as &dyn PartialReflect, &control.field_path)
        else {
            if let Ok(mut field) = value_fields.get_mut(control.value_input) {
                field.disabled = true;
            }
            if let Ok(mut dropdown) = unit_dropdowns.get_mut(control.unit_input) {
                dropdown.disabled = true;
            }
            continue;
        };
        if let Some((value, selected, number_enabled)) = read_val_field(control.adapter, style_field)
        {
            if let Ok(mut field) = value_fields.get_mut(control.value_input) {
                field.value = value;
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

pub(super) fn sync_inspector_vec2_controls(
    panel_state: Res<InspectorPanelState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
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

    for control in vec2_controls.iter() {
        let Some(style_field) = read_reflect_path(style as &dyn PartialReflect, &control.field_path)
        else {
            if let Ok(mut field) = value_fields.get_mut(control.x_input) {
                field.disabled = true;
            }
            if let Ok(mut field) = value_fields.get_mut(control.y_input) {
                field.disabled = true;
            }
            continue;
        };
        if let Some(value) = read_vec2_field(InspectorVec2Adapter::Vec2, style_field) {
            if let Ok(mut field) = value_fields.get_mut(control.x_input) {
                field.value = value.x;
                field.disabled = false;
            }
            if let Ok(mut field) = value_fields.get_mut(control.y_input) {
                field.value = value.y;
                field.disabled = false;
            }
        }
    }
}

fn selected_node_style<'a>(
    panel_state: &InspectorPanelState,
    document: &'a blueprint::WidgetBlueprintDocument,
) -> Option<&'a WidgetStyle> {
    let node_id = panel_state.selected_node?;
    let node = document.nodes.get(&node_id)?;
    if !panel_state.visible {
        return None;
    }
    Some(&node.style)
}

fn apply_style_change(
    node_id: blueprint::BlueprintNodeId,
    style: WidgetStyle,
    document: &mut blueprint::WidgetBlueprintDocument,
    schemas: &blueprint::WidgetSchemaRegistry,
    widget_registry: &WidgetRegistry,
) {
    let _ = blueprint::apply_blueprint_command(
        blueprint::BlueprintCommand::SetNodeStyle {
            node: node_id,
            style,
        },
        document,
        schemas,
        widget_registry,
    );
}
