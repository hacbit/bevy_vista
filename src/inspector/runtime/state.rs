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
    ) -> Entity {
        let Some(driver) = self.registration_for(field.editor) else {
            panic!(
                "missing inspector control builder for {}",
                field.editor.driver_id
            );
        };
        driver.build(commands, field, theme)
    }

    pub(super) fn serialize_value(
        &self,
        editor: InspectorFieldEditor,
        field: &dyn PartialReflect,
        theme: Option<&Theme>,
    ) -> Option<String> {
        let driver = self.registration_for(editor)?;
        driver.serialize(editor, field, theme)
    }

    pub(super) fn apply_serialized_value(
        &self,
        editor: InspectorFieldEditor,
        field: &mut dyn PartialReflect,
        raw: &str,
        numeric_min: Option<f32>,
        theme: Option<&Theme>,
    ) -> bool {
        let Some(driver) = self.registration_for(editor) else {
            return false;
        };
        driver.apply_serialized(editor, field, raw, numeric_min, theme)
    }

    pub(super) fn retarget_control(
        &self,
        commands: &mut Commands,
        editor: InspectorFieldEditor,
        control: Entity,
        target: InspectorBindingTarget,
    ) {
        let Some(driver) = self.registration_for(editor) else {
            return;
        };
        driver.retarget_control(commands, control, target);
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
pub enum InspectorBindingTarget {
    Style,
    WidgetProp,
}

#[derive(Component, Clone)]
pub(super) struct InspectorControlBinding {
    pub(super) field_path: String,
    pub(super) editor: InspectorFieldEditor,
    pub(super) target: InspectorBindingTarget,
}

#[derive(Component, Clone)]
pub(super) struct InspectorNumberControl {
    pub(super) field_path: String,
    pub(super) numeric_min: Option<f32>,
    pub(super) target: InspectorBindingTarget,
    pub(super) value_input: Entity,
    pub(super) kind_input: Entity,
}

#[derive(Component, Copy, Clone)]
pub(super) struct InspectorNumberValueInput {
    pub(super) owner: Entity,
}

#[derive(Component, Copy, Clone)]
pub(super) struct InspectorNumberKindInput {
    pub(super) owner: Entity,
}

#[derive(Component, Clone)]
pub(super) struct InspectorValControl {
    pub(super) field_path: String,
    pub(super) numeric_min: Option<f32>,
    pub(super) target: InspectorBindingTarget,
    pub(super) value_input: Entity,
    pub(super) unit_input: Entity,
}

#[derive(Component, Copy, Clone)]
pub(super) struct InspectorValValueInput {
    pub(super) owner: Entity,
}

#[derive(Component, Copy, Clone)]
pub(super) struct InspectorValUnitInput {
    pub(super) owner: Entity,
}

#[derive(Component, Clone)]
pub(super) struct InspectorVec2Control {
    pub(super) field_path: String,
    pub(super) target: InspectorBindingTarget,
    pub(super) x_input: Entity,
    pub(super) y_input: Entity,
}

#[derive(Component, Copy, Clone)]
pub(super) struct InspectorVec2AxisInput {
    pub(super) owner: Entity,
    pub(super) axis: usize,
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
