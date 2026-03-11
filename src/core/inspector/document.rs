use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::fmt;

use crate::widget::{WidgetChildRule, WidgetRegistry, WidgetStyle};

pub type BlueprintNodeId = u64;

#[derive(Debug, Clone)]
pub struct WidgetBlueprintNode {
    pub id: BlueprintNodeId,
    pub name: String,
    pub widget_path: String,
    pub style: WidgetStyle,
    pub props: HashMap<String, String>,
    pub parent: Option<BlueprintNodeId>,
    pub slot: Option<String>,
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

    pub fn from_parts(
        roots: Vec<BlueprintNodeId>,
        nodes: HashMap<BlueprintNodeId, WidgetBlueprintNode>,
    ) -> Self {
        let next_id = nodes.keys().copied().max().unwrap_or(0).saturating_add(1);
        Self {
            roots,
            nodes,
            next_id,
            dirty: true,
            pending_select: None,
        }
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
    RemoveNodeProp {
        node: BlueprintNodeId,
        key: String,
    },
}

#[derive(Debug)]
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

impl fmt::Display for BlueprintCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownWidgetPath(path) => write!(f, "unknown widget path: {path}"),
            Self::ParentNotFound(parent) => write!(f, "parent node not found: {parent}"),
            Self::NodeNotFound(node) => write!(f, "node not found: {node}"),
            Self::InvalidMove => write!(f, "invalid node move"),
            Self::ChildConstraintViolated {
                parent,
                parent_widget,
            } => write!(
                f,
                "child constraint violated for parent {parent} ({parent_widget})"
            ),
        }
    }
}

pub fn apply_blueprint_command(
    command: BlueprintCommand,
    document: &mut WidgetBlueprintDocument,
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
                    slot: None,
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
            if !allows_child_count(widget_registry, &parent_widget, next_count) {
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
                    slot: None,
                    children: Vec::new(),
                },
            );
            if let Some(parent_node) = document.nodes.get_mut(&parent) {
                parent_node.children.push(id);
            }
            refresh_child_slots_for_parent(document, parent, widget_registry);
            document.dirty = true;
            document.pending_select = Some(id);
            Ok(id)
        }
        BlueprintCommand::RemoveNode { node } => {
            if !document.nodes.contains_key(&node) {
                return Err(BlueprintCommandError::NodeNotFound(node));
            }
            let parent_before_remove = document.nodes.get(&node).and_then(|n| n.parent);
            let fallback = document
                .nodes
                .get(&node)
                .and_then(|n| n.parent)
                .or_else(|| document.roots.first().copied().filter(|r| *r != node));
            remove_node_subtree(document, node);
            if let Some(parent) = parent_before_remove {
                refresh_child_slots_for_parent(document, parent, widget_registry);
            }
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
                        refresh_child_slots_for_parent(document, parent, widget_registry);
                        document.dirty = true;
                    }
                } else if let Some(pos) = document.roots.iter().position(|id| *id == node) {
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
                if !allows_child_count(widget_registry, &parent_widget, current_child_count + 1) {
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
                node_mut.slot = None;
            }
            if let Some(parent) = old_parent {
                refresh_child_slots_for_parent(document, parent, widget_registry);
            }
            if let Some(parent) = new_parent {
                refresh_child_slots_for_parent(document, parent, widget_registry);
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
        BlueprintCommand::RemoveNodeProp { node, key } => {
            let Some(node_data) = document.nodes.get_mut(&node) else {
                return Err(BlueprintCommandError::NodeNotFound(node));
            };
            node_data.props.remove(&key);
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

#[derive(Component, Copy, Clone)]
pub struct BlueprintNodeRef;

#[derive(Resource, Default)]
pub struct BlueprintRuntimeMap {
    pub node_to_entity: HashMap<BlueprintNodeId, Entity>,
    pub entity_to_node: HashMap<Entity, BlueprintNodeId>,
}

fn allows_child_count(
    widget_registry: &WidgetRegistry,
    widget_path: &str,
    next_count: usize,
) -> bool {
    let Some(registration) = widget_registry.get_widget_by_path(widget_path) else {
        return true;
    };
    match registration.child_rule() {
        WidgetChildRule::Any => true,
        WidgetChildRule::Exact(n) => next_count <= n,
        WidgetChildRule::Range { max } => match max {
            Some(max) => next_count <= max,
            None => true,
        },
    }
}

fn refresh_child_slots_for_parent(
    document: &mut WidgetBlueprintDocument,
    parent: BlueprintNodeId,
    widget_registry: &WidgetRegistry,
) {
    let Some(parent_node) = document.nodes.get(&parent) else {
        return;
    };
    let Some(registration) = widget_registry.get_widget_by_path(&parent_node.widget_path) else {
        return;
    };
    let children = parent_node.children.clone();
    for (index, child) in children.into_iter().enumerate() {
        if let Some(child_node) = document.nodes.get_mut(&child) {
            child_node.slot = registration.child_slot_at(index).map(str::to_owned);
        }
    }
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
