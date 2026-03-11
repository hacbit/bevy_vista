//! Shared Vista UI foundation.
//!
//! This module contains the reusable building blocks that are not specific to
//! the runtime document manager or the editor overlay:
//! - asset/document representations
//! - widget registrations and built-in widgets
//! - theme types
//! - inspector metadata and built-in drivers
//! - editor icon definitions used across the crate
//!
//! Most users will interact with this layer through [`prelude`] or
//! [`VistaUiCorePlugin`].
pub mod asset;
pub mod icons;
pub mod inspector;
pub mod theme;
pub mod widget;

/// Convenience imports for the shared Vista UI foundation.
///
/// This prelude focuses on data models and widget/theme/inspector definitions
/// that are usable without the editor overlay or runtime document utilities.
pub mod prelude {
    pub use super::VistaUiCorePlugin;
    pub use super::asset::{
        VISTA_UI_ASSET_EXTENSION, VISTA_UI_ASSET_VERSION, VistaAssetPlugin, VistaNodeId,
        VistaUiAsset, VistaUiAssetError, VistaUiNodeAsset, VistaUiSpawnResult,
    };
    pub use super::icons::{EditorIconsPlugin, Icons};
    pub use super::inspector::{
        BlueprintCommand, BlueprintCommandError, BlueprintNodeId, BlueprintNodeRef,
        BlueprintRuntimeMap, INSPECTOR_DRIVER_BOOL, INSPECTOR_DRIVER_CHOICE,
        INSPECTOR_DRIVER_COLOR, INSPECTOR_DRIVER_NUMBER, INSPECTOR_DRIVER_STRING,
        INSPECTOR_DRIVER_VAL, INSPECTOR_DRIVER_VEC2, InspectorDriverId, InspectorEditorRegistry,
        InspectorEntryDescriptor, InspectorFieldDescriptor, InspectorFieldEditor,
        InspectorFieldMetadata, InspectorFieldOptions, InspectorHeaderDescriptor,
        InspectorHeaderOptions, InspectorTypeEditorResolver, ShowInInspector,
        WidgetBlueprintDocument, WidgetBlueprintNode, apply_blueprint_command,
        inspector_metadata_for, number_kind_for_field, parse_number_for_field, read_bool_field,
        read_choice_field, read_color_field, read_number_field, read_string_field, read_val_field,
        read_vec2_field, val_unit_options, write_bool_field, write_choice_field, write_color_field,
        write_number_field, write_number_kind_field, write_string_field, write_val_number_field,
        write_val_unit_field, write_vec2_axis_field,
    };
    pub use super::theme::*;
    pub use super::theme::{
        EditorTheme, Theme, ThemeBoundary, ThemeMode, ThemeScope, ViewportThemeState,
    };
    pub use super::widget::*;
    pub use crate::bevy_vista_macros::{ShowInInspector, Widget};
}

use crate::ensure_plugin_added;
use bevy::prelude::*;

/// Shared Vista UI foundation.
///
/// This plugin installs the common building blocks used by both runtime and
/// editor layers, including widget registrations, asset types, built-in
/// inspector drivers, and default theme resources.
pub struct VistaUiCorePlugin;

impl Plugin for VistaUiCorePlugin {
    fn build(&self, app: &mut App) {
        ensure_plugin_added(app, asset::VistaAssetPlugin);
        ensure_plugin_added(app, icons::EditorIconsPlugin);
        ensure_plugin_added(app, widget::VistaWidgetsPlugin);
        app.init_resource::<inspector::InspectorEditorRegistry>();
        inspector::runtime::init_inspector_runtime(app);
        if !app.world().contains_resource::<theme::Theme>() {
            let default_theme =
                theme::Theme::quick_from_hex("Default Theme", "#C83A6C", theme::ThemeMode::Dark)
                    .unwrap();
            app.insert_resource(default_theme);
        }
    }
}
