use std::fs;
use std::path::{Path, PathBuf};

use bevy::ecs::component::Mutable;
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::asset::{VistaUiAsset, VistaUiAssetError};
use crate::inspector::runtime::InspectorControlRegistry;
use crate::inspector::{
    BlueprintCommand, BlueprintCommandError, BlueprintNodeId, BlueprintNodeRef,
    BlueprintRuntimeMap, InspectorEditorRegistry, InspectorEntryDescriptor,
    WidgetBlueprintDocument, apply_blueprint_command,
};
use crate::theme::Theme;
use crate::widget::{Widget, WidgetRegistry, WidgetSpawnResult, spawn_blueprint_widget_content};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct WidgetDocId(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct WidgetDocInstanceId(u64);

#[derive(Debug)]
pub enum WidgetDocError {
    Io(String),
    Asset(VistaUiAssetError),
    Blueprint(BlueprintCommandError),
    DocumentNotFound(WidgetDocId),
    InstanceNotFound(WidgetDocInstanceId),
    NodeNotFound(BlueprintNodeId),
    WidgetNotRegistered(&'static str),
    WidgetPathNotFound(String),
    WidgetQueryNotFound {
        widget_path: String,
        name: Option<String>,
    },
    WidgetPropsUnavailable(String),
    WidgetPropTypeMismatch {
        widget_path: String,
        expected: &'static str,
    },
    LiveNodeEntityNotFound {
        instance_id: WidgetDocInstanceId,
        node_id: BlueprintNodeId,
    },
    LiveWidgetUnavailable {
        entity: Entity,
        expected: &'static str,
    },
}

impl std::fmt::Display for WidgetDocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Asset(error) => write!(f, "{error:?}"),
            Self::Blueprint(error) => write!(f, "{error}"),
            Self::DocumentNotFound(id) => write!(f, "widget document {:?} not found", id),
            Self::InstanceNotFound(id) => write!(f, "widget document instance {:?} not found", id),
            Self::NodeNotFound(node_id) => write!(f, "widget document node {node_id} not found"),
            Self::WidgetNotRegistered(type_name) => {
                write!(f, "widget type `{type_name}` is not registered")
            }
            Self::WidgetPathNotFound(path) => write!(f, "widget path `{path}` is not registered"),
            Self::WidgetQueryNotFound { widget_path, name } => {
                if let Some(name) = name {
                    write!(f, "widget `{widget_path}` with name `{name}` not found")
                } else {
                    write!(f, "widget `{widget_path}` not found")
                }
            }
            Self::WidgetPropsUnavailable(widget_path) => {
                write!(f, "widget `{widget_path}` does not expose inspector props")
            }
            Self::WidgetPropTypeMismatch {
                widget_path,
                expected,
            } => write!(
                f,
                "widget `{widget_path}` props do not match expected type `{expected}`"
            ),
            Self::LiveNodeEntityNotFound {
                instance_id,
                node_id,
            } => write!(
                f,
                "widget document instance {:?} has no live entity for node {}",
                instance_id, node_id
            ),
            Self::LiveWidgetUnavailable { entity, expected } => write!(
                f,
                "entity {:?} does not expose live widget component `{expected}`",
                entity
            ),
        }
    }
}

impl std::error::Error for WidgetDocError {}

impl From<VistaUiAssetError> for WidgetDocError {
    fn from(value: VistaUiAssetError) -> Self {
        Self::Asset(value)
    }
}

impl From<BlueprintCommandError> for WidgetDocError {
    fn from(value: BlueprintCommandError) -> Self {
        Self::Blueprint(value)
    }
}

#[derive(Default, Resource)]
pub(crate) struct WidgetDocStore {
    next_doc_id: u64,
    next_instance_id: u64,
    documents: HashMap<WidgetDocId, WidgetDocumentRecord>,
    instances: HashMap<WidgetDocInstanceId, WidgetDocumentInstance>,
}

struct WidgetDocumentRecord {
    source_path: Option<PathBuf>,
    document: WidgetBlueprintDocument,
}

struct WidgetDocumentInstance {
    doc_id: WidgetDocId,
    parent: Entity,
    theme: Option<Theme>,
    roots: Vec<Entity>,
    runtime_map: BlueprintRuntimeMap,
}

impl WidgetDocStore {
    fn insert_document(
        &mut self,
        document: WidgetBlueprintDocument,
        source_path: Option<PathBuf>,
    ) -> WidgetDocId {
        self.next_doc_id = self.next_doc_id.saturating_add(1);
        let id = WidgetDocId(self.next_doc_id);
        self.documents.insert(
            id,
            WidgetDocumentRecord {
                source_path,
                document,
            },
        );
        id
    }

    fn alloc_instance_id(&mut self) -> WidgetDocInstanceId {
        self.next_instance_id = self.next_instance_id.saturating_add(1);
        WidgetDocInstanceId(self.next_instance_id)
    }
}

#[derive(SystemParam)]
pub struct WidgetDocUtility<'w, 's> {
    commands: Commands<'w, 's>,
    store: ResMut<'w, WidgetDocStore>,
    widget_registry: Res<'w, WidgetRegistry>,
    inspector_registry: Res<'w, InspectorEditorRegistry>,
    control_registry: Res<'w, InspectorControlRegistry>,
    theme: Option<Res<'w, Theme>>,
}

#[derive(SystemParam)]
pub struct WidgetDocLiveRef<'w, 's, T>
where
    T: Component + 'static,
{
    widgets: Query<'w, 's, &'static T>,
}

#[derive(SystemParam)]
pub struct WidgetDocLiveMut<'w, 's, T>
where
    T: Component<Mutability = Mutable> + 'static,
{
    widgets: Query<'w, 's, &'static mut T>,
}

impl<'w, 's> WidgetDocUtility<'w, 's> {
    pub fn load_path(&mut self, path: impl AsRef<Path>) -> Result<WidgetDocId, WidgetDocError> {
        let path = path.as_ref().to_path_buf();
        let input =
            fs::read_to_string(&path).map_err(|error| WidgetDocError::Io(error.to_string()))?;
        let asset = VistaUiAsset::from_ron_str(&input)?;
        let document = asset.to_blueprint_document()?;
        Ok(self.store.insert_document(document, Some(path)))
    }

    pub fn load_and_spawn_path(
        &mut self,
        path: impl AsRef<Path>,
        parent: Entity,
    ) -> Result<WidgetDocInstanceId, WidgetDocError> {
        let doc_id = self.load_path(path)?;
        self.spawn(doc_id, parent)
    }

    pub fn insert_document(&mut self, document: WidgetBlueprintDocument) -> WidgetDocId {
        self.store.insert_document(document, None)
    }

    pub fn source_path(&self, doc_id: WidgetDocId) -> Option<&Path> {
        self.store
            .documents
            .get(&doc_id)
            .and_then(|record| record.source_path.as_deref())
    }

    pub fn document(&self, doc_id: WidgetDocId) -> Option<&WidgetBlueprintDocument> {
        self.store
            .documents
            .get(&doc_id)
            .map(|record| &record.document)
    }

    pub fn document_mut(&mut self, doc_id: WidgetDocId) -> Option<&mut WidgetBlueprintDocument> {
        self.store
            .documents
            .get_mut(&doc_id)
            .map(|record| &mut record.document)
    }

    pub fn apply(
        &mut self,
        doc_id: WidgetDocId,
        command: BlueprintCommand,
    ) -> Result<BlueprintNodeId, WidgetDocError> {
        let widget_registry = &self.widget_registry;
        let Some(record) = self.store.documents.get_mut(&doc_id) else {
            return Err(WidgetDocError::DocumentNotFound(doc_id));
        };
        Ok(apply_blueprint_command(
            command,
            &mut record.document,
            widget_registry,
        )?)
    }

    pub fn query_first_by_name(&self, doc_id: WidgetDocId, name: &str) -> Option<BlueprintNodeId> {
        self.query_all_by_name(doc_id, name).into_iter().next()
    }

    pub fn query_all_by_name(&self, doc_id: WidgetDocId, name: &str) -> Vec<BlueprintNodeId> {
        self.query_nodes(doc_id, |node| node.name == name)
    }

    pub fn query_first_by_widget_path(
        &self,
        doc_id: WidgetDocId,
        widget_path: &str,
        name: Option<&str>,
    ) -> Option<BlueprintNodeId> {
        self.query_nodes(doc_id, |node| {
            node.widget_path == widget_path && name.is_none_or(|value| node.name == value)
        })
        .into_iter()
        .next()
    }

    pub fn query_first<T>(
        &self,
        doc_id: WidgetDocId,
        name: Option<&str>,
    ) -> Result<Option<BlueprintNodeId>, WidgetDocError>
    where
        T: Widget + 'static,
    {
        let widget_path =
            self.widget_registry
                .widget_path::<T>()
                .ok_or(WidgetDocError::WidgetNotRegistered(
                    std::any::type_name::<T>(),
                ))?;
        Ok(self.query_first_by_widget_path(doc_id, &widget_path, name))
    }

    pub fn style(
        &self,
        doc_id: WidgetDocId,
        node_id: BlueprintNodeId,
    ) -> Option<&crate::widget::WidgetStyle> {
        self.document(doc_id)?
            .nodes
            .get(&node_id)
            .map(|node| &node.style)
    }

    pub fn with_style_mut(
        &mut self,
        doc_id: WidgetDocId,
        node_id: BlueprintNodeId,
        update: impl FnOnce(&mut crate::widget::WidgetStyle),
    ) -> Result<(), WidgetDocError> {
        let Some(document) = self.document_mut(doc_id) else {
            return Err(WidgetDocError::DocumentNotFound(doc_id));
        };
        let Some(node) = document.nodes.get_mut(&node_id) else {
            return Err(WidgetDocError::NodeNotFound(node_id));
        };
        update(&mut node.style);
        document.dirty = true;
        Ok(())
    }

    pub fn read_widget<T>(
        &self,
        doc_id: WidgetDocId,
        node_id: BlueprintNodeId,
    ) -> Result<T, WidgetDocError>
    where
        T: Widget + Component + Reflect + Default + Clone + 'static,
    {
        let (widget_path, current, _, _) = self.materialize_widget_props(doc_id, node_id)?;
        current
            .try_downcast_ref::<T>()
            .cloned()
            .ok_or(WidgetDocError::WidgetPropTypeMismatch {
                widget_path,
                expected: std::any::type_name::<T>(),
            })
    }

    pub fn with_widget_mut<T>(
        &mut self,
        doc_id: WidgetDocId,
        node_id: BlueprintNodeId,
        update: impl FnOnce(&mut T),
    ) -> Result<(), WidgetDocError>
    where
        T: Widget + Component + Reflect + Default + Clone + 'static,
    {
        let (widget_path, mut current, default, entries) =
            self.materialize_widget_props(doc_id, node_id)?;
        let Some(value) = current.try_downcast_mut::<T>() else {
            return Err(WidgetDocError::WidgetPropTypeMismatch {
                widget_path,
                expected: std::any::type_name::<T>(),
            });
        };
        update(value);
        self.store_widget_props(doc_id, node_id, current, default, &entries)
    }

    pub fn with_named_widget_mut<T>(
        &mut self,
        doc_id: WidgetDocId,
        name: Option<&str>,
        update: impl FnOnce(&mut T),
    ) -> Result<BlueprintNodeId, WidgetDocError>
    where
        T: Widget + Component + Reflect + Default + Clone + 'static,
    {
        let widget_path =
            self.widget_registry
                .widget_path::<T>()
                .ok_or(WidgetDocError::WidgetNotRegistered(
                    std::any::type_name::<T>(),
                ))?;
        let Some(node_id) = self.query_first_by_widget_path(doc_id, &widget_path, name) else {
            return Err(WidgetDocError::WidgetQueryNotFound {
                widget_path,
                name: name.map(str::to_owned),
            });
        };
        self.with_widget_mut::<T>(doc_id, node_id, update)?;
        Ok(node_id)
    }

    pub fn spawn(
        &mut self,
        doc_id: WidgetDocId,
        parent: Entity,
    ) -> Result<WidgetDocInstanceId, WidgetDocError> {
        self.spawn_with_theme(doc_id, parent, self.theme.as_deref().cloned())
    }

    pub fn spawn_with_theme(
        &mut self,
        doc_id: WidgetDocId,
        parent: Entity,
        theme: Option<Theme>,
    ) -> Result<WidgetDocInstanceId, WidgetDocError> {
        let Some(record) = self.store.documents.get(&doc_id) else {
            return Err(WidgetDocError::DocumentNotFound(doc_id));
        };
        let spawn = spawn_document_instance(
            &mut self.commands,
            &record.document,
            parent,
            &self.widget_registry,
            &self.inspector_registry,
            &self.control_registry,
            theme.as_ref(),
        )?;
        let instance_id = self.store.alloc_instance_id();
        self.store.instances.insert(
            instance_id,
            WidgetDocumentInstance {
                doc_id,
                parent,
                theme,
                roots: spawn.roots,
                runtime_map: spawn.runtime_map,
            },
        );
        Ok(instance_id)
    }

    pub fn flush(&mut self, instance_id: WidgetDocInstanceId) -> Result<(), WidgetDocError> {
        let Some((doc_id, parent, theme, roots)) =
            self.store.instances.get_mut(&instance_id).map(|instance| {
                (
                    instance.doc_id,
                    instance.parent,
                    instance.theme.clone(),
                    std::mem::take(&mut instance.roots),
                )
            })
        else {
            return Err(WidgetDocError::InstanceNotFound(instance_id));
        };
        let Some(record) = self.store.documents.get(&doc_id) else {
            return Err(WidgetDocError::DocumentNotFound(doc_id));
        };

        for root in roots {
            self.commands.entity(root).despawn();
        }

        let spawn = spawn_document_instance(
            &mut self.commands,
            &record.document,
            parent,
            &self.widget_registry,
            &self.inspector_registry,
            &self.control_registry,
            theme.as_ref(),
        )?;
        if let Some(instance) = self.store.instances.get_mut(&instance_id) {
            instance.roots = spawn.roots;
            instance.runtime_map = spawn.runtime_map;
        }
        Ok(())
    }

    pub fn despawn(&mut self, instance_id: WidgetDocInstanceId) -> Result<(), WidgetDocError> {
        let Some(instance) = self.store.instances.remove(&instance_id) else {
            return Err(WidgetDocError::InstanceNotFound(instance_id));
        };
        for root in instance.roots {
            self.commands.entity(root).despawn();
        }
        Ok(())
    }

    pub fn doc_for_instance(&self, instance_id: WidgetDocInstanceId) -> Option<WidgetDocId> {
        self.store
            .instances
            .get(&instance_id)
            .map(|instance| instance.doc_id)
    }

    pub fn entity(
        &self,
        instance_id: WidgetDocInstanceId,
        node_id: BlueprintNodeId,
    ) -> Option<Entity> {
        self.store
            .instances
            .get(&instance_id)?
            .runtime_map
            .node_to_entity
            .get(&node_id)
            .copied()
    }

    pub fn entity_by_name(&self, instance_id: WidgetDocInstanceId, name: &str) -> Option<Entity> {
        let doc_id = self.doc_for_instance(instance_id)?;
        let node_id = self.query_first_by_name(doc_id, name)?;
        self.entity(instance_id, node_id)
    }

    pub fn entity_of<T>(
        &self,
        instance_id: WidgetDocInstanceId,
        name: Option<&str>,
    ) -> Result<Option<Entity>, WidgetDocError>
    where
        T: Widget + 'static,
    {
        let Some(doc_id) = self.doc_for_instance(instance_id) else {
            return Err(WidgetDocError::InstanceNotFound(instance_id));
        };
        let Some(node_id) = self.query_first::<T>(doc_id, name)? else {
            return Ok(None);
        };
        Ok(self.entity(instance_id, node_id))
    }

    pub fn read_live_widget<T>(
        &self,
        instance_id: WidgetDocInstanceId,
        node_id: BlueprintNodeId,
        widgets: &WidgetDocLiveRef<T>,
    ) -> Result<T, WidgetDocError>
    where
        T: Widget + Component + Clone + 'static,
    {
        let entity =
            self.entity(instance_id, node_id)
                .ok_or(WidgetDocError::LiveNodeEntityNotFound {
                    instance_id,
                    node_id,
                })?;
        widgets
            .widgets
            .get(entity)
            .cloned()
            .map_err(|_| WidgetDocError::LiveWidgetUnavailable {
                entity,
                expected: std::any::type_name::<T>(),
            })
    }

    pub fn read_named_live_widget<T>(
        &self,
        instance_id: WidgetDocInstanceId,
        name: Option<&str>,
        widgets: &WidgetDocLiveRef<T>,
    ) -> Result<T, WidgetDocError>
    where
        T: Widget + Component + Clone + 'static,
    {
        let widget_path = self.widget_path_for::<T>()?;
        let Some(node_id) = self
            .doc_for_instance(instance_id)
            .and_then(|doc_id| self.query_first_by_widget_path(doc_id, &widget_path, name))
        else {
            return Err(WidgetDocError::WidgetQueryNotFound {
                widget_path,
                name: name.map(str::to_owned),
            });
        };
        self.read_live_widget(instance_id, node_id, widgets)
    }

    pub fn with_live_widget_mut<T>(
        &self,
        instance_id: WidgetDocInstanceId,
        node_id: BlueprintNodeId,
        widgets: &mut WidgetDocLiveMut<T>,
        update: impl FnOnce(&mut T),
    ) -> Result<(), WidgetDocError>
    where
        T: Widget + Component<Mutability = Mutable> + 'static,
    {
        let entity =
            self.entity(instance_id, node_id)
                .ok_or(WidgetDocError::LiveNodeEntityNotFound {
                    instance_id,
                    node_id,
                })?;
        let mut widget =
            widgets
                .widgets
                .get_mut(entity)
                .map_err(|_| WidgetDocError::LiveWidgetUnavailable {
                    entity,
                    expected: std::any::type_name::<T>(),
                })?;
        update(&mut widget);
        Ok(())
    }

    pub fn with_named_live_widget_mut<T>(
        &self,
        instance_id: WidgetDocInstanceId,
        name: Option<&str>,
        widgets: &mut WidgetDocLiveMut<T>,
        update: impl FnOnce(&mut T),
    ) -> Result<BlueprintNodeId, WidgetDocError>
    where
        T: Widget + Component<Mutability = Mutable> + 'static,
    {
        let widget_path = self.widget_path_for::<T>()?;
        let Some(node_id) = self
            .doc_for_instance(instance_id)
            .and_then(|doc_id| self.query_first_by_widget_path(doc_id, &widget_path, name))
        else {
            return Err(WidgetDocError::WidgetQueryNotFound {
                widget_path,
                name: name.map(str::to_owned),
            });
        };
        self.with_live_widget_mut(instance_id, node_id, widgets, update)?;
        Ok(node_id)
    }

    fn query_nodes(
        &self,
        doc_id: WidgetDocId,
        predicate: impl Fn(&crate::inspector::WidgetBlueprintNode) -> bool,
    ) -> Vec<BlueprintNodeId> {
        let Some(document) = self.document(doc_id) else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for root in &document.roots {
            collect_matching_nodes(document, *root, &predicate, &mut result);
        }
        result
    }

    fn widget_path_for<T>(&self) -> Result<String, WidgetDocError>
    where
        T: Widget + 'static,
    {
        self.widget_registry
            .widget_path::<T>()
            .ok_or(WidgetDocError::WidgetNotRegistered(
                std::any::type_name::<T>(),
            ))
    }

    fn materialize_widget_props(
        &self,
        doc_id: WidgetDocId,
        node_id: BlueprintNodeId,
    ) -> Result<
        (
            String,
            Box<dyn bevy::reflect::PartialReflect>,
            Box<dyn bevy::reflect::PartialReflect>,
            Vec<InspectorEntryDescriptor>,
        ),
        WidgetDocError,
    > {
        let Some(document) = self.document(doc_id) else {
            return Err(WidgetDocError::DocumentNotFound(doc_id));
        };
        let Some(node) = document.nodes.get(&node_id) else {
            return Err(WidgetDocError::NodeNotFound(node_id));
        };
        let widget_path = node.widget_path.clone();
        let Some(registration) = self.widget_registry.get_widget_by_path(&widget_path) else {
            return Err(WidgetDocError::WidgetPathNotFound(widget_path));
        };
        let Some(mut current) = registration.default_inspector_value() else {
            return Err(WidgetDocError::WidgetPropsUnavailable(
                registration.full_path(),
            ));
        };
        let Some(default) = registration.default_inspector_value() else {
            return Err(WidgetDocError::WidgetPropsUnavailable(
                registration.full_path(),
            ));
        };
        let entries = registration.inspector_entries(&self.inspector_registry);
        for entry in &entries {
            let InspectorEntryDescriptor::Field(field) = entry else {
                continue;
            };
            let Some(raw) = node.props.get(&field.field_path) else {
                continue;
            };
            let Some(target) =
                crate::inspector::read_reflect_path_mut(current.as_mut(), &field.field_path)
            else {
                continue;
            };
            let _ = self
                .control_registry
                .apply_serialized_value(field.editor, target, raw)
                || crate::inspector::apply_serialized_editor_value(
                    field.editor,
                    target,
                    raw,
                    self.theme.as_deref(),
                );
        }
        Ok((registration.full_path(), current, default, entries))
    }

    fn store_widget_props(
        &mut self,
        doc_id: WidgetDocId,
        node_id: BlueprintNodeId,
        current: Box<dyn bevy::reflect::PartialReflect>,
        default: Box<dyn bevy::reflect::PartialReflect>,
        entries: &[InspectorEntryDescriptor],
    ) -> Result<(), WidgetDocError> {
        let serialized = crate::inspector::collect_non_default_serialized_fields(
            current.as_ref(),
            default.as_ref(),
            entries,
            self.theme.as_deref(),
        );
        let Some(document) = self.document_mut(doc_id) else {
            return Err(WidgetDocError::DocumentNotFound(doc_id));
        };
        let Some(node) = document.nodes.get_mut(&node_id) else {
            return Err(WidgetDocError::NodeNotFound(node_id));
        };
        for entry in entries {
            let InspectorEntryDescriptor::Field(field) = entry else {
                continue;
            };
            node.props.remove(&field.field_path);
        }
        node.props.extend(serialized);
        document.dirty = true;
        Ok(())
    }
}

struct WidgetDocSpawnResult {
    roots: Vec<Entity>,
    runtime_map: BlueprintRuntimeMap,
}

fn spawn_document_instance(
    commands: &mut Commands,
    document: &WidgetBlueprintDocument,
    parent: Entity,
    widget_registry: &WidgetRegistry,
    inspector_registry: &InspectorEditorRegistry,
    control_registry: &InspectorControlRegistry,
    theme: Option<&Theme>,
) -> Result<WidgetDocSpawnResult, WidgetDocError> {
    let mut runtime_map = BlueprintRuntimeMap::default();
    let mut roots = Vec::new();

    for root_id in document.roots.iter().copied() {
        let root = spawn_document_node_recursive(
            commands,
            document,
            &mut runtime_map,
            widget_registry,
            inspector_registry,
            control_registry,
            theme,
            parent,
            root_id,
        )?;
        roots.push(root);
    }

    Ok(WidgetDocSpawnResult { roots, runtime_map })
}

fn spawn_document_node_recursive(
    commands: &mut Commands,
    document: &WidgetBlueprintDocument,
    runtime_map: &mut BlueprintRuntimeMap,
    widget_registry: &WidgetRegistry,
    inspector_registry: &InspectorEditorRegistry,
    control_registry: &InspectorControlRegistry,
    theme: Option<&Theme>,
    parent: Entity,
    node_id: BlueprintNodeId,
) -> Result<Entity, WidgetDocError> {
    let Some(node) = document.nodes.get(&node_id) else {
        return Err(WidgetDocError::NodeNotFound(node_id));
    };
    let Some(spawn) = spawn_blueprint_widget_content(
        widget_registry,
        inspector_registry,
        Some(control_registry),
        commands,
        &node.widget_path,
        &node.style,
        &node.props,
        theme,
    ) else {
        return Err(WidgetDocError::WidgetPathNotFound(node.widget_path.clone()));
    };

    commands.entity(parent).add_child(spawn.root);
    commands.entity(spawn.root).insert(BlueprintNodeRef);
    runtime_map.node_to_entity.insert(node_id, spawn.root);
    runtime_map.entity_to_node.insert(spawn.root, node_id);

    for (index, child) in node.children.iter().copied().enumerate() {
        let child_parent = resolve_child_parent_entity(document, &spawn, child, index);
        let _ = spawn_document_node_recursive(
            commands,
            document,
            runtime_map,
            widget_registry,
            inspector_registry,
            control_registry,
            theme,
            child_parent,
            child,
        )?;
    }

    Ok(spawn.root)
}

fn resolve_child_parent_entity(
    document: &WidgetBlueprintDocument,
    parent_spawn: &WidgetSpawnResult,
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

fn collect_matching_nodes(
    document: &WidgetBlueprintDocument,
    node_id: BlueprintNodeId,
    predicate: &impl Fn(&crate::inspector::WidgetBlueprintNode) -> bool,
    output: &mut Vec<BlueprintNodeId>,
) {
    let Some(node) = document.nodes.get(&node_id) else {
        return;
    };
    if predicate(node) {
        output.push(node_id);
    }
    for child in &node.children {
        collect_matching_nodes(document, *child, predicate, output);
    }
}
