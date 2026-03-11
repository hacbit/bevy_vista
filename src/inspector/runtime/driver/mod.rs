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

pub(super) fn install_inspector_drivers(app: &mut App) {
    app.init_resource::<InspectorDriverRuntimeRegistry>().add_systems(
        Update,
        run_inspector_driver_apply_hooks
            .before(refresh_inspector_panel)
            .run_if(in_state(crate::editor::VistaEditorInitPhase::Finalize)),
    )
    .add_systems(
        Update,
        run_inspector_driver_sync_hooks
            .after(sync_widget_property_section)
            .before(sync_inspector_field_markers)
            .run_if(in_state(crate::editor::VistaEditorInitPhase::Finalize)),
    );

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
pub struct InspectorDriverApplyContext<'w> {
    options: Res<'w, VistaEditorViewOptions>,
    panel_state: Res<'w, InspectorPanelState>,
    inspector_registry: Res<'w, InspectorEditorRegistry>,
    control_registry: Res<'w, InspectorControlRegistry>,
    document: ResMut<'w, WidgetBlueprintDocument>,
    widget_registry: Res<'w, WidgetRegistry>,
}

impl<'w> InspectorDriverApplyContext<'w> {
    pub fn can_edit(&self) -> bool {
        !self.options.is_preview_mode
            && self.panel_state.visible
            && self.panel_state.selected_node.is_some()
    }

    pub(super) fn apply_to_binding<F>(
        &mut self,
        binding: &InspectorControlBinding,
        theme: Option<&Theme>,
        apply: F,
    ) -> bool
    where
        F: FnOnce(&mut dyn PartialReflect) -> bool,
    {
        self.apply_to_field(
            &binding.target,
            &binding.field_path,
            binding.editor,
            theme,
            apply,
        )
    }

    pub fn apply_to_field<F>(
        &mut self,
        target: &InspectorBindingTarget,
        field_path: &str,
        editor: InspectorFieldEditor,
        theme: Option<&Theme>,
        apply: F,
    ) -> bool
    where
        F: FnOnce(&mut dyn PartialReflect) -> bool,
    {
        apply_selected_field_change(
            &InspectorPanelState {
                selected_node: self.panel_state.selected_node,
                visible: self.panel_state.visible,
            },
            &mut self.document,
            &self.widget_registry,
            &self.inspector_registry,
            &self.control_registry,
            target,
            field_path,
            editor,
            theme,
            apply,
        )
    }
}

#[derive(SystemParam)]
pub struct InspectorDriverSyncContext<'w> {
    panel_state: Res<'w, InspectorPanelState>,
    document: Res<'w, WidgetBlueprintDocument>,
    widget_registry: Res<'w, WidgetRegistry>,
    inspector_registry: Res<'w, InspectorEditorRegistry>,
    control_registry: Res<'w, InspectorControlRegistry>,
}

impl<'w> InspectorDriverSyncContext<'w> {
    pub fn changed(&self) -> bool {
        self.panel_state.is_changed()
            || self.document.is_changed()
            || self.widget_registry.is_changed()
            || self.inspector_registry.is_changed()
            || self.control_registry.is_changed()
    }

    pub fn selection(&self) -> Option<InspectorDriverSelection<'_>> {
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
            None,
        );
        Some(InspectorDriverSelection {
            style,
            widget_reflect,
        })
    }
}

pub struct InspectorDriverSelection<'a> {
    style: &'a crate::widget::WidgetStyle,
    widget_reflect: Option<Box<dyn PartialReflect>>,
}

impl<'a> InspectorDriverSelection<'a> {
    pub fn source(
        &self,
        target: &InspectorBindingTarget,
        field_path: &str,
    ) -> Option<&dyn PartialReflect> {
        selected_binding_source(self.style, self.widget_reflect.as_deref(), target, field_path)
    }

    pub(super) fn binding_source(
        &self,
        binding: &InspectorControlBinding,
    ) -> Option<&dyn PartialReflect> {
        self.source(&binding.target, &binding.field_path)
    }
}

fn run_inspector_driver_apply_hooks(world: &mut World) {
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
    world.resource_mut::<InspectorDriverRuntimeRegistry>().apply_systems = systems;
}

fn run_inspector_driver_sync_hooks(world: &mut World) {
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
    world.resource_mut::<InspectorDriverRuntimeRegistry>().sync_systems = systems;
}
