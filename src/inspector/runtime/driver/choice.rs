use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(ChoiceInspectorDriver)
}

struct ChoiceInspectorDriver;

impl InspectorDriver for ChoiceInspectorDriver {
    fn id(&self) -> InspectorDriverId {
        INSPECTOR_DRIVER_CHOICE
    }

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let entity = DropdownBuilder::new()
            .width(px(144.0))
            .options(default_choice_options(theme))
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
        theme: Option<&Theme>,
    ) -> Option<String> {
        let (options, selected) = read_choice_field(field, theme)?;
        options.get(selected).cloned()
    }

    fn apply_serialized(
        &self,
        _editor: InspectorFieldEditor,
        field: &mut dyn PartialReflect,
        raw: &str,
        _numeric_min: Option<f32>,
        theme: Option<&Theme>,
    ) -> bool {
        let Some((options, _)) = read_choice_field(field, theme) else {
            return false;
        };
        let Some(selected) = options.iter().position(|option| option == raw) else {
            return false;
        };
        write_choice_field(field, selected, theme)
    }

    fn install_systems(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_inspector_choice_driver_changes.before(refresh_inspector_panel),
                sync_inspector_choice_controls.after(sync_widget_property_section),
            )
                .run_if(in_state(crate::editor::VistaEditorInitPhase::Finalize)),
        );
    }
}

fn apply_inspector_choice_driver_changes(
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    editor_theme: Res<EditorTheme>,
    mut changes: MessageReader<DropdownChange>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    controls: Query<&InspectorControlBinding>,
    val_unit_inputs: Query<(), With<InspectorValUnitInput>>,
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
    let theme = Some(&editor_theme.0);

    for change in changes.read() {
        if val_unit_inputs.contains(change.entity) {
            continue;
        }
        let Ok(control) = controls.get(change.entity) else {
            continue;
        };
        if control.editor.driver_id != INSPECTOR_DRIVER_CHOICE {
            continue;
        }
        if matches!(control.target, InspectorBindingTarget::WidgetProp) {
            let Some(mut value) = selected_node_widget_reflect(
                &panel_state,
                &document,
                &widget_registry,
                &inspector_registry,
                &control_registry,
                theme,
            ) else {
                continue;
            };
            let Some(field) = read_reflect_path_mut(value.as_mut(), &control.field_path) else {
                continue;
            };
            if !write_choice_field(field, change.selected, theme) {
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
                theme,
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
        if !write_choice_field(field, change.selected, theme) {
            continue;
        }
        apply_style_change(node_id, style, &mut document, &widget_registry);
    }
}

fn sync_inspector_choice_controls(
    panel_state: Res<InspectorPanelState>,
    document: Res<blueprint::WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    inspector_registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
    editor_theme: Res<EditorTheme>,
    mut dropdown_controls: Query<(&InspectorControlBinding, &mut Dropdown)>,
) {
    let theme = Some(&editor_theme.0);
    if !panel_state.is_changed() && !document.is_changed() && !editor_theme.is_changed() {
        return;
    }

    let Some(style) = selected_node_style(&panel_state, &document) else {
        for (binding, mut dropdown) in dropdown_controls.iter_mut() {
            if binding.editor.driver_id != INSPECTOR_DRIVER_CHOICE {
                continue;
            }
            dropdown.options = default_choice_options(theme);
            dropdown.selected = 0;
            dropdown.expanded = false;
            dropdown.disabled = true;
        }
        return;
    };
    let widget_reflect = selected_node_widget_reflect(
        &panel_state,
        &document,
        &widget_registry,
        &inspector_registry,
        &control_registry,
        theme,
    );

    for (binding, mut dropdown) in dropdown_controls.iter_mut() {
        let source: Option<&dyn PartialReflect> = match binding.target {
            InspectorBindingTarget::Style => {
                read_reflect_path(style as &dyn PartialReflect, &binding.field_path)
            }
            InspectorBindingTarget::WidgetProp => widget_reflect
                .as_deref()
                .and_then(|value| read_reflect_path(value, &binding.field_path)),
        };
        let Some(style_field) = source else {
            dropdown.disabled = true;
            continue;
        };
        if binding.editor.driver_id != INSPECTOR_DRIVER_CHOICE {
            dropdown.disabled = true;
            continue;
        }
        if let Some((options, selected)) = read_choice_field(style_field, theme) {
            dropdown.options = options;
            dropdown.selected = selected;
            dropdown.expanded = false;
            dropdown.disabled = false;
        } else {
            dropdown.disabled = true;
        }
    }
}
