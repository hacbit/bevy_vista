use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(ColorInspectorDriver)
}

struct ColorInspectorDriver;

impl InspectorDriver for ColorInspectorDriver {
    fn id(&self) -> InspectorDriverId {
        INSPECTOR_DRIVER_COLOR
    }

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let entity = ColorFieldBuilder::new()
            .width(px(180.0))
            .disabled(true)
            .build(commands, theme);
        commands.entity(entity).insert(InspectorControlBinding {
            field_path: field.field_path.clone(),
            editor: field.editor,
            numeric_min: field.numeric_min,
            target: InspectorBindingTarget::Style,
        });
        entity
    }

    fn serialize(
        &self,
        _editor: InspectorFieldEditor,
        field: &dyn PartialReflect,
        _theme: Option<&Theme>,
    ) -> Option<String> {
        let color = read_color_field(field)?.to_srgba();
        Some(format!(
            "{},{},{},{}",
            color.red, color.green, color.blue, color.alpha
        ))
    }

    fn apply_serialized(
        &self,
        _editor: InspectorFieldEditor,
        field: &mut dyn PartialReflect,
        raw: &str,
        _numeric_min: Option<f32>,
        _theme: Option<&Theme>,
    ) -> bool {
        let parts = raw.split(',').collect::<Vec<_>>();
        if parts.len() != 4 {
            return false;
        }
        let Ok(r) = parts[0].parse::<f32>() else {
            return false;
        };
        let Ok(g) = parts[1].parse::<f32>() else {
            return false;
        };
        let Ok(b) = parts[2].parse::<f32>() else {
            return false;
        };
        let Ok(a) = parts[3].parse::<f32>() else {
            return false;
        };
        write_color_field(field, Color::srgba(r, g, b, a))
    }

    fn install_systems(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_inspector_color_driver_changes.before(refresh_inspector_panel),
                sync_inspector_color_controls.after(sync_widget_property_section),
            )
                .run_if(in_state(crate::editor::VistaEditorInitPhase::Finalize)),
        );
    }
}

fn apply_inspector_color_driver_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    mut changes: MessageReader<ColorFieldChange>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    controls: Query<&InspectorControlBinding>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
) {
    if options.is_preview_mode {
        changes.clear();
        return;
    }
    let Some(node_id) = panel_state.selected_node else {
        return;
    };

    for change in changes.read() {
        let Ok(control) = controls.get(change.entity) else {
            continue;
        };
        if control.editor.driver_id != INSPECTOR_DRIVER_COLOR {
            continue;
        }
        if matches!(control.target, InspectorBindingTarget::WidgetProp) {
            let Some(mut value) = selected_node_widget_reflect(
                &panel_state,
                &document,
                &widget_registry,
                &inspector_registry,
                &control_registry,
                None,
            ) else {
                continue;
            };
            let Some(field) = read_reflect_path_mut(value.as_mut(), &control.field_path) else {
                continue;
            };
            if !write_color_field(field, change.color) {
                continue;
            }
            store_widget_prop_change(
                node_id,
                &control.field_path,
                control.editor,
                field,
                &mut document,
                &widget_registry,
                &control_registry,
                None,
            );
            continue;
        }
        let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone()) else {
            continue;
        };
        let style_reflect: &mut dyn PartialReflect = &mut style;
        let Some(field) = read_reflect_path_mut(style_reflect, &control.field_path) else {
            continue;
        };
        if !write_color_field(field, change.color) {
            continue;
        }
        apply_style_change(node_id, style, &mut document, &widget_registry);
    }
}

fn sync_inspector_color_controls(
    panel_state: Res<InspectorPanelState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    mut color_controls: Query<(&InspectorControlBinding, &mut ColorField)>,
) {
    if !panel_state.is_changed() && !document.is_changed() {
        return;
    }

    let Some(style) = selected_node_style(&panel_state, &document) else {
        for (_, mut field) in color_controls.iter_mut() {
            field.disabled = true;
        }
        return;
    };
    let widget_reflect = selected_node_widget_reflect(
        &panel_state,
        &document,
        &widget_registry,
        &inspector_registry,
        &control_registry,
        None,
    );

    for (binding, mut field) in color_controls.iter_mut() {
        let source: Option<&dyn PartialReflect> = match binding.target {
            InspectorBindingTarget::Style => {
                read_reflect_path(style as &dyn PartialReflect, &binding.field_path)
            }
            InspectorBindingTarget::WidgetProp => widget_reflect
                .as_deref()
                .and_then(|value| read_reflect_path(value, &binding.field_path)),
        };
        let Some(style_field) = source else {
            field.disabled = true;
            continue;
        };
        if binding.editor.driver_id != INSPECTOR_DRIVER_COLOR {
            field.disabled = true;
            continue;
        }
        if let Some(color) = read_color_field(style_field) {
            field.color = color;
            field.disabled = false;
        } else {
            field.disabled = true;
        }
    }
}
