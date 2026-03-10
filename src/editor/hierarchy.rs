use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::widget::{
    TreeNodeBuilder, TreeNodeHeader, TreeNodeItemId, TreeViewBuilder, spawn_tree_node,
};

use super::*;

#[derive(Resource)]
pub(crate) struct HierarchyState {
    pub dirty: bool,
}

impl Default for HierarchyState {
    fn default() -> Self {
        Self { dirty: true }
    }
}

#[derive(Resource, Default)]
pub(super) struct HierarchyDragState {
    dragging_node: Option<blueprint::BlueprintNodeId>,
    hover_target: Option<blueprint::BlueprintNodeId>,
    ghost_entity: Option<Entity>,
    source_header: Option<Entity>,
}

#[derive(Resource, Default)]
pub(super) struct HierarchyTreeCache {
    tree_content: Option<Entity>,
    nodes: HashMap<blueprint::BlueprintNodeId, HierarchyNodeUi>,
}

#[derive(Clone, Copy)]
struct HierarchyNodeUi {
    node: Entity,
    children_container: Entity,
    label: Entity,
}

#[derive(Component)]
pub(super) struct HierarchyContentRoot;

#[derive(Component)]
pub(super) struct HierarchyItemObserverBound;

pub(super) fn init_hierarchy_panel(
    mut commands: Commands,
    hierarchy: Single<Entity, With<Hierarchy>>,
    editor_theme: Res<EditorTheme>,
    mut cache: ResMut<HierarchyTreeCache>,
) {
    commands
        .entity(*hierarchy)
        .insert(BackgroundColor(editor_theme.0.palette.surface));

    let content = commands
        .spawn((
            Name::new("Hierarchy Content"),
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(2.0),
                padding: UiRect::all(px(4.0)),
                ..default()
            },
            HierarchyContentRoot,
        ))
        .observe(on_hierarchy_content_drag_end)
        .observe(on_hierarchy_content_drag_cancel)
        .id();

    let tree = TreeViewBuilder::new()
        .width(percent(100.0))
        .height(percent(100.0))
        .item_gap(2.0)
        .padding(UiRect::all(px(0.0)))
        .build_with_result(
            &mut commands,
            std::iter::empty::<TreeNodeBuilder>(),
            Some(&editor_theme.0),
        );
    cache.tree_content = Some(tree.content);

    commands.entity(*hierarchy).add_child(content);
    commands.entity(content).add_child(tree.root);
}

pub(super) fn refresh_hierarchy_panel(
    mut commands: Commands,
    mut state: ResMut<HierarchyState>,
    drag_state: Res<HierarchyDragState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    editor_theme: Res<EditorTheme>,
    mut cache: ResMut<HierarchyTreeCache>,
    parents: Query<&ChildOf>,
) {
    if drag_state.dragging_node.is_some() || !state.dirty {
        return;
    }
    let Some(tree_content) = cache.tree_content else {
        return;
    };

    let mut alive = HashSet::new();
    let mut root_entities = Vec::with_capacity(document.roots.len());
    for root in document.roots.iter().copied() {
        if let Some(entity) = ensure_hierarchy_node(
            &mut commands,
            &document,
            &mut cache,
            root,
            &mut alive,
            Some(&editor_theme.0),
        ) {
            root_entities.push(entity);
        }
    }
    commands
        .entity(tree_content)
        .replace_children(&root_entities);

    let stale = cache
        .nodes
        .keys()
        .copied()
        .filter(|id| !alive.contains(id))
        .collect::<Vec<_>>();
    let stale_entities = stale
        .iter()
        .filter_map(|id| cache.nodes.get(id).map(|ui| ui.node))
        .collect::<HashSet<_>>();
    for id in stale {
        if let Some(ui) = cache.nodes.remove(&id) {
            let is_child_of_stale = parents
                .get(ui.node)
                .map(|parent| stale_entities.contains(&parent.parent()))
                .unwrap_or(false);
            if !is_child_of_stale {
                commands.entity(ui.node).despawn();
            }
        }
    }

    state.dirty = false;
}

pub(super) fn attach_hierarchy_tree_item_observers(
    mut commands: Commands,
    hierarchy_content: Single<Entity, With<HierarchyContentRoot>>,
    parents: Query<&ChildOf>,
    headers: Query<
        Entity,
        (
            With<TreeNodeHeader>,
            With<TreeNodeItemId>,
            Without<HierarchyItemObserverBound>,
        ),
    >,
) {
    for header in headers.iter() {
        if !is_descendant_of(header, *hierarchy_content, &parents) {
            continue;
        }
        commands
            .entity(header)
            .observe(on_hierarchy_item_click)
            .observe(on_hierarchy_item_drag_start)
            .observe(on_hierarchy_item_drag)
            .observe(on_hierarchy_item_over)
            .observe(on_hierarchy_item_out)
            .observe(on_hierarchy_item_drag_end)
            .observe(on_hierarchy_item_drag_cancel)
            .insert(HierarchyItemObserverBound);
    }
}

fn on_hierarchy_item_click(
    mut event: On<Pointer<Click>>,
    options: Res<VistaEditorViewOptions>,
    runtime_map: Res<blueprint::BlueprintRuntimeMap>,
    parents: Query<&ChildOf>,
    headers: Query<(), (With<TreeNodeHeader>, With<TreeNodeItemId>)>,
    item_ids: Query<&TreeNodeItemId>,
    mut selection: ResMut<VistaEditorSelection>,
) {
    if options.is_preview_mode {
        return;
    }
    let Some(header) = find_ancestor_with(event.event_target(), &parents, |entity| {
        headers.contains(entity)
    }) else {
        return;
    };
    let Ok(item_id) = item_ids.get(header) else {
        return;
    };
    selection.selected_entity = runtime_map.node_to_entity.get(&item_id.0).copied();
    event.propagate(false);
}

fn on_hierarchy_item_drag_start(
    mut event: On<Pointer<DragStart>>,
    mut commands: Commands,
    options: Res<VistaEditorViewOptions>,
    parents: Query<&ChildOf>,
    headers: Query<(), (With<TreeNodeHeader>, With<TreeNodeItemId>)>,
    item_ids: Query<&TreeNodeItemId>,
    mut row_bg: Query<&mut BackgroundColor>,
    mut drag: ResMut<HierarchyDragState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    editor_root: Single<Entity, With<EditorRoot>>,
    editor_theme: Res<EditorTheme>,
) {
    if options.is_preview_mode {
        return;
    }
    let Some(header) = find_ancestor_with(event.event_target(), &parents, |entity| {
        headers.contains(entity)
    }) else {
        return;
    };
    let Ok(item_id) = item_ids.get(header) else {
        return;
    };

    if let Some(ghost) = drag.ghost_entity.take() {
        commands.entity(ghost).despawn();
    }

    let label = document
        .nodes
        .get(&item_id.0)
        .map(|node| node.name.clone())
        .unwrap_or_else(|| "Node".to_owned());
    let theme = &editor_theme.0;
    let ghost_bg = theme.palette.surface_variant.lighter(0.08).with_alpha(0.9);
    let text_color = theme.palette.on_surface;
    let font = theme.typography.body_medium.font.clone();
    let pointer = event.event().pointer_location.position;
    let ghost = commands
        .spawn((
            Name::new("Hierarchy Drag Ghost"),
            Node {
                position_type: PositionType::Absolute,
                left: px(pointer.x + 12.0),
                top: px(pointer.y + 12.0),
                padding: UiRect::axes(px(8.0), px(4.0)),
                ..default()
            },
            BackgroundColor(ghost_bg),
            BorderRadius::all(px(5.0)),
            GlobalZIndex(EDITOR_DEFAULT_Z_INDEX + 100),
            children![(Text::new(label), font, TextColor(text_color),)],
        ))
        .id();
    commands.entity(*editor_root).add_child(ghost);

    drag.dragging_node = Some(item_id.0);
    drag.hover_target = None;
    drag.ghost_entity = Some(ghost);
    drag.source_header = Some(header);
    if let Ok(mut bg) = row_bg.get_mut(header) {
        bg.0 = bg.0.with_alpha(0.5);
    }
    event.propagate(false);
}

fn on_hierarchy_item_drag(
    event: On<Pointer<Drag>>,
    drag: Res<HierarchyDragState>,
    mut nodes: Query<&mut Node>,
) {
    let Some(ghost) = drag.ghost_entity else {
        return;
    };
    let Ok(mut node) = nodes.get_mut(ghost) else {
        return;
    };
    let pointer = event.event().pointer_location.position;
    node.left = px(pointer.x + 12.0);
    node.top = px(pointer.y + 12.0);
}

fn on_hierarchy_item_over(
    event: On<Pointer<Over>>,
    mut drag: ResMut<HierarchyDragState>,
    parents: Query<&ChildOf>,
    headers: Query<(), (With<TreeNodeHeader>, With<TreeNodeItemId>)>,
    item_ids: Query<&TreeNodeItemId>,
    mut row_bg: Query<&mut BackgroundColor>,
    editor_theme: Res<EditorTheme>,
) {
    if drag.dragging_node.is_none() {
        return;
    }
    let Some(header) = find_ancestor_with(event.event_target(), &parents, |entity| {
        headers.contains(entity)
    }) else {
        return;
    };
    let Ok(item_id) = item_ids.get(header) else {
        return;
    };
    if drag.dragging_node == Some(item_id.0) {
        drag.hover_target = None;
        return;
    }
    drag.hover_target = Some(item_id.0);
    let hover_color = editor_theme.0.palette.outline_variant;
    if let Ok(mut bg) = row_bg.get_mut(header) {
        bg.0 = hover_color;
    }
}

fn on_hierarchy_item_out(
    event: On<Pointer<Out>>,
    mut drag: ResMut<HierarchyDragState>,
    parents: Query<&ChildOf>,
    headers: Query<(), (With<TreeNodeHeader>, With<TreeNodeItemId>)>,
    item_ids: Query<&TreeNodeItemId>,
    mut row_bg: Query<&mut BackgroundColor>,
    editor_theme: Res<EditorTheme>,
) {
    if drag.dragging_node.is_none() {
        return;
    }
    let Some(header) = find_ancestor_with(event.event_target(), &parents, |entity| {
        headers.contains(entity)
    }) else {
        return;
    };
    let Ok(item_id) = item_ids.get(header) else {
        return;
    };
    if drag.hover_target == Some(item_id.0) {
        drag.hover_target = None;
    }
    let normal_bg = editor_theme.0.palette.surface_variant;
    if let Ok(mut bg) = row_bg.get_mut(header) {
        bg.0 = normal_bg;
    }
}

fn on_hierarchy_item_drag_end(
    mut event: On<Pointer<DragEnd>>,
    mut commands: Commands,
    options: Res<VistaEditorViewOptions>,
    parents: Query<&ChildOf>,
    headers: Query<(), (With<TreeNodeHeader>, With<TreeNodeItemId>)>,
    item_ids: Query<&TreeNodeItemId>,
    mut row_bg: Query<&mut BackgroundColor>,
    mut drag: ResMut<HierarchyDragState>,
    widget_registry: Res<WidgetRegistry>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    mut state: ResMut<HierarchyState>,
) {
    if options.is_preview_mode {
        return;
    }
    let header = find_ancestor_with(event.event_target(), &parents, |entity| {
        headers.contains(entity)
    });
    let source = drag
        .dragging_node
        .or_else(|| header.and_then(|entity| item_ids.get(entity).ok().map(|item| item.0)));
    let Some(source) = source else {
        cleanup_hierarchy_drag_visual(&mut commands, &mut drag, &mut row_bg);
        state.dirty = true;
        return;
    };

    match drag.hover_target {
        Some(target) if target != source => {
            let _ = blueprint::apply_blueprint_command(
                blueprint::BlueprintCommand::MoveNode {
                    node: source,
                    new_parent: Some(target),
                    index: None,
                },
                &mut document,
                &widget_registry,
            );
            state.dirty = true;
        }
        None => {
            let _ = blueprint::apply_blueprint_command(
                blueprint::BlueprintCommand::MoveNode {
                    node: source,
                    new_parent: None,
                    index: None,
                },
                &mut document,
                &widget_registry,
            );
            state.dirty = true;
        }
        _ => {}
    }

    cleanup_hierarchy_drag_visual(&mut commands, &mut drag, &mut row_bg);
    event.propagate(false);
}

fn on_hierarchy_item_drag_cancel(
    mut event: On<Pointer<Cancel>>,
    mut commands: Commands,
    mut row_bg: Query<&mut BackgroundColor>,
    mut state: ResMut<HierarchyState>,
    mut drag: ResMut<HierarchyDragState>,
) {
    cleanup_hierarchy_drag_visual(&mut commands, &mut drag, &mut row_bg);
    state.dirty = true;
    event.propagate(false);
}

fn on_hierarchy_content_drag_end(
    mut event: On<Pointer<DragEnd>>,
    mut commands: Commands,
    options: Res<VistaEditorViewOptions>,
    mut row_bg: Query<&mut BackgroundColor>,
    mut drag: ResMut<HierarchyDragState>,
    widget_registry: Res<WidgetRegistry>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    mut state: ResMut<HierarchyState>,
) {
    if options.is_preview_mode {
        return;
    }
    let Some(source) = drag.dragging_node else {
        return;
    };

    if drag.hover_target.is_none() {
        let _ = blueprint::apply_blueprint_command(
            blueprint::BlueprintCommand::MoveNode {
                node: source,
                new_parent: None,
                index: None,
            },
            &mut document,
            &widget_registry,
        );
    }

    cleanup_hierarchy_drag_visual(&mut commands, &mut drag, &mut row_bg);
    state.dirty = true;
    event.propagate(false);
}

fn on_hierarchy_content_drag_cancel(
    mut event: On<Pointer<Cancel>>,
    mut commands: Commands,
    mut row_bg: Query<&mut BackgroundColor>,
    mut state: ResMut<HierarchyState>,
    mut drag: ResMut<HierarchyDragState>,
) {
    cleanup_hierarchy_drag_visual(&mut commands, &mut drag, &mut row_bg);
    state.dirty = true;
    event.propagate(false);
}

fn cleanup_hierarchy_drag_visual(
    commands: &mut Commands,
    drag: &mut HierarchyDragState,
    row_bg: &mut Query<&mut BackgroundColor>,
) {
    if let Some(ghost) = drag.ghost_entity.take() {
        commands.entity(ghost).despawn();
    }
    if let Some(header) = drag.source_header.take()
        && let Ok(mut bg) = row_bg.get_mut(header)
    {
        bg.0 = bg.0.with_alpha(1.0);
    }
    drag.dragging_node = None;
    drag.hover_target = None;
}

fn ensure_hierarchy_node(
    commands: &mut Commands,
    document: &blueprint::WidgetBlueprintDocument,
    cache: &mut HierarchyTreeCache,
    node_id: blueprint::BlueprintNodeId,
    alive: &mut HashSet<blueprint::BlueprintNodeId>,
    theme: Option<&Theme>,
) -> Option<Entity> {
    let node = document.nodes.get(&node_id)?;
    alive.insert(node_id);

    let label = node.name.clone();

    let ui = if let Some(ui) = cache.nodes.get(&node_id).copied() {
        commands.entity(ui.label).insert(Text::new(label));
        ui
    } else {
        let builder = if node.children.is_empty() {
            TreeNodeBuilder::leaf(label)
        } else {
            TreeNodeBuilder::branch(label, true, Vec::new())
        }
        .with_item_id(node_id);
        let spawned = spawn_tree_node(commands, builder, theme, 14.0, 2.0);
        let ui = HierarchyNodeUi {
            node: spawned.node,
            children_container: spawned.children_container,
            label: spawned.label,
        };
        cache.nodes.insert(node_id, ui);
        ui
    };

    let mut child_entities = Vec::with_capacity(node.children.len());
    for child in node.children.iter().copied() {
        if let Some(entity) = ensure_hierarchy_node(commands, document, cache, child, alive, theme)
        {
            child_entities.push(entity);
        }
    }
    commands
        .entity(ui.children_container)
        .replace_children(&child_entities);
    Some(ui.node)
}

fn is_descendant_of(entity: Entity, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    find_ancestor_with(entity, parents, |e| e == ancestor).is_some()
}

fn find_ancestor_with<F>(
    mut current: Entity,
    parents: &Query<&ChildOf>,
    mut predicate: F,
) -> Option<Entity>
where
    F: FnMut(Entity) -> bool,
{
    loop {
        if predicate(current) {
            return Some(current);
        }
        let Ok(parent) = parents.get(current) else {
            return None;
        };
        current = parent.parent();
    }
}
