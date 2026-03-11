use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use crate::inspector::{
    BlueprintNodeId, InspectorDriverId, InspectorFieldDescriptor, InspectorFieldEditor,
};
use crate::theme::Theme;

use super::driver::{self, InspectorDriver};

pub(crate) fn install_inspector_drivers(app: &mut App) {
    app.init_resource::<InspectorControlRegistry>();
    driver::install_inspector_drivers(app);
}

#[derive(Resource, Default)]
pub struct InspectorContext {
    pub selected_node: Option<BlueprintNodeId>,
    pub visible: bool,
}

impl InspectorContext {
    pub fn clear(&mut self) {
        self.selected_node = None;
        self.visible = false;
    }

    pub fn select(&mut self, node_id: BlueprintNodeId) {
        self.selected_node = Some(node_id);
        self.visible = true;
    }
}

pub type InspectorPanelState = InspectorContext;

#[derive(Resource)]
pub(crate) struct InspectorControlRegistry {
    registrations:
        bevy::platform::collections::HashMap<InspectorDriverId, Arc<dyn InspectorDriver>>,
}

impl Default for InspectorControlRegistry {
    fn default() -> Self {
        Self {
            registrations: bevy::platform::collections::HashMap::default(),
        }
    }
}

impl InspectorControlRegistry {
    pub(super) fn register(&mut self, driver: Arc<dyn InspectorDriver>) {
        self.registrations.insert(driver.id(), driver);
    }

    pub(super) fn registration_for(
        &self,
        editor: InspectorFieldEditor,
    ) -> Option<&Arc<dyn InspectorDriver>> {
        let driver = self.registrations.get(&editor.driver_id)?;
        driver.supports(editor).then_some(driver)
    }

    pub(super) fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
        target: InspectorBindingTarget,
    ) -> Entity {
        let Some(driver) = self.registration_for(field.editor) else {
            panic!(
                "missing inspector control builder for {}",
                field.editor.driver_id
            );
        };
        let control = driver.build(commands, field, theme);
        commands.entity(control).insert(InspectorControlBinding {
            field_path: field.field_path.clone(),
            editor: field.editor,
            target,
        });
        control
    }

    pub(super) fn serialize_value(
        &self,
        editor: InspectorFieldEditor,
        field: &dyn PartialReflect,
    ) -> Option<String> {
        let driver = self.registration_for(editor)?;
        driver.serialize(field)
    }

    pub(crate) fn apply_serialized_value(
        &self,
        editor: InspectorFieldEditor,
        field: &mut dyn PartialReflect,
        raw: &str,
    ) -> bool {
        let Some(driver) = self.registration_for(editor) else {
            return false;
        };
        driver.apply_serialized(field, raw)
    }
}

#[derive(Component)]
pub(crate) struct InspectorContentRoot;

#[derive(Component)]
pub(crate) struct InspectorWidgetSectionRoot;

#[derive(Component, Default)]
pub(crate) struct InspectorWidgetSectionState {
    pub(super) selected_node: Option<BlueprintNodeId>,
    pub(super) widget_path: Option<String>,
}

#[derive(Component)]
pub(crate) struct InspectorNameField;

#[derive(Component, Clone)]
pub(crate) enum InspectorBindingTarget {
    Style,
    WidgetProp,
}

#[derive(Component, Clone)]
pub(super) struct InspectorControlBinding {
    pub(super) field_path: String,
    pub(super) editor: InspectorFieldEditor,
    pub(super) target: InspectorBindingTarget,
}

#[derive(Component, Copy, Clone)]
pub(super) struct InspectorControlOwner {
    pub(super) owner: Entity,
}

#[derive(Component, Clone)]
pub(crate) struct InspectorFieldDecoration {
    pub(super) field_path: String,
    pub(super) target: InspectorBindingTarget,
}

#[derive(Component)]
pub(crate) struct InspectorFieldLabel;

#[derive(Component)]
pub(crate) struct InspectorFieldRow;

#[derive(Component)]
pub(crate) struct InspectorResetButton;
