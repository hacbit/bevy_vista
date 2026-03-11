use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use crate::core::inspector::{
    BlueprintCommand, BlueprintNodeId, InspectorEditorRegistry, InspectorEntryDescriptor,
    InspectorFieldEditor, WidgetBlueprintDocument, apply_blueprint_command, read_reflect_path,
    read_reflect_path_mut,
};
use crate::core::widget::{WidgetRegistry, WidgetStyle};

use super::{InspectorBindingTarget, InspectorControlRegistry, InspectorPanelState};

pub(crate) fn selected_node_style<'a>(
    panel_state: &InspectorPanelState,
    document: &'a WidgetBlueprintDocument,
) -> Option<&'a WidgetStyle> {
    let node_id = panel_state.selected_node?;
    let node = document.nodes.get(&node_id)?;
    if !panel_state.visible {
        return None;
    }
    Some(&node.style)
}

pub(crate) fn selected_node_widget_reflect(
    panel_state: &InspectorPanelState,
    document: &WidgetBlueprintDocument,
    widget_registry: &WidgetRegistry,
    inspector_registry: &InspectorEditorRegistry,
    control_registry: &InspectorControlRegistry,
) -> Option<Box<dyn PartialReflect>> {
    let node_id = panel_state.selected_node?;
    let node = document.nodes.get(&node_id)?;
    if !panel_state.visible {
        return None;
    }
    let registration = widget_registry.get_widget_by_path(&node.widget_path)?;
    let mut value = registration.default_inspector_value()?;
    let reflect = value.as_mut();
    for entry in registration.inspector_entries(inspector_registry) {
        let InspectorEntryDescriptor::Field(field) = entry else {
            continue;
        };
        let Some(raw) = node.props.get(&field.field_path) else {
            continue;
        };
        let Some(target) = read_reflect_path_mut(reflect, &field.field_path) else {
            continue;
        };
        let _ = control_registry.apply_serialized_value(field.editor, target, raw);
    }
    Some(value)
}

pub(crate) fn selected_node_widget_default_reflect(
    panel_state: &InspectorPanelState,
    document: &WidgetBlueprintDocument,
    widget_registry: &WidgetRegistry,
) -> Option<Box<dyn PartialReflect>> {
    let node_id = panel_state.selected_node?;
    let node = document.nodes.get(&node_id)?;
    if !panel_state.visible {
        return None;
    }
    let registration = widget_registry.get_widget_by_path(&node.widget_path)?;
    registration.default_inspector_value()
}

fn apply_widget_prop_change(
    node_id: BlueprintNodeId,
    field_path: &str,
    serialized: String,
    document: &mut WidgetBlueprintDocument,
    widget_registry: &WidgetRegistry,
) {
    let _ = apply_blueprint_command(
        BlueprintCommand::SetNodeProp {
            node: node_id,
            key: field_path.to_owned(),
            value: serialized,
        },
        document,
        widget_registry,
    );
}

pub(crate) fn clear_widget_prop_change(
    node_id: BlueprintNodeId,
    field_path: &str,
    document: &mut WidgetBlueprintDocument,
    widget_registry: &WidgetRegistry,
) {
    let _ = apply_blueprint_command(
        BlueprintCommand::RemoveNodeProp {
            node: node_id,
            key: field_path.to_owned(),
        },
        document,
        widget_registry,
    );
}

pub(crate) fn store_widget_prop_change(
    node_id: BlueprintNodeId,
    field_path: &str,
    editor: InspectorFieldEditor,
    field: &dyn PartialReflect,
    document: &mut WidgetBlueprintDocument,
    widget_registry: &WidgetRegistry,
    control_registry: &InspectorControlRegistry,
) {
    let Some(node) = document.nodes.get(&node_id) else {
        return;
    };
    let Some(serialized) = control_registry.serialize_value(editor, field) else {
        return;
    };
    let Some(registration) = widget_registry.get_widget_by_path(&node.widget_path) else {
        apply_widget_prop_change(node_id, field_path, serialized, document, widget_registry);
        return;
    };
    let Some(default_value) = registration.default_inspector_value() else {
        apply_widget_prop_change(node_id, field_path, serialized, document, widget_registry);
        return;
    };
    let Some(default_field) = read_reflect_path(default_value.as_ref(), field_path) else {
        apply_widget_prop_change(node_id, field_path, serialized, document, widget_registry);
        return;
    };

    if field.reflect_partial_eq(default_field).unwrap_or(false) {
        clear_widget_prop_change(node_id, field_path, document, widget_registry);
        return;
    }

    apply_widget_prop_change(node_id, field_path, serialized, document, widget_registry);
}

pub(crate) fn find_ancestor_with<F>(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    predicate: F,
) -> Option<Entity>
where
    F: Fn(Entity) -> bool,
{
    loop {
        if predicate(entity) {
            return Some(entity);
        }
        let Ok(parent) = parents.get(entity) else {
            return None;
        };
        entity = parent.parent();
    }
}

pub(crate) fn apply_style_change(
    node_id: BlueprintNodeId,
    style: WidgetStyle,
    document: &mut WidgetBlueprintDocument,
    widget_registry: &WidgetRegistry,
) {
    let _ = apply_blueprint_command(
        BlueprintCommand::SetNodeStyle {
            node: node_id,
            style,
        },
        document,
        widget_registry,
    );
}

pub(crate) fn selected_binding_source<'a>(
    style: &'a WidgetStyle,
    widget_reflect: Option<&'a dyn PartialReflect>,
    target: &InspectorBindingTarget,
    field_path: &str,
) -> Option<&'a dyn PartialReflect> {
    match target {
        InspectorBindingTarget::Style => {
            read_reflect_path(style as &dyn PartialReflect, field_path)
        }
        InspectorBindingTarget::WidgetProp => {
            widget_reflect.and_then(|value| read_reflect_path(value, field_path))
        }
    }
}

pub(crate) fn apply_selected_field_change<F>(
    panel_state: &InspectorPanelState,
    document: &mut WidgetBlueprintDocument,
    widget_registry: &WidgetRegistry,
    inspector_registry: &InspectorEditorRegistry,
    control_registry: &InspectorControlRegistry,
    target: &InspectorBindingTarget,
    field_path: &str,
    editor: InspectorFieldEditor,
    apply: F,
) -> bool
where
    F: FnOnce(&mut dyn PartialReflect) -> bool,
{
    let Some(node_id) = panel_state.selected_node else {
        return false;
    };

    match target {
        InspectorBindingTarget::WidgetProp => {
            let Some(mut value) = selected_node_widget_reflect(
                panel_state,
                document,
                widget_registry,
                inspector_registry,
                control_registry,
            ) else {
                return false;
            };
            let Some(field) = read_reflect_path_mut(value.as_mut(), field_path) else {
                return false;
            };
            if !apply(field) {
                return false;
            }
            store_widget_prop_change(
                node_id,
                field_path,
                editor,
                field,
                document,
                widget_registry,
                control_registry,
            );
            true
        }
        InspectorBindingTarget::Style => {
            let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone())
            else {
                return false;
            };
            let style_reflect: &mut dyn PartialReflect = &mut style;
            let Some(field) = read_reflect_path_mut(style_reflect, field_path) else {
                return false;
            };
            if !apply(field) {
                return false;
            }
            apply_style_change(node_id, style, document, widget_registry);
            true
        }
    }
}
