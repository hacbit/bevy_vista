use std::sync::Arc;

use bevy::ecs::system::{BoxedSystem, IntoSystem, System, SystemParam};
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

    fn serialize(&self, _field: &dyn PartialReflect) -> Option<String> {
        None
    }

    fn apply_serialized(&self, _field: &mut dyn PartialReflect, _raw: &str) -> bool {
        false
    }

    fn install_runtime(&self, _builder: &mut InspectorDriverRuntimeBuilder) {}

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

        let mut builder = InspectorDriverRuntimeBuilder::default();
        driver.install_runtime(&mut builder);

        let world = self.world_mut();
        let systems = builder.build(world);
        world
            .resource_mut::<InspectorControlRegistry>()
            .register(driver);
        world
            .resource_mut::<InspectorDriverRuntimeRegistry>()
            .extend(systems);
        self
    }
}

pub(super) fn init_inspector_drivers(app: &mut App) {
    app.init_resource::<InspectorDriverRuntimeRegistry>();
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

pub trait DriverApplySystem<Out = ()>: System<In = (), Out = Out> + Send + 'static {}

impl<Out, T> DriverApplySystem<Out> for T where T: System<In = (), Out = Out> + Send + 'static {}

pub trait IntoDriverApplySystem<M, Out = ()>: Send + 'static {
    type System: DriverApplySystem<Out>;

    fn into_system(this: Self) -> Self::System;
}

impl<M, Out, S> IntoDriverApplySystem<M, Out> for S
where
    S: IntoSystem<(), Out, M> + Send + 'static,
    S::System: DriverApplySystem<Out>,
{
    type System = S::System;

    fn into_system(this: Self) -> Self::System {
        IntoSystem::into_system(this)
    }
}

pub trait DriverSyncSystem<Out = ()>: System<In = (), Out = Out> + Send + 'static {}

impl<Out, T> DriverSyncSystem<Out> for T where T: System<In = (), Out = Out> + Send + 'static {}

pub trait IntoDriverSyncSystem<M, Out = ()>: Send + 'static {
    type System: DriverSyncSystem<Out>;

    fn into_system(this: Self) -> Self::System;
}

impl<M, Out, S> IntoDriverSyncSystem<M, Out> for S
where
    S: IntoSystem<(), Out, M> + Send + 'static,
    S::System: DriverSyncSystem<Out>,
{
    type System = S::System;

    fn into_system(this: Self) -> Self::System {
        IntoSystem::into_system(this)
    }
}

#[derive(Default, Resource)]
struct InspectorDriverRuntimeRegistry {
    apply_systems: Vec<BoxedSystem>,
    sync_systems: Vec<BoxedSystem>,
}

impl InspectorDriverRuntimeRegistry {
    fn extend(&mut self, systems: InspectorDriverRuntimeSystems) {
        self.apply_systems.extend(systems.apply_systems);
        self.sync_systems.extend(systems.sync_systems);
    }
}

#[derive(Default)]
pub struct InspectorDriverRuntimeBuilder {
    apply_systems: Vec<BoxedSystem>,
    sync_systems: Vec<BoxedSystem>,
}

impl InspectorDriverRuntimeBuilder {
    pub fn on_apply<M>(&mut self, apply_system: impl IntoDriverApplySystem<M>) {
        self.apply_systems
            .push(Box::new(IntoDriverApplySystem::into_system(apply_system)));
    }

    pub fn on_sync<M>(&mut self, sync_system: impl IntoDriverSyncSystem<M>) {
        self.sync_systems
            .push(Box::new(IntoDriverSyncSystem::into_system(sync_system)));
    }

    fn build(mut self, world: &mut World) -> InspectorDriverRuntimeSystems {
        for system in &mut self.apply_systems {
            system.initialize(world);
        }
        for system in &mut self.sync_systems {
            system.initialize(world);
        }
        InspectorDriverRuntimeSystems {
            apply_systems: self.apply_systems,
            sync_systems: self.sync_systems,
        }
    }
}

struct InspectorDriverRuntimeSystems {
    apply_systems: Vec<BoxedSystem>,
    sync_systems: Vec<BoxedSystem>,
}

#[derive(SystemParam)]
pub struct InspectorDriverApplyContext<'w, 's> {
    options: Res<'w, VistaEditorViewOptions>,
    panel_state: Res<'w, InspectorPanelState>,
    inspector_registry: Res<'w, InspectorEditorRegistry>,
    control_registry: Res<'w, InspectorControlRegistry>,
    document: ResMut<'w, WidgetBlueprintDocument>,
    widget_registry: Res<'w, WidgetRegistry>,
    bindings: Query<'w, 's, &'static InspectorControlBinding>,
    owners: Query<'w, 's, &'static InspectorControlOwner>,
}

impl<'w, 's> InspectorDriverApplyContext<'w, 's> {
    pub fn can_edit(&self) -> bool {
        !self.options.is_preview_mode
            && self.panel_state.visible
            && self.panel_state.selected_node.is_some()
    }

    pub fn is_control(&self, entity: Entity, driver_id: InspectorDriverId) -> bool {
        self.binding_for(entity)
            .is_some_and(|binding| binding.editor.driver_id == driver_id)
    }

    pub fn write_for<F>(&mut self, entity: Entity, apply: F) -> bool
    where
        F: FnOnce(&mut dyn PartialReflect) -> bool,
    {
        let Some(binding) = self.binding_for(entity).cloned() else {
            return false;
        };
        apply_selected_field_change(
            &InspectorPanelState {
                selected_node: self.panel_state.selected_node,
                visible: self.panel_state.visible,
            },
            &mut self.document,
            &self.widget_registry,
            &self.inspector_registry,
            &self.control_registry,
            &binding.target,
            &binding.field_path,
            binding.editor,
            apply,
        )
    }

    fn binding_for(&self, entity: Entity) -> Option<&InspectorControlBinding> {
        let entity = self.owners.get(entity).map_or(entity, |owner| owner.owner);
        self.bindings.get(entity).ok()
    }
}

#[derive(SystemParam)]
pub struct InspectorDriverSyncContext<'w, 's> {
    panel_state: Res<'w, InspectorPanelState>,
    document: Res<'w, WidgetBlueprintDocument>,
    widget_registry: Res<'w, WidgetRegistry>,
    inspector_registry: Res<'w, InspectorEditorRegistry>,
    control_registry: Res<'w, InspectorControlRegistry>,
    bindings: Query<'w, 's, &'static InspectorControlBinding>,
    owners: Query<'w, 's, &'static InspectorControlOwner>,
}

impl<'w, 's> InspectorDriverSyncContext<'w, 's> {
    pub fn changed(&self) -> bool {
        self.panel_state.is_changed()
            || self.document.is_changed()
            || self.widget_registry.is_changed()
            || self.inspector_registry.is_changed()
            || self.control_registry.is_changed()
    }

    pub fn is_control(&self, entity: Entity, driver_id: InspectorDriverId) -> bool {
        self.binding_for(entity)
            .is_some_and(|binding| binding.editor.driver_id == driver_id)
    }

    pub fn read_for<T>(
        &self,
        entity: Entity,
        read: impl FnOnce(&dyn PartialReflect) -> Option<T>,
    ) -> Option<T> {
        let binding = self.binding_for(entity)?;
        let panel_state = InspectorPanelState {
            selected_node: self.panel_state.selected_node,
            visible: self.panel_state.visible,
        };
        let style = selected_node_style(&panel_state, &self.document)?;
        let widget_reflect = selected_node_widget_reflect(
            &panel_state,
            &self.document,
            &self.widget_registry,
            &self.inspector_registry,
            &self.control_registry,
        );
        let source = selected_binding_source(
            style,
            widget_reflect.as_deref(),
            &binding.target,
            &binding.field_path,
        )?;
        read(source)
    }

    fn binding_for(&self, entity: Entity) -> Option<&InspectorControlBinding> {
        let entity = self.owners.get(entity).map_or(entity, |owner| owner.owner);
        self.bindings.get(entity).ok()
    }
}

pub(crate) fn run_inspector_driver_apply_hooks(world: &mut World) {
    let mut systems = {
        let mut registry = world.resource_mut::<InspectorDriverRuntimeRegistry>();
        std::mem::take(&mut registry.apply_systems)
    };
    for system in &mut systems {
        system.run((), world).unwrap_or_else(|err| {
            panic!(
                "failed to run inspector driver apply system `{}`: {err}",
                system.name()
            )
        });
    }
    world
        .resource_mut::<InspectorDriverRuntimeRegistry>()
        .apply_systems = systems;
}

pub(crate) fn run_inspector_driver_sync_hooks(world: &mut World) {
    let mut systems = {
        let mut registry = world.resource_mut::<InspectorDriverRuntimeRegistry>();
        std::mem::take(&mut registry.sync_systems)
    };
    for system in &mut systems {
        system.run((), world).unwrap_or_else(|err| {
            panic!(
                "failed to run inspector driver sync system `{}`: {err}",
                system.name()
            )
        });
    }
    world
        .resource_mut::<InspectorDriverRuntimeRegistry>()
        .sync_systems = systems;
}
