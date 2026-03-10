use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(StringInspectorDriver)
}

struct StringInspectorDriver;

impl InspectorDriver for StringInspectorDriver {
    fn id(&self) -> InspectorDriverId {
        INSPECTOR_DRIVER_STRING
    }

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let entity = TextFieldBuilder::new()
            .width(px(180.0))
            .height(px(28.0))
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
        read_string_field(field)
    }

    fn apply_serialized(
        &self,
        _editor: InspectorFieldEditor,
        field: &mut dyn PartialReflect,
        raw: &str,
        _numeric_min: Option<f32>,
        _theme: Option<&Theme>,
    ) -> bool {
        write_string_field(field, raw.to_owned())
    }

    fn install_systems(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_inspector_string_changes.before(refresh_inspector_panel),
                sync_inspector_string_controls.after(sync_widget_property_section),
            )
                .run_if(in_state(crate::editor::VistaEditorInitPhase::Finalize)),
        );
    }
}

fn apply_inspector_string_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    mut changes: MessageReader<TextInputChange>,
    mut submits: MessageReader<TextInputSubmit>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    controls: Query<&InspectorControlBinding>,
    mut document: ResMut<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
) {
    if options.is_preview_mode {
        changes.clear();
        submits.clear();
        return;
    }
    let Some(node_id) = panel_state.selected_node else {
        return;
    };

    let events = changes
        .read()
        .map(|event| (event.entity, event.value.clone()))
        .chain(
            submits
                .read()
                .map(|event| (event.entity, event.value.clone())),
        );

    for (entity, value) in events {
        let Ok(control) = controls.get(entity) else {
            continue;
        };
        if matches!(control.target, InspectorBindingTarget::WidgetProp) {
            let Some(mut widget_value) = selected_node_widget_reflect(
                &panel_state,
                &document,
                &widget_registry,
                &inspector_registry,
                &control_registry,
                None,
            ) else {
                continue;
            };
            let Some(field) = read_reflect_path_mut(widget_value.as_mut(), &control.field_path)
            else {
                continue;
            };
            if control.editor.driver_id != INSPECTOR_DRIVER_STRING {
                continue;
            }
            if !write_string_field(field, value.clone()) {
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
        if control.editor.driver_id != INSPECTOR_DRIVER_STRING {
            continue;
        }
        if !write_string_field(field, value) {
            continue;
        }
        apply_style_change(node_id, style, &mut document, &widget_registry);
    }
}

fn sync_inspector_string_controls(
    panel_state: Res<InspectorPanelState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    mut string_controls: Query<(&InspectorControlBinding, &mut TextField)>,
) {
    if !panel_state.is_changed() && !document.is_changed() {
        return;
    }

    let Some(style) = selected_node_style(&panel_state, &document) else {
        for (_, mut field) in string_controls.iter_mut() {
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

    for (binding, mut field) in string_controls.iter_mut() {
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
        if binding.editor.driver_id != INSPECTOR_DRIVER_STRING {
            field.disabled = true;
            continue;
        }
        if let Some(value) = read_string_field(style_field) {
            if field.value != value {
                field.value = value;
                field.cursor_pos = field.value.chars().count();
                field.selection = None;
            }
            field.disabled = false;
        } else {
            field.disabled = true;
        }
    }
}
