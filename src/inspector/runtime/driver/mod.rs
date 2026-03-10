use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use super::*;

mod bool;
mod choice;
mod color;
mod number;
mod string;
mod val;
mod vec2;

pub trait InspectorDriver: Send + Sync + 'static {
    fn id(&self) -> InspectorDriverId;

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity;

    fn supports(&self, editor: InspectorFieldEditor) -> bool {
        editor.driver_id == self.id()
    }

    fn serialize(
        &self,
        _editor: InspectorFieldEditor,
        _field: &dyn PartialReflect,
        _theme: Option<&Theme>,
    ) -> Option<String> {
        None
    }

    fn apply_serialized(
        &self,
        _editor: InspectorFieldEditor,
        _field: &mut dyn PartialReflect,
        _raw: &str,
        _numeric_min: Option<f32>,
        _theme: Option<&Theme>,
    ) -> bool {
        false
    }

    fn retarget_control(
        &self,
        commands: &mut Commands,
        control: Entity,
        target: InspectorBindingTarget,
    ) {
        retarget_standard_control(commands, control, target);
    }

    fn install_systems(&self, _app: &mut App) {}
}

pub trait InspectorDriverAppExt {
    fn register_inspector_driver<D>(&mut self, driver: D) -> &mut Self
    where
        D: InspectorDriver;

    fn register_boxed_inspector_driver(&mut self, driver: Arc<dyn InspectorDriver>) -> &mut Self;
}

impl InspectorDriverAppExt for App {
    fn register_inspector_driver<D>(&mut self, driver: D) -> &mut Self
    where
        D: InspectorDriver,
    {
        self.register_boxed_inspector_driver(Arc::new(driver))
    }

    fn register_boxed_inspector_driver(&mut self, driver: Arc<dyn InspectorDriver>) -> &mut Self {
        driver.install_systems(self);
        self.world_mut()
            .resource_mut::<InspectorControlRegistry>()
            .register(driver);
        self
    }
}

pub(super) fn install_inspector_drivers(app: &mut App) {
    for driver in default_inspector_drivers() {
        app.register_boxed_inspector_driver(driver);
    }
}

fn default_inspector_drivers() -> Vec<Arc<dyn InspectorDriver>> {
    vec![
        number::driver(),
        string::driver(),
        choice::driver(),
        color::driver(),
        bool::driver(),
        val::driver(),
        vec2::driver(),
    ]
}

fn retarget_standard_control(
    commands: &mut Commands,
    control: Entity,
    target: InspectorBindingTarget,
) {
    commands
        .entity(control)
        .entry::<InspectorControlBinding>()
        .and_modify(move |mut binding| {
            binding.target = target.clone();
        });
}
