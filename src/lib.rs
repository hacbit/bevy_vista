#![deny(unsafe_code)]

use bevy::prelude::*;
pub use bevy_vista_macros;

pub mod asset;
pub mod editor;
pub mod editor_resources;
pub mod grid;
pub mod icons;
pub mod inspector;
pub mod theme;
pub mod widget;

pub mod prelude {
    pub use super::VistaUiPlugin;
    pub use super::asset::{
        VISTA_UI_ASSET_EXTENSION, VISTA_UI_ASSET_VERSION, VistaAssetPlugin, VistaNodeId,
        VistaUiAsset, VistaUiAssetError, VistaUiNodeAsset, VistaUiSpawnResult,
    };
    pub use super::bevy_vista_macros::{ShowInInspector, Widget};
    pub use super::editor_resources::{
        EditingMode, VistaEditorActive, VistaEditorCanvasInfo, VistaEditorExpanded,
        VistaEditorGridInfo, VistaEditorMode, VistaEditorSelection, VistaEditorViewOptions,
    };
    pub use super::inspector::{
        BlueprintCommand, BlueprintNodeId, InspectorDriverId, InspectorEditorRegistry,
        InspectorFieldDescriptor, InspectorFieldEditor, WidgetBlueprintDocument,
        WidgetBlueprintNode, apply_blueprint_command,
    };
    pub use super::inspector::runtime::{
        InspectorBindingTarget, InspectorContext, InspectorDriver, InspectorDriverAppExt,
        InspectorDriverApplyContext, InspectorDriverRuntimeBuilder, InspectorDriverSelection,
        InspectorDriverSyncContext,
    };
    pub use super::icons::{EditorIconsPlugin, Icons};
    pub use super::theme::{
        EditorTheme, Theme, ThemeBoundary, ThemeMode, ThemeScope, ViewportThemeState,
    };
    pub use super::widget::*;
}

/// # Vista Ui Editor
///
///
pub struct VistaUiPlugin;

macro_rules! ensure_plugins_added {
    ($app:expr, $( $plugin:expr ),* $(,)?) => {
        $(
            ensure_plugin_added($app, $plugin);
        )*
    }
}

impl Plugin for VistaUiPlugin {
    fn build(&self, app: &mut App) {
        use theme::{EditorTheme, Theme, ThemeMode, ViewportThemeState};
        let default_theme =
            Theme::quick_from_hex("Default Theme", "#C83A6C", ThemeMode::Dark).unwrap();
        app.insert_resource(default_theme)
            .init_resource::<EditorTheme>()
            .init_resource::<ViewportThemeState>();

        ensure_plugins_added!(
            app,
            icons::EditorIconsPlugin,
            widget::VistaWidgetsPlugin,
            asset::VistaAssetPlugin
        );
        grid::load_grid_shader(app);
        editor_resources::init_vista_editor_resources(app);
        // canvas::init_canvas(app);
        editor::init_editor_ui(app);
    }
}

#[inline]
fn ensure_plugin_added<T: Plugin>(app: &mut App, plugin: T) {
    if !app.is_plugin_added::<T>() {
        app.add_plugins(plugin);
    }
}
