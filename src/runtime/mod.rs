//! Runtime document workflow for Vista UI.
//!
//! This module builds on [`crate::core`] and provides APIs for:
//! - loading `.vista.ron` documents
//! - mutating document data before spawning
//! - spawning document instances into the world
//! - querying and mutating live widget components after spawning
//!
//! The main entry points are [`VistaUiRuntimePlugin`] and
//! [`widget_doc::WidgetDocUtility`].
pub mod widget_doc;

use bevy::prelude::*;

use crate::core;
use crate::ensure_plugin_added;

/// Convenience imports for runtime-only usage.
///
/// This prelude re-exports [`crate::core::prelude`] and runtime document APIs
/// such as [`widget_doc::WidgetDocUtility`].
pub mod prelude {
    pub use super::VistaUiRuntimePlugin;
    pub use super::widget_doc::{
        WidgetDocError, WidgetDocId, WidgetDocInstanceId, WidgetDocLiveMut, WidgetDocLiveRef,
        WidgetDocUtility,
    };
    pub use crate::core::inspector::runtime::{
        InspectorContext, InspectorDriver, InspectorDriverAppExt, InspectorDriverApplyContext,
        InspectorDriverRuntimeBuilder, InspectorDriverSyncContext,
    };
    pub use crate::core::prelude::*;
}

/// Runtime-facing Vista UI setup.
///
/// This plugin builds on [`crate::core::VistaUiCorePlugin`] and installs the
/// document store used by [`widget_doc::WidgetDocUtility`].
///
/// Use this when you want to load, mutate, and spawn `.vista.ron` documents at
/// runtime without the editor overlay.
pub struct VistaUiRuntimePlugin;

impl Plugin for VistaUiRuntimePlugin {
    fn build(&self, app: &mut App) {
        ensure_plugin_added(app, core::VistaUiCorePlugin);
        app.init_resource::<widget_doc::WidgetDocStore>();
    }
}
