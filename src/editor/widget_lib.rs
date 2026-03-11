use bevy::window::PrimaryWindow;

use crate::widget::{TreeNodeBuilder, TreeNodeState, TreeViewBuilder, WidgetRegistry};

use super::*;

#[derive(Resource, Default)]
pub(super) struct WidgetLibDragState {
    widget_id: Option<String>,
    ghost_entity: Option<Entity>,
}

#[derive(Component)]
pub(super) struct WidgetLibDragObserverBound;

pub fn init_widget_lib_panel(
    mut commands: Commands,
    widget_lib: Single<Entity, With<WidgetLib>>,
    widget_registry: Res<WidgetRegistry>,
    editor_theme: Res<EditorTheme>,
) {
    commands
        .entity(*widget_lib)
        .insert(BackgroundColor(editor_theme.0.palette.surface));

    let mut tree_nodes: Vec<TreeNodeBuilder> = Vec::new();

    let widgets = widget_registry.get_all_widgets();
    for w in widgets {
        let category_index = tree_nodes
            .iter()
            .position(|node| node.label == w.category());
        if let Some(index) = category_index {
            tree_nodes[index]
                .children
                .push(TreeNodeBuilder::leaf(w.name()));
        } else {
            tree_nodes.push(TreeNodeBuilder::branch(
                w.category(),
                false,
                vec![TreeNodeBuilder::leaf(w.name())],
            ));
        }
    }

    let widget_tree = TreeViewBuilder::new()
        .height(percent(100.))
        .width(percent(100.))
        .item_gap(2.0)
        .build(&mut commands, tree_nodes, Some(&editor_theme.0));

    commands.entity(*widget_lib).add_child(widget_tree);
}

pub(super) fn attach_widget_lib_tree_item_observers(
    mut commands: Commands,
    widget_lib: Single<Entity, With<WidgetLib>>,
    parents: Query<&ChildOf>,
    headers: Query<Entity, (With<TreeNodeHeader>, Without<WidgetLibDragObserverBound>)>,
) {
    for header in headers.iter() {
        if !is_descendant_of(header, *widget_lib, &parents) {
            continue;
        }
        commands
            .entity(header)
            .observe(on_widget_tree_drag_start)
            .observe(on_widget_tree_drag)
            .observe(on_widget_tree_drag_end)
            .observe(on_widget_tree_drag_cancel)
            .insert(WidgetLibDragObserverBound);
    }
}

pub(super) fn on_widget_tree_drag_start(
    event: On<Pointer<DragStart>>,
    mut commands: Commands,
    editor_root: Single<Entity, With<EditorRoot>>,
    tree_node_states: Query<&TreeNodeState>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    names: Query<&Name>,
    texts: Query<&Text>,
    editor_theme: Res<EditorTheme>,
    mut drag: ResMut<WidgetLibDragState>,
) {
    let Ok(ChildOf(header_parent)) = parents.get(event.entity) else {
        return;
    };

    // check if the drag started on a leaf tree node (i.e. a widget item)
    if tree_node_states
        .get(*header_parent)
        .map(|s| s.has_children())
        .unwrap_or(true)
    {
        return;
    }

    let Some(widget_id) = collect_tree_path(*header_parent, &parents, &children, &names, &texts)
    else {
        return;
    };

    if let Some(ghost) = drag.ghost_entity.take() {
        commands.entity(ghost).despawn();
    }

    let theme = &editor_theme.0;
    let ghost_bg = theme.palette.surface_variant.lighter(0.08).with_alpha(0.9);
    let text_color = theme.palette.on_surface;
    let font = theme.typography.body_medium.font.clone();

    let pointer = event.event().pointer_location.position;
    let display_name = widget_id
        .split('/')
        .next_back()
        .map(str::to_owned)
        .unwrap_or(widget_id.clone());
    let ghost_entity = commands
        .spawn((
            Name::new("Widget Drag Ghost"),
            Node {
                position_type: PositionType::Absolute,
                left: px(pointer.x + 12.0),
                top: px(pointer.y + 12.0),
                padding: UiRect::axes(px(8.0), px(4.0)),
                column_gap: px(6.0),
                ..default()
            },
            BackgroundColor(ghost_bg),
            BorderRadius::all(px(5.0)),
            GlobalZIndex(EDITOR_DEFAULT_Z_INDEX + 100),
            children![(Text::new(display_name), font, TextColor(text_color),)],
        ))
        .id();
    commands.entity(*editor_root).add_child(ghost_entity);

    drag.widget_id = Some(widget_id);
    drag.ghost_entity = Some(ghost_entity);
}

pub(super) fn on_widget_tree_drag(
    event: On<Pointer<Drag>>,
    drag: Res<WidgetLibDragState>,
    mut nodes: Query<&mut Node>,
) {
    let Some(ghost) = drag.ghost_entity else {
        return;
    };
    if drag.widget_id.is_none() {
        return;
    };
    let Ok(mut node) = nodes.get_mut(ghost) else {
        return;
    };

    let pointer = event.event().pointer_location.position;
    node.left = px(pointer.x + 12.0);
    node.top = px(pointer.y + 12.0);
}

pub(super) fn on_widget_tree_drag_end(
    _event: On<Pointer<DragEnd>>,
    mut commands: Commands,
    window: Single<&Window, With<PrimaryWindow>>,
    widget_registry: Res<WidgetRegistry>,
    runtime_map: Res<super::blueprint::BlueprintRuntimeMap>,
    elements_container: Single<
        (Entity, &ComputedNode, &UiGlobalTransform),
        With<super::viewport::ElementsContainer>,
    >,
    hierarchy_content: Single<
        (Entity, &ComputedNode, &UiGlobalTransform),
        With<super::hierarchy::HierarchyContentRoot>,
    >,
    hierarchy_items: Query<
        (Entity, &TreeNodeItemId, &ComputedNode, &UiGlobalTransform),
        With<TreeNodeHeader>,
    >,
    parents: Query<&ChildOf>,
    selection: Res<VistaEditorSelection>,
    mut document: ResMut<super::blueprint::WidgetBlueprintDocument>,
    mut hierarchy: ResMut<super::hierarchy::HierarchyState>,
    mut drag: ResMut<WidgetLibDragState>,
) {
    if drag.widget_id.is_none() {
        return;
    }

    if let Some(cursor) = window.physical_cursor_position()
        && let Some(widget_id) = drag.widget_id.as_deref()
        && widget_registry.get_widget_by_path(widget_id).is_some()
    {
        let (_container_entity, container_node, container_transform) = *elements_container;
        let mut handled = false;

        if container_node.contains_point(*container_transform, cursor) {
            let mut add_result = None;
            if let Some(selected_entity) = selection.selected_entity
                && let Some(parent_node) = runtime_map.entity_to_node.get(&selected_entity)
            {
                add_result = Some(super::blueprint::apply_blueprint_command(
                    super::blueprint::BlueprintCommand::AddChild {
                        parent: *parent_node,
                        widget_path: widget_id.to_owned(),
                    },
                    &mut document,
                    &widget_registry,
                ));
            }

            if add_result.is_none() || add_result.as_ref().is_some_and(Result::is_err) {
                let _ = super::blueprint::apply_blueprint_command(
                    super::blueprint::BlueprintCommand::AddRoot {
                        widget_path: widget_id.to_owned(),
                    },
                    &mut document,
                    &widget_registry,
                );
            }
            handled = true;
        } else {
            let (hierarchy_root, hier_node, hier_transform) = *hierarchy_content;
            if hier_node.contains_point(*hier_transform, cursor) {
                let mut target = None;
                for (header, item_id, node, transform) in hierarchy_items.iter() {
                    if !is_descendant_of(header, hierarchy_root, &parents) {
                        continue;
                    }
                    if node.contains_point(*transform, cursor) {
                        target = Some(item_id.0);
                        break;
                    }
                }

                let result = if let Some(parent) = target {
                    super::blueprint::apply_blueprint_command(
                        super::blueprint::BlueprintCommand::AddChild {
                            parent,
                            widget_path: widget_id.to_owned(),
                        },
                        &mut document,
                        &widget_registry,
                    )
                } else {
                    super::blueprint::apply_blueprint_command(
                        super::blueprint::BlueprintCommand::AddRoot {
                            widget_path: widget_id.to_owned(),
                        },
                        &mut document,
                        &widget_registry,
                    )
                };

                if result.is_err() {
                    let _ = super::blueprint::apply_blueprint_command(
                        super::blueprint::BlueprintCommand::AddRoot {
                            widget_path: widget_id.to_owned(),
                        },
                        &mut document,
                        &widget_registry,
                    );
                }
                handled = true;
            }
        }

        if handled {
            hierarchy.dirty = true;
        }
    }

    cleanup_drag_ghost(&mut commands, &mut drag);
}

pub(super) fn on_widget_tree_drag_cancel(
    _event: On<Pointer<Cancel>>,
    mut commands: Commands,
    mut drag: ResMut<WidgetLibDragState>,
) {
    if drag.widget_id.is_none() {
        return;
    }
    cleanup_drag_ghost(&mut commands, &mut drag);
}

fn cleanup_drag_ghost(commands: &mut Commands, drag: &mut WidgetLibDragState) {
    if let Some(ghost) = drag.ghost_entity.take() {
        commands.entity(ghost).despawn();
    }
    drag.widget_id = None;
}

fn collect_tree_path(
    node: Entity,
    parents: &Query<&ChildOf>,
    children: &Query<&Children>,
    names: &Query<&Name>,
    texts: &Query<&Text>,
) -> Option<String> {
    let mut labels = Vec::new();
    let mut cursor = Some(node);
    while let Some(current) = cursor {
        labels.push(node_label(current, children, names, texts)?);
        cursor = parent_tree_node(current, parents, names);
    }
    labels.reverse();
    Some(labels.join("/"))
}

fn node_label(
    node: Entity,
    children: &Query<&Children>,
    names: &Query<&Name>,
    texts: &Query<&Text>,
) -> Option<String> {
    let node_children = children.get(node).ok()?;
    let header = node_children
        .iter()
        .find(|child| is_named(*child, "Tree Node Header", names))?;
    let header_children = children.get(header).ok()?;
    for child in header_children {
        if let Ok(text) = texts.get(*child) {
            return Some(text.0.clone());
        }
    }
    None
}

fn parent_tree_node(
    node: Entity,
    parents: &Query<&ChildOf>,
    names: &Query<&Name>,
) -> Option<Entity> {
    let parent_container = parents.get(node).ok()?.parent();
    let ancestor = parents.get(parent_container).ok()?.parent();
    if is_named(ancestor, "Tree Node", names) {
        Some(ancestor)
    } else {
        None
    }
}

fn is_descendant_of(entity: Entity, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    find_ancestor_with(entity, parents, |e| e == ancestor).is_some()
}

fn is_named(entity: Entity, expected: &str, names: &Query<&Name>) -> bool {
    names
        .get(entity)
        .map(|name| name.as_str() == expected)
        .unwrap_or(false)
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
