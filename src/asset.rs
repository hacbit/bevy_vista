use bevy::asset::Asset;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::reflect::{
    FromReflect, GetTypeRegistration, TypePath, TypeRegistry,
    serde::{TypedReflectDeserializer, TypedReflectSerializer},
};
use ron::ser::PrettyConfig;
use serde::de::DeserializeSeed;
use serde::{Deserialize, Serialize};

use crate::inspector::{BlueprintNodeId, WidgetBlueprintDocument, WidgetBlueprintNode};
use crate::inspector::InspectorEditorRegistry;
use crate::theme::Theme;
use crate::widget::{WidgetRegistry, WidgetStyle, spawn_blueprint_widget_content};

pub const VISTA_UI_ASSET_VERSION: u32 = 1;
pub const VISTA_UI_ASSET_EXTENSION: &str = "vista.ron";

pub struct VistaAssetPlugin;

impl Plugin for VistaAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<VistaUiAsset>();
    }
}

pub type VistaNodeId = BlueprintNodeId;

#[derive(Asset, TypePath, Clone, Debug)]
pub struct VistaUiAsset {
    pub version: u32,
    pub roots: Vec<VistaNodeId>,
    pub nodes: Vec<VistaUiNodeAsset>,
}

impl Default for VistaUiAsset {
    fn default() -> Self {
        Self {
            version: VISTA_UI_ASSET_VERSION,
            roots: Vec::new(),
            nodes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct VistaUiNodeAsset {
    pub id: VistaNodeId,
    pub name: String,
    pub widget_path: String,
    pub style: WidgetStyle,
    pub props: HashMap<String, String>,
    pub slot: Option<String>,
    pub children: Vec<VistaNodeId>,
}

#[derive(Serialize, Deserialize)]
struct SerializableVistaUiAsset {
    version: u32,
    roots: Vec<VistaNodeId>,
    nodes: Vec<SerializableVistaUiNodeAsset>,
}

#[derive(Serialize, Deserialize)]
struct SerializableVistaUiNodeAsset {
    id: VistaNodeId,
    name: String,
    widget_path: String,
    #[serde(default, skip_serializing_if = "serializable_style_is_empty")]
    style: SerializableVistaUiStyle,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    props: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    slot: Option<String>,
    children: Vec<VistaNodeId>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum SerializableVistaUiStyle {
    Full(String),
    Overrides(HashMap<String, String>),
}

impl Default for SerializableVistaUiStyle {
    fn default() -> Self {
        Self::Overrides(HashMap::default())
    }
}

#[derive(Clone, Debug)]
pub enum VistaUiAssetError {
    UnsupportedVersion(u32),
    DuplicateNodeId(VistaNodeId),
    MissingNode(VistaNodeId),
    MissingChild {
        parent: VistaNodeId,
        child: VistaNodeId,
    },
    InvalidParentLink {
        child: VistaNodeId,
        expected_parent: Option<VistaNodeId>,
        actual_parent: Option<VistaNodeId>,
    },
    CycleDetected(VistaNodeId),
    RonDecode(String),
    RonEncode(String),
}

#[derive(Default)]
pub struct VistaUiSpawnResult {
    pub roots: Vec<Entity>,
    pub node_to_entity: HashMap<VistaNodeId, Entity>,
}

impl VistaUiAsset {
    pub fn to_ron_string_pretty(&self) -> Result<String, VistaUiAssetError> {
        let serializable = SerializableVistaUiAsset::try_from(self)?;
        ron::ser::to_string_pretty(&serializable, PrettyConfig::default())
            .map_err(|error| VistaUiAssetError::RonEncode(error.to_string()))
    }

    pub fn to_ron_string(&self) -> Result<String, VistaUiAssetError> {
        let serializable = SerializableVistaUiAsset::try_from(self)?;
        ron::ser::to_string(&serializable)
            .map_err(|error| VistaUiAssetError::RonEncode(error.to_string()))
    }

    pub fn to_ron_string_pretty_compact(
        &self,
        widget_registry: &WidgetRegistry,
        inspector_registry: &InspectorEditorRegistry,
    ) -> Result<String, VistaUiAssetError> {
        let serializable =
            SerializableVistaUiAsset::try_from_compact(self, widget_registry, inspector_registry)?;
        ron::ser::to_string_pretty(&serializable, PrettyConfig::default())
            .map_err(|error| VistaUiAssetError::RonEncode(error.to_string()))
    }

    pub fn from_ron_str(input: &str) -> Result<Self, VistaUiAssetError> {
        let serializable: SerializableVistaUiAsset = ron::from_str(input)
            .map_err(|error| VistaUiAssetError::RonDecode(error.to_string()))?;
        serializable.try_into()
    }

    pub fn to_blueprint_document(&self) -> Result<WidgetBlueprintDocument, VistaUiAssetError> {
        self.try_into()
    }

    pub fn spawn_into(
        &self,
        commands: &mut Commands,
        parent: Entity,
        widget_registry: &WidgetRegistry,
        inspector_registry: &InspectorEditorRegistry,
        theme: Option<&Theme>,
    ) -> Result<VistaUiSpawnResult, VistaUiAssetError> {
        let document = self.to_blueprint_document()?;
        let mut result = VistaUiSpawnResult::default();
        for root_id in document.roots.iter().copied() {
            let root_entity = spawn_asset_node_recursive(
                commands,
                &document,
                root_id,
                parent,
                widget_registry,
                inspector_registry,
                theme,
                &mut result.node_to_entity,
            )?;
            result.roots.push(root_entity);
        }
        Ok(result)
    }
}

impl TryFrom<&VistaUiAsset> for SerializableVistaUiAsset {
    type Error = VistaUiAssetError;

    fn try_from(asset: &VistaUiAsset) -> Result<Self, Self::Error> {
        Ok(Self {
            version: asset.version,
            roots: asset.roots.clone(),
            nodes: asset
                .nodes
                .iter()
                .map(SerializableVistaUiNodeAsset::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl TryFrom<SerializableVistaUiAsset> for VistaUiAsset {
    type Error = VistaUiAssetError;

    fn try_from(asset: SerializableVistaUiAsset) -> Result<Self, Self::Error> {
        Ok(Self {
            version: asset.version,
            roots: asset.roots,
            nodes: asset
                .nodes
                .into_iter()
                .map(VistaUiNodeAsset::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl TryFrom<&VistaUiNodeAsset> for SerializableVistaUiNodeAsset {
    type Error = VistaUiAssetError;

    fn try_from(node: &VistaUiNodeAsset) -> Result<Self, Self::Error> {
        Ok(Self {
            id: node.id,
            name: node.name.clone(),
            widget_path: node.widget_path.clone(),
            style: SerializableVistaUiStyle::Full(serialize_widget_style(&node.style)?),
            props: node.props.clone(),
            slot: node.slot.clone(),
            children: node.children.clone(),
        })
    }
}

impl TryFrom<SerializableVistaUiNodeAsset> for VistaUiNodeAsset {
    type Error = VistaUiAssetError;

    fn try_from(node: SerializableVistaUiNodeAsset) -> Result<Self, Self::Error> {
        Ok(Self {
            id: node.id,
            name: node.name,
            widget_path: node.widget_path,
            style: deserialize_widget_style(node.style)?,
            props: node.props,
            slot: node.slot,
            children: node.children,
        })
    }
}

impl SerializableVistaUiAsset {
    fn try_from_compact(
        asset: &VistaUiAsset,
        widget_registry: &WidgetRegistry,
        inspector_registry: &InspectorEditorRegistry,
    ) -> Result<Self, VistaUiAssetError> {
        Ok(Self {
            version: asset.version,
            roots: asset.roots.clone(),
            nodes: asset
                .nodes
                .iter()
                .map(|node| {
                    SerializableVistaUiNodeAsset::try_from_compact(
                        node,
                        widget_registry,
                        inspector_registry,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl SerializableVistaUiNodeAsset {
    fn try_from_compact(
        node: &VistaUiNodeAsset,
        widget_registry: &WidgetRegistry,
        inspector_registry: &InspectorEditorRegistry,
    ) -> Result<Self, VistaUiAssetError> {
        Ok(Self {
            id: node.id,
            name: node.name.clone(),
            widget_path: node.widget_path.clone(),
            style: SerializableVistaUiStyle::Overrides(serialize_widget_style_overrides(
                &node.style,
                inspector_registry,
            )?),
            props: serialize_widget_prop_overrides(node, widget_registry, inspector_registry)?,
            slot: node.slot.clone(),
            children: node.children.clone(),
        })
    }
}

impl From<&WidgetBlueprintDocument> for VistaUiAsset {
    fn from(document: &WidgetBlueprintDocument) -> Self {
        let mut nodes = document
            .nodes
            .values()
            .map(|node| VistaUiNodeAsset {
                id: node.id,
                name: node.name.clone(),
                widget_path: node.widget_path.clone(),
                style: node.style.clone(),
                props: node.props.clone(),
                slot: node.slot.clone(),
                children: node.children.clone(),
            })
            .collect::<Vec<_>>();
        nodes.sort_by_key(|node| node.id);
        Self {
            version: VISTA_UI_ASSET_VERSION,
            roots: document.roots.clone(),
            nodes,
        }
    }
}

impl TryFrom<&VistaUiAsset> for WidgetBlueprintDocument {
    type Error = VistaUiAssetError;

    fn try_from(asset: &VistaUiAsset) -> Result<Self, Self::Error> {
        if asset.version != VISTA_UI_ASSET_VERSION {
            return Err(VistaUiAssetError::UnsupportedVersion(asset.version));
        }

        let mut nodes = HashMap::default();
        let mut parent_links = HashMap::<VistaNodeId, Option<VistaNodeId>>::default();

        for node in &asset.nodes {
            if nodes.contains_key(&node.id) {
                return Err(VistaUiAssetError::DuplicateNodeId(node.id));
            }
            nodes.insert(
                node.id,
                WidgetBlueprintNode {
                    id: node.id,
                    name: node.name.clone(),
                    widget_path: node.widget_path.clone(),
                    style: node.style.clone(),
                    props: node.props.clone(),
                    parent: None,
                    slot: node.slot.clone(),
                    children: node.children.clone(),
                },
            );
            parent_links.insert(node.id, None);
        }

        for root in &asset.roots {
            if !nodes.contains_key(root) {
                return Err(VistaUiAssetError::MissingNode(*root));
            }
        }

        for node in &asset.nodes {
            for child in &node.children {
                if !nodes.contains_key(child) {
                    return Err(VistaUiAssetError::MissingChild {
                        parent: node.id,
                        child: *child,
                    });
                }
                let entry = parent_links.entry(*child).or_insert(None);
                if let Some(actual_parent) = *entry {
                    return Err(VistaUiAssetError::InvalidParentLink {
                        child: *child,
                        expected_parent: Some(node.id),
                        actual_parent: Some(actual_parent),
                    });
                }
                *entry = Some(node.id);
            }
        }

        for root in &asset.roots {
            if parent_links.get(root).copied().flatten().is_some() {
                return Err(VistaUiAssetError::InvalidParentLink {
                    child: *root,
                    expected_parent: None,
                    actual_parent: parent_links.get(root).copied().flatten(),
                });
            }
        }

        for (node_id, parent) in parent_links {
            let Some(node) = nodes.get_mut(&node_id) else {
                return Err(VistaUiAssetError::MissingNode(node_id));
            };
            node.parent = parent;
        }

        validate_asset_acyclic(asset, &nodes)?;

        Ok(WidgetBlueprintDocument::from_parts(
            asset.roots.clone(),
            nodes,
        ))
    }
}

fn validate_asset_acyclic(
    asset: &VistaUiAsset,
    nodes: &HashMap<VistaNodeId, WidgetBlueprintNode>,
) -> Result<(), VistaUiAssetError> {
    fn visit(
        node_id: VistaNodeId,
        nodes: &HashMap<VistaNodeId, WidgetBlueprintNode>,
        visiting: &mut HashSet<VistaNodeId>,
        visited: &mut HashSet<VistaNodeId>,
    ) -> Result<(), VistaUiAssetError> {
        if visited.contains(&node_id) {
            return Ok(());
        }
        if !visiting.insert(node_id) {
            return Err(VistaUiAssetError::CycleDetected(node_id));
        }
        let Some(node) = nodes.get(&node_id) else {
            return Err(VistaUiAssetError::MissingNode(node_id));
        };
        for child in &node.children {
            visit(*child, nodes, visiting, visited)?;
        }
        visiting.remove(&node_id);
        visited.insert(node_id);
        Ok(())
    }

    let mut visiting = HashSet::default();
    let mut visited = HashSet::default();
    for root in &asset.roots {
        visit(*root, nodes, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn spawn_asset_node_recursive(
    commands: &mut Commands,
    document: &WidgetBlueprintDocument,
    node_id: VistaNodeId,
    parent: Entity,
    widget_registry: &WidgetRegistry,
    inspector_registry: &InspectorEditorRegistry,
    theme: Option<&Theme>,
    node_to_entity: &mut HashMap<VistaNodeId, Entity>,
) -> Result<Entity, VistaUiAssetError> {
    let Some(node) = document.nodes.get(&node_id) else {
        return Err(VistaUiAssetError::MissingNode(node_id));
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
        return Err(VistaUiAssetError::MissingNode(node_id));
    };
    commands.entity(parent).add_child(spawn.root);
    node_to_entity.insert(node_id, spawn.root);
    for (index, child) in node.children.iter().copied().enumerate() {
        let child_parent = resolve_asset_child_parent_entity(document, &spawn, child, index);
        let _ = spawn_asset_node_recursive(
            commands,
            document,
            child,
            child_parent,
            widget_registry,
            inspector_registry,
            theme,
            node_to_entity,
        )?;
    }
    Ok(spawn.root)
}

fn resolve_asset_child_parent_entity(
    document: &WidgetBlueprintDocument,
    parent_spawn: &crate::widget::WidgetSpawnResult,
    child_node_id: VistaNodeId,
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

fn widget_style_registry() -> TypeRegistry {
    let mut registry = TypeRegistry::default();
    register_reflect_type::<WidgetStyle>(&mut registry);
    register_reflect_type::<Display>(&mut registry);
    register_reflect_type::<Visibility>(&mut registry);
    register_reflect_type::<Overflow>(&mut registry);
    register_reflect_type::<OverflowClipMargin>(&mut registry);
    register_reflect_type::<OverflowClipBox>(&mut registry);
    register_reflect_type::<PositionType>(&mut registry);
    register_reflect_type::<Val>(&mut registry);
    register_reflect_type::<FlexDirection>(&mut registry);
    register_reflect_type::<FlexWrap>(&mut registry);
    register_reflect_type::<AlignItems>(&mut registry);
    register_reflect_type::<JustifyItems>(&mut registry);
    register_reflect_type::<AlignSelf>(&mut registry);
    register_reflect_type::<JustifySelf>(&mut registry);
    register_reflect_type::<AlignContent>(&mut registry);
    register_reflect_type::<JustifyContent>(&mut registry);
    register_reflect_type::<BoxSizing>(&mut registry);
    register_reflect_type::<UiRect>(&mut registry);
    register_reflect_type::<Color>(&mut registry);
    register_reflect_type::<BorderRadius>(&mut registry);
    register_reflect_type::<BorderColor>(&mut registry);
    register_reflect_type::<UiTransform>(&mut registry);
    register_reflect_type::<Vec2>(&mut registry);
    register_reflect_type::<Rot2>(&mut registry);
    registry
}

fn register_reflect_type<T: GetTypeRegistration + TypePath>(registry: &mut TypeRegistry) {
    registry.register::<T>();
}

fn serialize_widget_style(style: &WidgetStyle) -> Result<String, VistaUiAssetError> {
    let registry = widget_style_registry();
    ron::ser::to_string(&TypedReflectSerializer::new(style, &registry))
        .map_err(|error| VistaUiAssetError::RonEncode(error.to_string()))
}

fn deserialize_widget_style(
    input: SerializableVistaUiStyle,
) -> Result<WidgetStyle, VistaUiAssetError> {
    match input {
        SerializableVistaUiStyle::Full(input) => deserialize_widget_style_full(&input),
        SerializableVistaUiStyle::Overrides(overrides) => {
            deserialize_widget_style_overrides(&overrides)
        }
    }
}

fn deserialize_widget_style_full(input: &str) -> Result<WidgetStyle, VistaUiAssetError> {
    let registry = widget_style_registry();
    let Some(registration) = registry.get(std::any::TypeId::of::<WidgetStyle>()) else {
        return Err(VistaUiAssetError::RonDecode(
            "missing WidgetStyle type registration".to_owned(),
        ));
    };
    let deserializer = TypedReflectDeserializer::new(registration, &registry);
    let mut ron_deserializer = ron::de::Deserializer::from_str(input)
        .map_err(|error| VistaUiAssetError::RonDecode(error.to_string()))?;
    let reflect_value = deserializer
        .deserialize(&mut ron_deserializer)
        .map_err(|error| VistaUiAssetError::RonDecode(error.to_string()))?;
    WidgetStyle::from_reflect(reflect_value.as_partial_reflect())
        .ok_or_else(|| VistaUiAssetError::RonDecode("failed to reconstruct WidgetStyle".to_owned()))
}

fn deserialize_widget_style_overrides(
    overrides: &HashMap<String, String>,
) -> Result<WidgetStyle, VistaUiAssetError> {
    let entries = InspectorEditorRegistry::default().entries_for::<WidgetStyle>();
    let mut style = WidgetStyle::default();
    let reflect: &mut dyn bevy::reflect::PartialReflect = &mut style;
    apply_serialized_field_overrides(reflect, &entries, overrides, None);
    Ok(style)
}

fn serialize_widget_style_overrides(
    style: &WidgetStyle,
    inspector_registry: &InspectorEditorRegistry,
) -> Result<HashMap<String, String>, VistaUiAssetError> {
    let entries = inspector_registry.entries_for::<WidgetStyle>();
    Ok(crate::inspector::collect_non_default_serialized_fields(
        style,
        &WidgetStyle::default(),
        &entries,
        None,
    ))
}

fn serialize_widget_prop_overrides(
    node: &VistaUiNodeAsset,
    widget_registry: &WidgetRegistry,
    inspector_registry: &InspectorEditorRegistry,
) -> Result<HashMap<String, String>, VistaUiAssetError> {
    let Some(registration) = widget_registry.get_widget_by_path(&node.widget_path) else {
        return Ok(node.props.clone());
    };
    let Some(default_value) = registration.default_inspector_value() else {
        return Ok(node.props.clone());
    };
    let Some(mut current_value) = registration.default_inspector_value() else {
        return Ok(node.props.clone());
    };
    let entries = registration.inspector_entries(inspector_registry);
    apply_serialized_field_overrides(current_value.as_mut(), &entries, &node.props, None);
    Ok(crate::inspector::collect_non_default_serialized_fields(
        current_value.as_ref(),
        default_value.as_ref(),
        &entries,
        None,
    ))
}

fn apply_serialized_field_overrides(
    reflect: &mut dyn bevy::reflect::PartialReflect,
    entries: &[crate::inspector::InspectorEntryDescriptor],
    values: &HashMap<String, String>,
    theme: Option<&Theme>,
) {
    for entry in entries {
        let crate::inspector::InspectorEntryDescriptor::Field(field) = entry else {
            continue;
        };
        let Some(raw) = values.get(&field.field_path) else {
            continue;
        };
        let Some(target) = crate::inspector::read_reflect_path_mut(reflect, &field.field_path)
        else {
            continue;
        };
        let _ = crate::inspector::apply_serialized_editor_value(
            field.editor,
            target,
            raw,
            field.numeric_min,
            theme,
        );
    }
}

fn serializable_style_is_empty(style: &SerializableVistaUiStyle) -> bool {
    matches!(style, SerializableVistaUiStyle::Overrides(overrides) if overrides.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::blueprint::{BlueprintCommand, apply_blueprint_command};

    #[test]
    fn vista_ui_asset_round_trips_blueprint_document() {
        let widget_registry = WidgetRegistry::new();
        let inspector_registry = InspectorEditorRegistry::default();
        let mut document = WidgetBlueprintDocument::default();

        let root = match apply_blueprint_command(
            BlueprintCommand::AddRoot {
                widget_path: "common/button".to_owned(),
            },
            &mut document,
            &widget_registry,
        ) {
            Ok(id) => id,
            Err(_) => panic!("root should be added"),
        };
        let child = match apply_blueprint_command(
            BlueprintCommand::AddChild {
                parent: root,
                widget_path: "common/label".to_owned(),
            },
            &mut document,
            &widget_registry,
        ) {
            Ok(id) => id,
            Err(_) => panic!("child should be added"),
        };

        document.nodes.get_mut(&root).unwrap().name = "Root".to_owned();
        document.nodes.get_mut(&child).unwrap().name = "Child".to_owned();
        document.nodes.get_mut(&root).unwrap().style.width = Val::Px(240.0);
        document
            .nodes
            .get_mut(&child)
            .unwrap()
            .props
            .insert("text".to_owned(), "Child".to_owned());

        let asset = VistaUiAsset::from(&document);
        let ron = asset
            .to_ron_string_pretty_compact(&widget_registry, &inspector_registry)
            .expect("asset should encode into ron");
        let restored = VistaUiAsset::from_ron_str(&ron)
            .expect("asset should decode from ron")
            .to_blueprint_document()
            .expect("asset should restore into blueprint");

        assert_eq!(restored.roots, document.roots);
        assert_eq!(restored.nodes.len(), document.nodes.len());
        assert_eq!(
            restored.nodes.get(&root).map(|node| node.name.as_str()),
            Some("Root")
        );
        assert_eq!(
            restored.nodes.get(&child).and_then(|node| node.parent),
            Some(root)
        );
        assert_eq!(
            restored.nodes.get(&root).map(|node| node.style.width),
            Some(Val::Px(240.0))
        );
        assert_eq!(
            restored
                .nodes
                .get(&child)
                .and_then(|node| node.props.get("text"))
                .map(String::as_str),
            Some("Child")
        );
    }

    #[test]
    fn compact_asset_serialization_omits_default_values() {
        let widget_registry = WidgetRegistry::new();
        let inspector_registry = InspectorEditorRegistry::default();
        let mut document = WidgetBlueprintDocument::default();

        let root = match apply_blueprint_command(
            BlueprintCommand::AddRoot {
                widget_path: "common/label".to_owned(),
            },
            &mut document,
            &widget_registry,
        ) {
            Ok(id) => id,
            Err(_) => panic!("root should be added"),
        };

        document
            .nodes
            .get_mut(&root)
            .unwrap()
            .props
            .insert("text".to_owned(), "Label".to_owned());

        let asset = VistaUiAsset::from(&document);
        let ron = asset
            .to_ron_string_pretty_compact(&widget_registry, &inspector_registry)
            .expect("asset should encode into ron");

        assert!(
            !ron.contains("style:"),
            "default style should not be serialized: {ron}"
        );
        assert!(
            !ron.contains("props:"),
            "default props should not be serialized: {ron}"
        );
    }
}
