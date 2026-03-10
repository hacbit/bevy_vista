use std::any::TypeId;
use std::sync::OnceLock;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::reflect::PartialReflect;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InspectorFieldEditor {
    pub driver_id: InspectorDriverId,
}

impl InspectorFieldEditor {
    pub const fn new(driver_id: InspectorDriverId) -> Self {
        Self { driver_id }
    }
}

pub type InspectorDriverId = &'static str;

pub const INSPECTOR_DRIVER_NUMBER: InspectorDriverId = "number";
pub const INSPECTOR_DRIVER_STRING: InspectorDriverId = "string";
pub const INSPECTOR_DRIVER_BOOL: InspectorDriverId = "bool";
pub const INSPECTOR_DRIVER_CHOICE: InspectorDriverId = "choice";
pub const INSPECTOR_DRIVER_COLOR: InspectorDriverId = "color";
pub const INSPECTOR_DRIVER_VAL: InspectorDriverId = "val";
pub const INSPECTOR_DRIVER_VEC2: InspectorDriverId = "vec2";

pub type InspectorTypeEditorResolver = fn(&dyn PartialReflect) -> Option<InspectorFieldEditor>;

#[derive(Clone, Debug, Default)]
pub struct InspectorFieldOptions {
    pub label: Option<String>,
    pub hidden: bool,
    pub numeric_min: Option<f32>,
    pub header: Option<InspectorHeaderOptions>,
    pub end_header: bool,
}

impl InspectorFieldOptions {
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn numeric_min(mut self, min: f32) -> Self {
        self.numeric_min = Some(min);
        self
    }

    pub fn header(mut self, title: impl Into<String>) -> Self {
        self.header = Some(InspectorHeaderOptions::new(title));
        self
    }

    pub fn header_with_options(mut self, options: InspectorHeaderOptions) -> Self {
        self.header = Some(options);
        self
    }

    pub fn end_header(mut self, end_header: bool) -> Self {
        self.end_header = end_header;
        self
    }
}

#[derive(Clone, Debug)]
pub struct InspectorFieldMetadata {
    pub field_name: &'static str,
    pub options: InspectorFieldOptions,
}

#[derive(Clone, Debug)]
pub struct InspectorHeaderOptions {
    pub title: String,
    pub default_open: bool,
}

impl InspectorHeaderOptions {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            default_open: true,
        }
    }

    pub fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }
}

#[derive(Clone, Debug)]
pub struct InspectorHeaderDescriptor {
    pub title: String,
    pub default_open: bool,
    pub implicit_close_previous: bool,
}

#[derive(Clone, Debug)]
pub struct InspectorFieldDescriptor {
    pub field_path: String,
    pub label: String,
    pub editor: InspectorFieldEditor,
    pub numeric_min: Option<f32>,
}

#[derive(Clone, Debug)]
pub enum InspectorEntryDescriptor {
    Header(InspectorHeaderDescriptor),
    Field(InspectorFieldDescriptor),
    EndHeader,
}

pub trait ShowInInspector {
    fn inspector_fields() -> Vec<InspectorFieldMetadata> {
        Vec::new()
    }
}

type InspectorMetadataFn = fn() -> Vec<InspectorFieldMetadata>;

#[derive(Default)]
pub struct InspectorMetadataRegistry {
    entries: HashMap<TypeId, InspectorMetadataFn>,
}

impl InspectorMetadataRegistry {
    fn register<T: ShowInInspector + 'static>(&mut self) {
        self.entries.insert(TypeId::of::<T>(), T::inspector_fields);
    }

    fn metadata_for<T: 'static>(&self) -> Option<Vec<InspectorFieldMetadata>> {
        self.entries.get(&TypeId::of::<T>()).map(|metadata| metadata())
    }
}

fn inspector_metadata_registry() -> &'static InspectorMetadataRegistry {
    static REGISTRY: OnceLock<InspectorMetadataRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = InspectorMetadataRegistry::default();
        __macro_exports::register_inspector_metadata(&mut registry);
        registry
    })
}

pub fn inspector_metadata_for<T: 'static>() -> Option<Vec<InspectorFieldMetadata>> {
    inspector_metadata_registry().metadata_for::<T>()
}

pub mod __macro_exports {
    use super::*;
    pub use inventory;

    pub struct AutomaticInspectorMetadata(pub fn(&mut InspectorMetadataRegistry));

    pub fn register_inspector_metadata(registry: &mut InspectorMetadataRegistry) {
        for registration in inventory::iter::<AutomaticInspectorMetadata> {
            (registration.0)(registry);
        }
    }

    inventory::collect!(AutomaticInspectorMetadata);

    pub trait RegisterForInspectorMetadata {
        fn __auto_register(registry: &mut InspectorMetadataRegistry);
    }

    impl<T: ShowInInspector + 'static> RegisterForInspectorMetadata for T {
        fn __auto_register(registry: &mut InspectorMetadataRegistry) {
            registry.register::<T>();
        }
    }
}
