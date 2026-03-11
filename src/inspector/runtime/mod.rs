mod driver;
mod panel;
mod props;
mod state;
mod sync;

pub(super) use crate::editor_resources::VistaEditorViewOptions;
pub(super) use crate::inspector::{
    WidgetBlueprintDocument, INSPECTOR_DRIVER_BOOL, INSPECTOR_DRIVER_CHOICE, INSPECTOR_DRIVER_COLOR,
    INSPECTOR_DRIVER_NUMBER, INSPECTOR_DRIVER_STRING, INSPECTOR_DRIVER_VAL, INSPECTOR_DRIVER_VEC2,
    InspectorDriverId, InspectorEditorRegistry, InspectorFieldDescriptor, InspectorFieldEditor,
    default_choice_options, parse_number_for_field, read_bool_field, read_choice_field, read_color_field,
    read_number_field, read_string_field, read_val_field, read_vec2_field, val_unit_options,
    write_bool_field, write_choice_field, write_color_field, write_number_field,
    write_number_kind_field, write_string_field, write_val_number_field, write_val_unit_field,
    write_vec2_axis_field,
};
pub(super) use crate::theme::Theme;
pub(super) use crate::widget::{
    Checkbox, CheckboxBuilder, CheckboxChange, ColorField, ColorFieldBuilder, ColorFieldChange,
    Dropdown, DropdownBuilder, DropdownChange, Number, NumberField, NumberFieldBuilder,
    NumberFieldChange, NumberKind, TextField, TextFieldBuilder, TextInputChange, TextInputSubmit,
    WidgetRegistry,
};

pub use driver::{InspectorDriver, InspectorDriverAppExt};
pub use state::{InspectorBindingTarget, InspectorContext};

pub(crate) use panel::init_inspector_panel;
pub(crate) use state::{InspectorPanelState, install_inspector_drivers};
pub(crate) use sync::{
    apply_inspector_name_changes, refresh_inspector_panel,
    sync_inspector_context_from_editor_selection, sync_inspector_field_markers,
    sync_widget_property_section,
};

pub(super) use panel::build_property_entries;
pub(super) use props::{
    apply_selected_field_change, apply_style_change, clear_widget_prop_change, find_ancestor_with,
    selected_binding_source, selected_node_style, selected_node_widget_default_reflect,
    selected_node_widget_reflect,
};
pub(super) use state::*;
