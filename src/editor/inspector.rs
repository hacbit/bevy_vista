use bevy::prelude::*;
use bevy::reflect::Struct;

use crate::inspector::{
    InspectorEditorRegistry, InspectorFieldDescriptor, InspectorResolvedEditor,
    default_choice_options, read_bool_field, read_choice_field, read_number_field,
    write_bool_field, write_choice_field, write_number_field,
};
use crate::widget::{
    Checkbox, CheckboxBuilder, CheckboxChange, Dropdown, DropdownBuilder, DropdownChange, F32Field,
    F32FieldBuilder, F32FieldChange, LabelBuilder, ListViewBuilder, TextField, TextFieldBuilder,
    TextInputChange, TextInputSubmit, WidgetRegistry, WidgetStyle,
};

use super::*;

#[derive(Component)]
pub(super) struct InspectorContentRoot;

#[derive(Component)]
pub(super) struct InspectorNameField;

#[derive(Component, Clone)]
pub(super) struct InspectorControlBinding {
    field_name: String,
    editor: InspectorResolvedEditor,
    numeric_min: Option<f32>,
}

pub(super) fn init_inspector_panel(
    mut commands: Commands,
    inspector: Single<Entity, With<Inspector>>,
    editor_theme: Res<EditorTheme>,
    registry: Res<InspectorEditorRegistry>,
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

    let mut properties = Vec::new();
    for field in registry.fields_for::<WidgetStyle>() {
        properties.push(spawn_property_row(
            &mut commands,
            &field,
            theme,
            font_size,
            text_color,
        ));
    }

    let property_list = ListViewBuilder::new()
        .width(percent(100.0))
        .height(auto())
        .item_gap(2.0)
        .selectable(false)
        .build_with_entities(&mut commands, properties);

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

pub(super) fn refresh_inspector_panel(
    options: Res<VistaEditorViewOptions>,
    selection: Res<VistaEditorSelection>,
    runtime_map: Res<blueprint::BlueprintRuntimeMap>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    editor_theme: Res<EditorTheme>,
    mut content_root: Single<&mut Node, With<InspectorContentRoot>>,
    mut name_field: Single<&mut TextField, With<InspectorNameField>>,
    mut numeric_controls: Query<(&InspectorControlBinding, &mut F32Field)>,
    mut dropdown_controls: Query<(&InspectorControlBinding, &mut Dropdown)>,
    mut checkbox_controls: Query<(&InspectorControlBinding, &mut Checkbox)>,
) {
    let theme_changed = editor_theme.is_changed();
    if !options.is_changed() && !selection.is_changed() && !document.is_changed() && !theme_changed
    {
        return;
    }

    let theme = Some(&editor_theme.0);

    if options.is_preview_mode {
        content_root.display = Display::None;
        name_field.disabled = true;
        disable_inspector_controls(
            &mut numeric_controls,
            &mut dropdown_controls,
            &mut checkbox_controls,
            theme,
        );
        return;
    }

    let selected_node = selection
        .selected_entity
        .and_then(|entity| runtime_map.entity_to_node.get(&entity).copied());
    let Some(node_id) = selected_node else {
        content_root.display = Display::None;
        name_field.disabled = true;
        disable_inspector_controls(
            &mut numeric_controls,
            &mut dropdown_controls,
            &mut checkbox_controls,
            theme,
        );
        return;
    };
    let Some(node) = document.nodes.get(&node_id) else {
        content_root.display = Display::None;
        name_field.disabled = true;
        disable_inspector_controls(
            &mut numeric_controls,
            &mut dropdown_controls,
            &mut checkbox_controls,
            theme,
        );
        return;
    };

    content_root.display = Display::Flex;
    name_field.disabled = false;
    if name_field.value != node.name {
        name_field.value = node.name.clone();
        name_field.cursor_pos = name_field.value.chars().count();
        name_field.selection = None;
    }
    sync_inspector_controls(
        &node.style,
        theme,
        &mut numeric_controls,
        &mut dropdown_controls,
        &mut checkbox_controls,
    );
}

pub(super) fn apply_inspector_name_changes(
    options: Res<VistaEditorViewOptions>,
    selection: Res<VistaEditorSelection>,
    runtime_map: Res<blueprint::BlueprintRuntimeMap>,
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
    let Some(node_id) = selected_node_id(&selection, &runtime_map) else {
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
    selection: Res<VistaEditorSelection>,
    runtime_map: Res<blueprint::BlueprintRuntimeMap>,
    mut changes: MessageReader<F32FieldChange>,
    controls: Query<&InspectorControlBinding>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    schemas: Res<blueprint::WidgetSchemaRegistry>,
) {
    if options.is_preview_mode {
        changes.clear();
        return;
    }
    let Some(node_id) = selected_node_id(&selection, &runtime_map) else {
        return;
    };

    for change in changes.read() {
        let Ok(control) = controls.get(change.entity) else {
            continue;
        };
        let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone()) else {
            continue;
        };
        let Some(field) = style.field_mut(&control.field_name) else {
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
    selection: Res<VistaEditorSelection>,
    runtime_map: Res<blueprint::BlueprintRuntimeMap>,
    editor_theme: Res<EditorTheme>,
    mut changes: MessageReader<DropdownChange>,
    controls: Query<&InspectorControlBinding>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    schemas: Res<blueprint::WidgetSchemaRegistry>,
) {
    if options.is_preview_mode {
        changes.clear();
        return;
    }
    let Some(node_id) = selected_node_id(&selection, &runtime_map) else {
        return;
    };
    let theme = Some(&editor_theme.0);

    for change in changes.read() {
        let Ok(control) = controls.get(change.entity) else {
            continue;
        };
        let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone()) else {
            continue;
        };
        let Some(field) = style.field_mut(&control.field_name) else {
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
    selection: Res<VistaEditorSelection>,
    runtime_map: Res<blueprint::BlueprintRuntimeMap>,
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
    let Some(node_id) = selected_node_id(&selection, &runtime_map) else {
        return;
    };

    for change in changes.read() {
        let Ok(control) = controls.get(change.entity) else {
            continue;
        };
        let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone()) else {
            continue;
        };
        let Some(field) = style.field_mut(&control.field_name) else {
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
    let control = spawn_field_control(commands, field, theme);
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

fn spawn_field_control(
    commands: &mut Commands,
    field: &InspectorFieldDescriptor,
    theme: Option<&Theme>,
) -> Entity {
    let entity = match field.editor {
        InspectorResolvedEditor::Number(_) => F32FieldBuilder::new()
            .width(px(132.0))
            .height(px(28.0))
            .disabled(true)
            .build(commands, theme),
        InspectorResolvedEditor::Choice(adapter) => DropdownBuilder::new()
            .width(px(144.0))
            .options(default_choice_options(adapter, theme))
            .disabled(true)
            .build(commands, theme),
        InspectorResolvedEditor::Bool(_) => {
            CheckboxBuilder::new().disabled(true).build(commands, theme)
        }
    };

    commands.entity(entity).insert(InspectorControlBinding {
        field_name: field.name.clone(),
        editor: field.editor,
        numeric_min: field.numeric_min,
    });
    entity
}

fn disable_inspector_controls(
    numeric_controls: &mut Query<(&InspectorControlBinding, &mut F32Field)>,
    dropdown_controls: &mut Query<(&InspectorControlBinding, &mut Dropdown)>,
    checkbox_controls: &mut Query<(&InspectorControlBinding, &mut Checkbox)>,
    theme: Option<&Theme>,
) {
    for (_, mut field) in numeric_controls.iter_mut() {
        field.disabled = true;
    }
    for (binding, mut dropdown) in dropdown_controls.iter_mut() {
        let InspectorResolvedEditor::Choice(adapter) = binding.editor else {
            continue;
        };
        dropdown.options = default_choice_options(adapter, theme);
        dropdown.selected = 0;
        dropdown.expanded = false;
        dropdown.disabled = true;
    }
    for (_, mut checkbox) in checkbox_controls.iter_mut() {
        checkbox.checked = false;
        checkbox.disabled = true;
    }
}

fn sync_inspector_controls(
    style: &WidgetStyle,
    theme: Option<&Theme>,
    numeric_controls: &mut Query<(&InspectorControlBinding, &mut F32Field)>,
    dropdown_controls: &mut Query<(&InspectorControlBinding, &mut Dropdown)>,
    checkbox_controls: &mut Query<(&InspectorControlBinding, &mut Checkbox)>,
) {
    for (binding, mut field) in numeric_controls.iter_mut() {
        let Some(style_field) = style.field(&binding.field_name) else {
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

    for (binding, mut dropdown) in dropdown_controls.iter_mut() {
        let Some(style_field) = style.field(&binding.field_name) else {
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

    for (binding, mut checkbox) in checkbox_controls.iter_mut() {
        let Some(style_field) = style.field(&binding.field_name) else {
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

fn selected_node_id(
    selection: &VistaEditorSelection,
    runtime_map: &blueprint::BlueprintRuntimeMap,
) -> Option<blueprint::BlueprintNodeId> {
    selection
        .selected_entity
        .and_then(|entity| runtime_map.entity_to_node.get(&entity).copied())
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
