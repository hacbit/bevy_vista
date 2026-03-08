//! some vista editor state

use bevy::app::App;
use bevy::ecs::{entity::Entity, resource::Resource};
use bevy::math::Vec2;
use bevy::prelude::{Deref, DerefMut};
use bevy::reflect::Reflect;

use crate::widget::WidgetId;

pub(crate) fn init_vista_editor_resources(app: &mut App) {
    app.init_resource::<VistaEditorActive>()
        .init_resource::<VistaEditorExpanded>()
        .init_resource::<VistaEditorMode>()
        .init_resource::<VistaEditorSelection>()
        .init_resource::<VistaEditorViewOptions>()
        .init_resource::<VistaEditorCanvasInfo>()
        .init_resource::<VistaEditorGridInfo>();
}

#[derive(Resource, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
pub struct VistaEditorActive(pub bool);

#[derive(Resource, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
pub struct VistaEditorExpanded(pub bool);

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy, Reflect)]
pub enum VistaEditorMode {
    #[default]
    Floating,
    Fullscreen,
}

#[derive(Resource, Default)]
pub struct VistaEditorSelection {
    pub selected_entity: Option<Entity>,
    pub editing_mode: EditingMode,
    pub widget_to_add: Option<WidgetId>,
}

#[derive(Default, PartialEq, Eq)]
pub enum EditingMode {
    #[default]
    Select,
    Move,
    Scale,
    Add,
}

#[derive(Resource, Default)]
pub struct VistaEditorViewOptions {
    pub show_grid: bool,
    pub snap_to_grid: bool,
    pub show_outlines: bool,
    pub is_preview_mode: bool,
}

#[derive(Resource)]
pub struct VistaEditorCanvasInfo {
    pub canvas_size: Vec2,
}

impl Default for VistaEditorCanvasInfo {
    fn default() -> Self {
        Self {
            canvas_size: Vec2::new(800.0, 600.0),
        }
    }
}

#[derive(Resource)]
pub struct VistaEditorGridInfo {
    pub width: f32,
    pub height: f32,
    pub cell_size: f32,
    pub major_frequency: u32,
}

impl Default for VistaEditorGridInfo {
    fn default() -> Self {
        Self {
            width: 1920.,
            height: 1080.,
            cell_size: 20.,
            major_frequency: 5,
        }
    }
}
