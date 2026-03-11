use bevy::prelude::*;

use crate::inspector::InspectorEditorRegistry;
use crate::widget::{WidgetRegistry, spawn_blueprint_widget_content};

use super::*;

pub use crate::inspector::{
    BlueprintCommand, BlueprintNodeId, BlueprintNodeRef, BlueprintRuntimeMap,
    WidgetBlueprintDocument, apply_blueprint_command,
};

pub(super) fn compile_blueprint_document(
    mut commands: Commands,
    widget_registry: Res<WidgetRegistry>,
    inspector_registry: Res<InspectorEditorRegistry>,
    viewport_theme: Res<ViewportThemeState>,
    elements_container: Single<Entity, With<viewport::ElementsContainer>>,
    container_children: Query<&Children>,
    mut document: ResMut<WidgetBlueprintDocument>,
    mut runtime_map: ResMut<BlueprintRuntimeMap>,
    mut hierarchy: ResMut<hierarchy::HierarchyState>,
    mut selection: ResMut<VistaEditorSelection>,
) {
    if !document.dirty && !viewport_theme.is_changed() {
        return;
    }

    if let Ok(existing) = container_children.get(*elements_container) {
        for entity in existing.iter() {
            commands.entity(entity).despawn();
        }
    }

    runtime_map.node_to_entity.clear();
    runtime_map.entity_to_node.clear();

    let roots = document.roots.clone();
    for root_id in roots {
        compile_node_recursive(
            &mut commands,
            &document,
            &mut runtime_map,
            &widget_registry,
            &inspector_registry,
            viewport_theme.active_theme(),
            *elements_container,
            root_id,
        );
    }

    if let Some(node_id) = document.pending_select.take() {
        selection.selected_entity = runtime_map.node_to_entity.get(&node_id).copied();
    }

    document.dirty = false;
    hierarchy.dirty = true;
}

pub(super) fn delete_selected_blueprint_node_shortcut(
    key_input: Res<ButtonInput<KeyCode>>,
    options: Res<VistaEditorViewOptions>,
    mut selection: ResMut<VistaEditorSelection>,
    runtime_map: Res<BlueprintRuntimeMap>,
    widget_registry: Res<WidgetRegistry>,
    mut document: ResMut<WidgetBlueprintDocument>,
    mut hierarchy: ResMut<hierarchy::HierarchyState>,
) {
    if options.is_preview_mode || !key_input.just_pressed(KeyCode::Delete) {
        return;
    }

    let Some(selected_entity) = selection.selected_entity else {
        return;
    };
    let Some(node_id) = runtime_map.entity_to_node.get(&selected_entity).copied() else {
        return;
    };

    if apply_blueprint_command(
        BlueprintCommand::RemoveNode { node: node_id },
        &mut document,
        &widget_registry,
    )
    .is_ok()
    {
        hierarchy.dirty = true;
        selection.selected_entity = None;
    }
}

fn compile_node_recursive(
    commands: &mut Commands,
    document: &WidgetBlueprintDocument,
    runtime_map: &mut BlueprintRuntimeMap,
    widget_registry: &WidgetRegistry,
    inspector_registry: &InspectorEditorRegistry,
    theme: Option<&Theme>,
    parent: Entity,
    node_id: BlueprintNodeId,
) {
    let Some(node) = document.nodes.get(&node_id) else {
        return;
    };
    let Some(spawn) = spawn_blueprint_widget_content(
        widget_registry,
        inspector_registry,
        commands,
        &node.widget_path,
        &node.style,
        &node.props,
        theme,
    ) else {
        return;
    };

    let entity =
        viewport::spawn_canvas_widget_instance(commands, parent, spawn.root, &node.widget_path);
    commands.entity(entity).insert(BlueprintNodeRef);
    runtime_map.node_to_entity.insert(node_id, entity);
    runtime_map.entity_to_node.insert(entity, node_id);

    for (index, child) in node.children.iter().copied().enumerate() {
        let child_parent = resolve_child_parent_entity(document, &spawn, child, index);
        compile_node_recursive(
            commands,
            document,
            runtime_map,
            widget_registry,
            inspector_registry,
            theme,
            child_parent,
            child,
        );
    }
}

fn resolve_child_parent_entity(
    document: &WidgetBlueprintDocument,
    parent_spawn: &crate::widget::WidgetSpawnResult,
    child_node_id: BlueprintNodeId,
    child_index: usize,
) -> Entity {
    let slot = document
        .nodes
        .get(&child_node_id)
        .and_then(|node| node.slot.as_deref())
        .or(match child_index {
            0 => Some("first"),
            1 => Some("second"),
            _ => None,
        });

    slot.and_then(|slot| parent_spawn.slot_entity(slot))
        .unwrap_or(parent_spawn.root)
}
