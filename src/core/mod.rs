pub mod asset;
pub mod icons;
pub mod inspector;
pub mod theme;
pub mod widget;

pub mod prelude {
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
    pub use super::widget::*;
}
