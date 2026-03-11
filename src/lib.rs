#![deny(unsafe_code)]

use bevy::prelude::*;
pub use bevy_vista_macros;

pub mod core;
pub mod editor;
pub mod runtime;

pub use core::VistaUiCorePlugin;
pub use core::{asset, icons, inspector, theme, widget};
pub use editor::VistaUiEditorPlugin;
pub use editor::{grid, resources as editor_resources};
pub use runtime::VistaUiRuntimePlugin;
pub use runtime::widget_doc;

pub mod prelude {
    pub use super::VistaUiPlugin;
    pub use crate::core::prelude::*;
    pub use crate::editor::prelude::*;
    pub use crate::runtime::prelude::*;
}

/// # Vista Ui Editor
///
///
pub struct VistaUiPlugin;

impl Plugin for VistaUiPlugin {
    fn build(&self, app: &mut App) {
        ensure_plugin_added(app, editor::VistaUiEditorPlugin);
        ensure_plugin_added(app, runtime::VistaUiRuntimePlugin);
    }
}

#[inline]
pub(crate) fn ensure_plugin_added<T: Plugin>(app: &mut App, plugin: T) {
    if !app.is_plugin_added::<T>() {
        app.add_plugins(plugin);
    }
}
