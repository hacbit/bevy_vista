//! `bevy_vista` provides a document-driven UI editor and runtime workflow for
//! Bevy UI.
//!
//! The crate is organized into three layers:
//! - [`core`]: shared data models, widgets, themes, assets, and inspector
//!   metadata
//! - [`runtime`]: runtime document loading, mutation, spawning, and live widget
//!   access
//! - [`editor`]: editor overlay, viewport tooling, hierarchy, and inspector UI
//!
//! For most users:
//! - use [`prelude`] for the full experience
//! - use [`runtime::prelude`] for runtime-only loading and document workflows
//! - use [`editor::prelude`] for the editor overlay
//!
//! If you are browsing the API for the first time, good starting points are:
//! - [`VistaUiPlugin`]
//! - [`VistaUiRuntimePlugin`]
//! - [`runtime::widget_doc::WidgetDocUtility`]
//! - [`core::asset::VistaUiAsset`]
//!
//! See the repository `README.md`, `docs/USAGE.md`, and the `examples/`
//! directory for end-to-end usage.
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

/// Convenience imports for the full editor + runtime setup.
///
/// This prelude re-exports the layered preludes from:
/// - [`crate::core::prelude`]
/// - [`crate::runtime::prelude`]
/// - [`crate::editor::prelude`]
///
/// # Example
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_vista::prelude::*;
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(VistaUiPlugin)
///         .run();
/// }
/// ```
pub mod prelude {
    pub use super::VistaUiPlugin;
    pub use crate::core::prelude::*;
    pub use crate::editor::prelude::*;
    pub use crate::runtime::prelude::*;
}

/// Full Vista UI setup.
///
/// This plugin installs both [`VistaUiEditorPlugin`] and
/// [`VistaUiRuntimePlugin`]. Use it when you want the editor overlay and the
/// runtime document APIs in the same app.
///
/// For layer-specific setups, prefer:
/// - [`VistaUiCorePlugin`]
/// - [`VistaUiRuntimePlugin`]
/// - [`VistaUiEditorPlugin`]
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
