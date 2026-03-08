use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::widget::{WidgetRegistry, WidgetStyle, spawn_blueprint_widget_content};

use super::*;

pub type BlueprintNodeId = u64;

#[derive(Debug, Clone, Copy)]
pub enum BlueprintChildRule {
    Any,
    Exact(usize),
    Range { min: usize, max: Option<usize> },
}

#[derive(Debug, Clone, Copy)]
pub struct WidgetSchema {
    pub child_rule: BlueprintChildRule,
}

impl Default for WidgetSchema {
    fn default() -> Self {
        Self {
            child_rule: BlueprintChildRule::Any,
        }
    }
}

#[derive(Resource)]
pub struct WidgetSchemaRegistry {
    schemas: HashMap<String, WidgetSchema>,
}

impl Default for WidgetSchemaRegistry {
    fn default() -> Self {
        let mut schemas = HashMap::new();
        schemas.insert(
            "layout/split_view".to_owned(),
            WidgetSchema {
                child_rule: BlueprintChildRule::Exact(2),
            },
        );
        schemas.insert(
            "layout/divider".to_owned(),
            WidgetSchema {
                child_rule: BlueprintChildRule::Exact(0),
            },
        );
        schemas.insert(
            "common/text".to_owned(),
            WidgetSchema {
                child_rule: BlueprintChildRule::Exact(0),
            },
        );
        schemas.insert(
            "common/button".to_owned(),
            WidgetSchema {
                child_rule: BlueprintChildRule::Range {
                    min: 0,
                    max: Some(1),
                },
            },
        );
        Self { schemas }
    }
}

impl WidgetSchemaRegistry {
    pub fn get_schema(&self, widget_path: &str) -> WidgetSchema {
        self.schemas.get(widget_path).copied().unwrap_or_default()
    }

    fn allows_child_count(&self, widget_path: &str, next_count: usize) -> bool {
        match self.get_schema(widget_path).child_rule {
            BlueprintChildRule::Any => true,
            BlueprintChildRule::Exact(n) => next_count <= n,
            BlueprintChildRule::Range { min: _, max } => match max {
                Some(max) => next_count <= max,
                None => true,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct WidgetBlueprintNode {
    pub id: BlueprintNodeId,
    pub name: String,
    pub widget_path: String,
    pub style: WidgetStyle,
    pub props: HashMap<String, String>,
    pub parent: Option<BlueprintNodeId>,
    pub children: Vec<BlueprintNodeId>,
}

#[derive(Resource)]
pub struct WidgetBlueprintDocument {
    pub roots: Vec<BlueprintNodeId>,
    pub nodes: HashMap<BlueprintNodeId, WidgetBlueprintNode>,
    next_id: BlueprintNodeId,
    pub dirty: bool,
    pub pending_select: Option<BlueprintNodeId>,
}

impl Default for WidgetBlueprintDocument {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            nodes: HashMap::new(),
            next_id: 1,
            dirty: true,
            pending_select: None,
        }
    }
}

impl WidgetBlueprintDocument {
    fn alloc_id(&mut self) -> BlueprintNodeId {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }
}

pub enum BlueprintCommand {
    AddRoot {
        widget_path: String,
    },
    AddChild {
        parent: BlueprintNodeId,
        widget_path: String,
    },
    RemoveNode {
        node: BlueprintNodeId,
    },
    MoveNode {
        node: BlueprintNodeId,
        new_parent: Option<BlueprintNodeId>,
        index: Option<usize>,
    },
    SetNodeStyle {
        node: BlueprintNodeId,
        style: WidgetStyle,
    },
    SetNodeName {
        node: BlueprintNodeId,
        name: String,
    },
    SetNodeProp {
        node: BlueprintNodeId,
        key: String,
        value: String,
    },
}

pub enum BlueprintCommandError {
    UnknownWidgetPath(String),
    ParentNotFound(BlueprintNodeId),
    NodeNotFound(BlueprintNodeId),
    InvalidMove,
    ChildConstraintViolated {
        parent: BlueprintNodeId,
        parent_widget: String,
    },
}

pub fn apply_blueprint_command(
    command: BlueprintCommand,
    document: &mut WidgetBlueprintDocument,
    schemas: &WidgetSchemaRegistry,
    widget_registry: &WidgetRegistry,
) -> Result<BlueprintNodeId, BlueprintCommandError> {
    match command {
        BlueprintCommand::AddRoot { widget_path } => {
            if widget_registry.get_widget_by_path(&widget_path).is_none() {
                return Err(BlueprintCommandError::UnknownWidgetPath(widget_path));
            }

            let id = document.alloc_id();
            document.nodes.insert(
                id,
                WidgetBlueprintNode {
                    id,
                    name: default_blueprint_node_name(&widget_path),
                    widget_path,
                    style: WidgetStyle::default(),
                    props: HashMap::new(),
                    parent: None,
                    children: Vec::new(),
                },
            );
            document.roots.push(id);
            document.dirty = true;
            document.pending_select = Some(id);
            Ok(id)
        }
        BlueprintCommand::AddChild {
            parent,
            widget_path,
        } => {
            if widget_registry.get_widget_by_path(&widget_path).is_none() {
                return Err(BlueprintCommandError::UnknownWidgetPath(widget_path));
            }

            let Some(parent_node) = document.nodes.get(&parent) else {
                return Err(BlueprintCommandError::ParentNotFound(parent));
            };
            let parent_widget = parent_node.widget_path.clone();
            let next_count = parent_node.children.len() + 1;
            if !schemas.allows_child_count(&parent_widget, next_count) {
                return Err(BlueprintCommandError::ChildConstraintViolated {
                    parent,
                    parent_widget,
                });
            }

            let id = document.alloc_id();
            document.nodes.insert(
                id,
                WidgetBlueprintNode {
                    id,
                    name: default_blueprint_node_name(&widget_path),
                    widget_path,
                    style: WidgetStyle::default(),
                    props: HashMap::new(),
                    parent: Some(parent),
                    children: Vec::new(),
                },
            );
            if let Some(parent_node) = document.nodes.get_mut(&parent) {
                parent_node.children.push(id);
            }
            document.dirty = true;
            document.pending_select = Some(id);
            Ok(id)
        }
        BlueprintCommand::RemoveNode { node } => {
            if !document.nodes.contains_key(&node) {
                return Err(BlueprintCommandError::NodeNotFound(node));
            }
            let fallback = document
                .nodes
                .get(&node)
                .and_then(|n| n.parent)
                .or_else(|| document.roots.first().copied().filter(|r| *r != node));
            remove_node_subtree(document, node);
            document.dirty = true;
            document.pending_select = fallback;
            Ok(node)
        }
        BlueprintCommand::MoveNode {
            node,
            new_parent,
            index,
        } => {
            if !document.nodes.contains_key(&node) {
                return Err(BlueprintCommandError::NodeNotFound(node));
            }

            if let Some(parent) = new_parent {
                if !document.nodes.contains_key(&parent) {
                    return Err(BlueprintCommandError::ParentNotFound(parent));
                }
                if parent == node || is_descendant(document, parent, node) {
                    return Err(BlueprintCommandError::InvalidMove);
                }
            }

            let old_parent = document.nodes.get(&node).and_then(|n| n.parent);
            if old_parent == new_parent {
                if let Some(parent) = new_parent {
                    if let Some(parent_node) = document.nodes.get_mut(&parent)
                        && let Some(pos) = parent_node.children.iter().position(|id| *id == node)
                    {
                        let mut insert_at = index.unwrap_or(parent_node.children.len());
                        if insert_at > parent_node.children.len() {
                            insert_at = parent_node.children.len();
                        }
                        let id = parent_node.children.remove(pos);
                        if insert_at > pos {
                            insert_at -= 1;
                        }
                        parent_node.children.insert(insert_at, id);
                        document.dirty = true;
                    }
                } else {
                    if let Some(pos) = document.roots.iter().position(|id| *id == node) {
                        let mut insert_at = index.unwrap_or(document.roots.len());
                        if insert_at > document.roots.len() {
                            insert_at = document.roots.len();
                        }
                        let id = document.roots.remove(pos);
                        if insert_at > pos {
                            insert_at -= 1;
                        }
                        document.roots.insert(insert_at, id);
                        document.dirty = true;
                    }
                }
                return Ok(node);
            }

            if let Some(parent) = new_parent {
                let parent_widget = document
                    .nodes
                    .get(&parent)
                    .map(|n| n.widget_path.clone())
                    .ok_or(BlueprintCommandError::ParentNotFound(parent))?;
                let current_child_count = document
                    .nodes
                    .get(&parent)
                    .map(|n| n.children.len())
                    .unwrap_or(0);
                if !schemas.allows_child_count(&parent_widget, current_child_count + 1) {
                    return Err(BlueprintCommandError::ChildConstraintViolated {
                        parent,
                        parent_widget,
                    });
                }
            }

            if let Some(parent) = old_parent {
                if let Some(parent_node) = document.nodes.get_mut(&parent) {
                    parent_node.children.retain(|id| *id != node);
                }
            } else {
                document.roots.retain(|id| *id != node);
            }

            if let Some(parent) = new_parent {
                if let Some(parent_node) = document.nodes.get_mut(&parent) {
                    let mut insert_at = index.unwrap_or(parent_node.children.len());
                    if insert_at > parent_node.children.len() {
                        insert_at = parent_node.children.len();
                    }
                    parent_node.children.insert(insert_at, node);
                }
            } else {
                let mut insert_at = index.unwrap_or(document.roots.len());
                if insert_at > document.roots.len() {
                    insert_at = document.roots.len();
                }
                document.roots.insert(insert_at, node);
            }

            if let Some(node_mut) = document.nodes.get_mut(&node) {
                node_mut.parent = new_parent;
            }
            document.dirty = true;
            document.pending_select = Some(node);
            Ok(node)
        }
        BlueprintCommand::SetNodeStyle { node, style } => {
            let Some(node_data) = document.nodes.get_mut(&node) else {
                return Err(BlueprintCommandError::NodeNotFound(node));
            };
            node_data.style = style;
            document.dirty = true;
            document.pending_select = Some(node);
            Ok(node)
        }
        BlueprintCommand::SetNodeName { node, name } => {
            let Some(node_data) = document.nodes.get_mut(&node) else {
                return Err(BlueprintCommandError::NodeNotFound(node));
            };
            node_data.name = if name.trim().is_empty() {
                default_blueprint_node_name(&node_data.widget_path)
            } else {
                name
            };
            document.pending_select = Some(node);
            Ok(node)
        }
        BlueprintCommand::SetNodeProp { node, key, value } => {
            let Some(node_data) = document.nodes.get_mut(&node) else {
                return Err(BlueprintCommandError::NodeNotFound(node));
            };
            node_data.props.insert(key, value);
            document.dirty = true;
            document.pending_select = Some(node);
            Ok(node)
        }
    }
}

pub fn default_blueprint_node_name(widget_path: &str) -> String {
    widget_path
        .split('/')
        .next_back()
        .map(str::to_owned)
        .unwrap_or_else(|| widget_path.to_owned())
}

fn remove_node_subtree(document: &mut WidgetBlueprintDocument, node: BlueprintNodeId) {
    let (parent, children) = match document.nodes.get(&node) {
        Some(n) => (n.parent, n.children.clone()),
        None => return,
    };

    if let Some(parent) = parent {
        if let Some(parent_node) = document.nodes.get_mut(&parent) {
            parent_node.children.retain(|id| *id != node);
        }
    } else {
        document.roots.retain(|id| *id != node);
    }

    for child in children {
        remove_node_subtree(document, child);
    }
    document.nodes.remove(&node);
}

fn is_descendant(
    document: &WidgetBlueprintDocument,
    candidate: BlueprintNodeId,
    ancestor: BlueprintNodeId,
) -> bool {
    let mut cursor = Some(candidate);
    while let Some(node) = cursor {
        if node == ancestor {
            return true;
        }
        cursor = document.nodes.get(&node).and_then(|n| n.parent);
    }
    false
}

#[derive(Component, Copy, Clone)]
pub struct BlueprintNodeRef(pub BlueprintNodeId);

#[derive(Resource, Default)]
pub struct BlueprintRuntimeMap {
    pub node_to_entity: HashMap<BlueprintNodeId, Entity>,
    pub entity_to_node: HashMap<Entity, BlueprintNodeId>,
}

pub(super) fn compile_blueprint_document(
    mut commands: Commands,
    widget_registry: Res<WidgetRegistry>,
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
    schemas: Res<WidgetSchemaRegistry>,
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
        &schemas,
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
    theme: Option<&Theme>,
    parent: Entity,
    node_id: BlueprintNodeId,
) {
    let Some(node) = document.nodes.get(&node_id) else {
        return;
    };
    let Some(content) = spawn_blueprint_widget_content(
        widget_registry,
        commands,
        &node.widget_path,
        &node.style,
        theme,
    ) else {
        return;
    };

    let entity =
        viewport::spawn_canvas_widget_instance(commands, parent, content, &node.widget_path);
    commands.entity(entity).insert(BlueprintNodeRef(node_id));
    runtime_map.node_to_entity.insert(node_id, entity);
    runtime_map.entity_to_node.insert(entity, node_id);

    for child in node.children.iter().copied() {
        compile_node_recursive(
            commands,
            document,
            runtime_map,
            widget_registry,
            theme,
            entity,
            child,
        );
    }
}
