use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use crate::editor::hierarchy;
use crate::editor_resources::{VistaEditorSelection, VistaEditorViewOptions};
use crate::inspector::{
    BlueprintCommand, BlueprintRuntimeMap, InspectorEditorRegistry, WidgetBlueprintDocument,
    apply_blueprint_command, reflect_path_differs_from_default,
};
use crate::theme::EditorTheme;
use crate::widget::{
    FoldoutBuilder, LabelWidget, TextField, TextInputChange, TextInputSubmit, WidgetRegistry,
    WidgetStyle,
};

use super::{
    InspectorBindingTarget, InspectorContentRoot, InspectorControlRegistry,
    InspectorFieldDecoration, InspectorFieldLabel, InspectorFieldRow, InspectorNameField,
    InspectorPanelState, InspectorResetButton, InspectorWidgetSectionRoot,
    InspectorWidgetSectionState, build_property_entries, selected_node_style,
    selected_node_widget_default_reflect, selected_node_widget_reflect,
};

pub(crate) fn sync_widget_property_section(
    mut commands: Commands,
    panel_state: Res<InspectorPanelState>,
    document: Res<WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    editor_theme: Res<EditorTheme>,
    section: Single<
        (Entity, &mut Node, &mut InspectorWidgetSectionState),
        With<InspectorWidgetSectionRoot>,
    >,
    children_query: Query<&Children>,
) {
    if !panel_state.is_changed() && !document.is_changed() {
        return;
    }

    let (section_entity, mut section_node, mut section_state) = section.into_inner();
    let Some(node_id) = panel_state.selected_node else {
        clear_section(
            &mut commands,
            section_entity,
            &children_query,
            &mut section_node,
            &mut section_state,
        );
        return;
    };
    let Some(node) = document.nodes.get(&node_id) else {
        clear_section(
            &mut commands,
            section_entity,
            &children_query,
            &mut section_node,
            &mut section_state,
        );
        return;
    };
    let Some(registration) = widget_registry.get_widget_by_path(&node.widget_path) else {
        clear_section(
            &mut commands,
            section_entity,
            &children_query,
            &mut section_node,
            &mut section_state,
        );
        section_state.selected_node = Some(node_id);
        section_state.widget_path = Some(node.widget_path.clone());
        return;
    };
    let entries = registration.inspector_entries(&inspector_registry);
    if entries.is_empty() {
        clear_section(
            &mut commands,
            section_entity,
            &children_query,
            &mut section_node,
            &mut section_state,
        );
        section_state.selected_node = Some(node_id);
        section_state.widget_path = Some(node.widget_path.clone());
        return;
    }

    if section_state.selected_node == Some(node_id)
        && section_state.widget_path.as_deref() == Some(node.widget_path.as_str())
        && children_query
            .get(section_entity)
            .map(|children| !children.is_empty())
            .unwrap_or(false)
    {
        return;
    }

    if let Ok(children) = children_query.get(section_entity) {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let theme = Some(&editor_theme.0);
    let text_color = editor_theme.0.palette.on_surface;
    let font_size = editor_theme.0.typography.body_medium.font.font_size;
    let content = build_property_entries(
        &mut commands,
        entries,
        &control_registry,
        theme,
        font_size,
        text_color,
        InspectorBindingTarget::WidgetProp,
    );
    let foldout = FoldoutBuilder::new("Widget")
        .expanded(true)
        .width(percent(100.0))
        .build_with_entity(&mut commands, content, theme);
    commands.entity(section_entity).add_child(foldout);
    section_node.display = Display::Flex;
    section_state.selected_node = Some(node_id);
    section_state.widget_path = Some(node.widget_path.clone());
}

pub(crate) fn sync_inspector_context_from_editor_selection(
    options: Res<VistaEditorViewOptions>,
    selection: Res<VistaEditorSelection>,
    runtime_map: Res<BlueprintRuntimeMap>,
    document: Res<WidgetBlueprintDocument>,
    mut panel_state: ResMut<InspectorPanelState>,
) {
    if !options.is_changed() && !selection.is_changed() && !document.is_changed() {
        return;
    }

    if options.is_preview_mode {
        panel_state.clear();
        return;
    }

    let selected_node = selection
        .selected_entity
        .and_then(|entity| runtime_map.entity_to_node.get(&entity).copied());
    let Some(node_id) = selected_node else {
        panel_state.clear();
        return;
    };
    if !document.nodes.contains_key(&node_id) {
        panel_state.clear();
        return;
    }

    panel_state.select(node_id);
}

pub(crate) fn refresh_inspector_panel(
    panel_state: Res<InspectorPanelState>,
    document: Res<WidgetBlueprintDocument>,
    mut content_root: Single<&mut Node, With<InspectorContentRoot>>,
    mut name_field: Single<&mut TextField, With<InspectorNameField>>,
) {
    if !panel_state.is_changed() && !document.is_changed() {
        return;
    }

    let Some(node_id) = panel_state.selected_node else {
        content_root.display = Display::None;
        name_field.disabled = true;
        return;
    };
    let Some(node) = document.nodes.get(&node_id) else {
        content_root.display = Display::None;
        name_field.disabled = true;
        return;
    };
    if !panel_state.visible {
        content_root.display = Display::None;
        name_field.disabled = true;
        return;
    }

    content_root.display = Display::Flex;
    name_field.disabled = false;
    if name_field.value != node.name {
        name_field.value = node.name.clone();
        name_field.cursor_pos = name_field.value.chars().count();
        name_field.selection = None;
    }
}

pub(crate) fn apply_inspector_name_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    mut changes: MessageReader<TextInputChange>,
    mut submits: MessageReader<TextInputSubmit>,
    name_fields: Query<(), With<InspectorNameField>>,
    mut document: ResMut<WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
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
    let _ = apply_blueprint_command(
        BlueprintCommand::SetNodeName {
            node: node_id,
            name,
        },
        &mut document,
        &widget_registry,
    );
    hierarchy.dirty = true;
}

pub(crate) fn sync_inspector_field_markers(
    panel_state: Res<InspectorPanelState>,
    document: Res<WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    editor_theme: Res<EditorTheme>,
    mut label_markers: Query<
        (&InspectorFieldDecoration, &mut LabelWidget),
        With<InspectorFieldLabel>,
    >,
    mut row_markers: Query<
        (
            &InspectorFieldDecoration,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<InspectorFieldRow>,
    >,
    mut reset_buttons: Query<(&InspectorFieldDecoration, &mut Node), With<InspectorResetButton>>,
) {
    if !panel_state.is_changed() && !document.is_changed() && !editor_theme.is_changed() {
        return;
    }

    let style = selected_node_style(&panel_state, &document);
    let default_style = WidgetStyle::default();
    let widget_current = selected_node_widget_reflect(
        &panel_state,
        &document,
        &widget_registry,
        &inspector_registry,
        &control_registry,
        None,
    );
    let widget_default =
        selected_node_widget_default_reflect(&panel_state, &document, &widget_registry);

    let default_label = editor_theme.0.palette.on_surface;
    let modified_label = editor_theme.0.palette.primary;
    let modified_bg = editor_theme.0.palette.primary_container.with_alpha(0.18);
    let modified_border = editor_theme.0.palette.primary;

    for (decoration, mut label) in label_markers.iter_mut() {
        label.color = if inspector_field_is_modified(
            decoration,
            style,
            &default_style,
            widget_current.as_deref(),
            widget_default.as_deref(),
        ) {
            modified_label
        } else {
            default_label
        };
    }

    for (decoration, mut background, mut border) in row_markers.iter_mut() {
        let modified = inspector_field_is_modified(
            decoration,
            style,
            &default_style,
            widget_current.as_deref(),
            widget_default.as_deref(),
        );
        background.0 = if modified { modified_bg } else { Color::NONE };
        *border = BorderColor::all(if modified {
            modified_border
        } else {
            Color::NONE
        });
    }

    for (decoration, mut node) in reset_buttons.iter_mut() {
        node.display = if inspector_field_is_modified(
            decoration,
            style,
            &default_style,
            widget_current.as_deref(),
            widget_default.as_deref(),
        ) {
            Display::Flex
        } else {
            Display::None
        };
    }
}

pub(crate) fn inspector_field_is_modified(
    decoration: &InspectorFieldDecoration,
    style: Option<&WidgetStyle>,
    default_style: &WidgetStyle,
    widget_current: Option<&dyn PartialReflect>,
    widget_default: Option<&dyn PartialReflect>,
) -> bool {
    match decoration.target {
        InspectorBindingTarget::Style => style
            .map(|value| {
                reflect_path_differs_from_default(value, default_style, &decoration.field_path)
            })
            .unwrap_or(false),
        InspectorBindingTarget::WidgetProp => match (widget_current, widget_default) {
            (Some(current), Some(default_value)) => {
                reflect_path_differs_from_default(current, default_value, &decoration.field_path)
            }
            _ => false,
        },
    }
}

fn clear_section(
    commands: &mut Commands,
    section_entity: Entity,
    children_query: &Query<&Children>,
    section_node: &mut Node,
    section_state: &mut InspectorWidgetSectionState,
) {
    if let Ok(children) = children_query.get(section_entity) {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }
    section_node.display = Display::None;
    section_state.selected_node = None;
    section_state.widget_path = None;
}
