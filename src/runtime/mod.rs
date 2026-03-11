pub mod widget_doc;

use bevy::prelude::*;

use crate::core;
use crate::ensure_plugin_added;

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

pub struct VistaUiRuntimePlugin;

impl Plugin for VistaUiRuntimePlugin {
    fn build(&self, app: &mut App) {
        ensure_plugin_added(app, core::VistaUiCorePlugin);
        app.init_resource::<widget_doc::WidgetDocStore>();
    }
}
