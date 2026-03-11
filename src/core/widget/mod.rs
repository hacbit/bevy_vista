//! default builder for widgets

use std::any::TypeId;

use bevy::app::PluginGroupBuilder;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::utils::TypeIdMap;

use crate as bevy_vista;
use crate::bevy_vista_macros::{ShowInInspector, Widget};
use crate::core::icons::Icons;
use crate::core::inspector::runtime::InspectorControlRegistry;
use crate::core::inspector::{
    InspectorEditorRegistry, InspectorEntryDescriptor, apply_serialized_editor_value,
    read_reflect_path_mut,
};
use crate::core::theme::{Theme, ThemeBoundary, ThemeScope};

pub mod common;
pub use common::{
    ButtonBuilder, ButtonWidget, ButtonWidgetPlugin, ImageBuilder, ImageWidget, ImageWidgetPlugin,
    LabelBuilder, LabelWidget, LabelWidgetPlugin, NodeBuilder, NodeWidget,
};
pub mod input;
pub use input::{
    Checkbox, CheckboxBuilder, CheckboxChange, CheckboxPlugin, ColorField, ColorFieldBuilder,
    ColorFieldChange, ColorFieldMode, ColorFieldPlugin, Dropdown, DropdownBuilder, DropdownChange,
    DropdownPlugin, Number, NumberField, NumberFieldBuilder, NumberFieldChange, NumberFieldPlugin,
    NumberKind, NumericFieldsPlugin, TextField, TextFieldBuilder, TextFieldLayoutMode,
    TextFieldPlugin, TextInputChange, TextInputFormatter, TextInputSubmit, TextInputType,
    TextInputValidator,
};
pub mod layout;
pub use layout::{
    Divider, DividerAxis, DividerBuilder, Foldout, FoldoutBuilder, FoldoutPlugin, ListView,
    ListViewBuilder, ListViewItem, ListViewPlugin, ScrollView, ScrollViewBuilder, ScrollViewPlugin,
    ScrollbarVisibility, SplitView, SplitViewAxis, SplitViewBuilder, SplitViewPlugin,
    TreeNodeBuilder, TreeNodeHeader, TreeNodeItemId, TreeNodeState, TreeView, TreeViewBuilder,
    TreeViewPlugin,
};

pub struct DefaultUiWidgetsPlugins;

impl PluginGroup for DefaultUiWidgetsPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            // common
            .add(ButtonWidgetPlugin)
            .add(LabelWidgetPlugin)
            .add(ImageWidgetPlugin)
            // input
            .add(TextFieldPlugin)
            .add(NumericFieldsPlugin)
            .add(CheckboxPlugin)
            .add(ColorFieldPlugin)
            .add(DropdownPlugin)
            // layout
            .add(FoldoutPlugin)
            .add(SplitViewPlugin)
            .add(ListViewPlugin)
            .add(TreeViewPlugin)
            .add(ScrollViewPlugin)
    }
}

pub struct VistaWidgetsPlugin;

impl Plugin for VistaWidgetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WidgetRegistry>()
            .init_resource::<GlobalPopupLayerState>()
            .add_systems(Update, ensure_global_popup_layer_root)
            .add_plugins(DefaultUiWidgetsPlugins);
    }
}

pub mod __macro_exports {
    use super::*;
    pub use inventory;

    pub struct AutomaticWidgetRegistrations(pub fn(&mut WidgetRegistry));

    pub fn register_widgets(registry: &mut WidgetRegistry) {
        for registration in inventory::iter::<AutomaticWidgetRegistrations> {
            (registration.0)(registry);
        }
    }

    inventory::collect!(AutomaticWidgetRegistrations);

    pub trait RegisterForWidget {
        fn __auto_register(registry: &mut WidgetRegistry);
    }

    impl<T: GetWidgetRegistration + 'static> RegisterForWidget for T {
        fn __auto_register(registry: &mut WidgetRegistry) {
            registry.register::<T>();
        }
    }
}

#[derive(Resource)]
pub struct WidgetRegistry {
    registrations: TypeIdMap<WidgetRegistration>,
    path_to_id: HashMap<WidgetId, TypeId>,
    full_path_to_id: HashMap<String, TypeId>,
}

impl Default for WidgetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            registrations: Default::default(),
            path_to_id: Default::default(),
            full_path_to_id: Default::default(),
        };
        registry.register_widgets();
        registry
    }

    fn register_widgets(&mut self) {
        __macro_exports::register_widgets(self);
    }

    pub fn register<T>(&mut self)
    where
        T: GetWidgetRegistration + 'static,
    {
        if self.register_internal(TypeId::of::<T>(), T::get_widget_registration) {
            T::register_widget_dependencies(self);
        }
    }

    fn register_internal(
        &mut self,
        type_id: TypeId,
        get_registration: impl FnOnce() -> WidgetRegistration,
    ) -> bool {
        use bevy::platform::collections::hash_map::Entry;

        match self.registrations.entry(type_id) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                let registration = get_registration();
                Self::update_registration_indices(
                    &registration,
                    &mut self.path_to_id,
                    &mut self.full_path_to_id,
                );
                entry.insert(registration);
                true
            }
        }
    }

    fn update_registration_indices(
        registration: &WidgetRegistration,
        path_to_id: &mut HashMap<WidgetId, TypeId>,
        full_path_to_id: &mut HashMap<String, TypeId>,
    ) {
        let widget_id = (registration.category, registration.name);
        path_to_id.insert(widget_id, registration.type_id());
        full_path_to_id.insert(registration.full_path(), registration.type_id());
    }

    pub fn get_all_widgets(&self) -> impl Iterator<Item = &WidgetRegistration> {
        self.registrations.values()
    }

    pub fn get_widget_by_path(&self, path: &str) -> Option<&WidgetRegistration> {
        let type_id = self.full_path_to_id.get(path)?;
        self.registrations.get(type_id)
    }

    pub fn get_widget<T>(&self) -> Option<&WidgetRegistration>
    where
        T: Widget + 'static,
    {
        self.registrations.get(&TypeId::of::<T>())
    }

    pub fn widget_path<T>(&self) -> Option<String>
    where
        T: Widget + 'static,
    {
        self.get_widget::<T>().map(WidgetRegistration::full_path)
    }

    pub fn spawn_default_widget(
        &self,
        path: &str,
        commands: &mut Commands,
        theme: Option<&Theme>,
    ) -> Option<Entity> {
        self.get_widget_by_path(path)
            .map(|registration| registration.spawn_default(commands, theme).root)
    }
}

pub type WidgetId = (&'static str, &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetChildRule {
    Any,
    Exact(usize),
    Range { max: Option<usize> },
}

impl Default for WidgetChildRule {
    fn default() -> Self {
        Self::Any
    }
}

pub struct WidgetSpawnResult {
    pub root: Entity,
    slots: HashMap<&'static str, Entity>,
}

impl WidgetSpawnResult {
    pub fn new(root: Entity) -> Self {
        Self {
            root,
            slots: HashMap::default(),
        }
    }

    pub fn with_slot(mut self, slot: &'static str, entity: Entity) -> Self {
        self.slots.insert(slot, entity);
        self
    }

    pub fn slot_entity(&self, slot: &str) -> Option<Entity> {
        self.slots.get(slot).copied()
    }
}

impl From<Entity> for WidgetSpawnResult {
    fn from(root: Entity) -> Self {
        Self::new(root)
    }
}

pub struct WidgetRegistration {
    category: &'static str,
    name: &'static str,
    type_id: TypeId,
    child_rule: WidgetChildRule,
    child_slots: &'static [&'static str],
    spawn_default_fn: fn(&mut Commands, Option<&Theme>) -> WidgetSpawnResult,
    inspector_entries_fn: Option<fn(&InspectorEditorRegistry) -> Vec<InspectorEntryDescriptor>>,
    default_inspector_value_fn: Option<fn() -> Box<dyn bevy::reflect::PartialReflect>>,
    apply_props_fn: Option<
        fn(
            &mut Commands,
            Entity,
            &HashMap<String, String>,
            &InspectorEditorRegistry,
            Option<&InspectorControlRegistry>,
            Option<&Theme>,
        ),
    >,
}

impl WidgetRegistration {
    pub fn of<T, B>(category: &'static str, name: &'static str) -> Self
    where
        T: Widget + 'static,
        B: DefaultWidgetBuilder + 'static,
    {
        Self {
            category,
            name,
            type_id: TypeId::of::<T>(),
            child_rule: WidgetChildRule::Any,
            child_slots: &[],
            spawn_default_fn: B::spawn_default,
            inspector_entries_fn: None,
            default_inspector_value_fn: None,
            apply_props_fn: None,
        }
    }

    pub fn of_with_inspector<T, B>(category: &'static str, name: &'static str) -> Self
    where
        T: Widget + Component + Reflect + Default + Clone + 'static,
        B: DefaultWidgetBuilder + 'static,
    {
        Self {
            category,
            name,
            type_id: TypeId::of::<T>(),
            child_rule: WidgetChildRule::Any,
            child_slots: &[],
            spawn_default_fn: B::spawn_default,
            inspector_entries_fn: Some(widget_inspector_entries::<T>),
            default_inspector_value_fn: Some(default_widget_inspector_value::<T>),
            apply_props_fn: Some(apply_widget_props::<T>),
        }
    }

    pub fn category(&self) -> &'static str {
        self.category
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn child_rule(&self) -> WidgetChildRule {
        self.child_rule
    }

    pub fn child_slot_at(&self, index: usize) -> Option<&'static str> {
        match self.child_slots {
            [] => None,
            [slot] => Some(*slot),
            slots => slots.get(index).copied(),
        }
    }

    pub fn child_rule_config(mut self, rule: WidgetChildRule) -> Self {
        self.child_rule = rule;
        self
    }

    pub fn child_slots(mut self, slots: &'static [&'static str]) -> Self {
        self.child_slots = slots;
        self
    }

    pub fn full_path(&self) -> String {
        format!("{}/{}", self.category, self.name)
    }

    pub fn spawn_default(
        &self,
        commands: &mut Commands,
        theme: Option<&Theme>,
    ) -> WidgetSpawnResult {
        (self.spawn_default_fn)(commands, theme)
    }

    pub fn inspector_entries(
        &self,
        registry: &InspectorEditorRegistry,
    ) -> Vec<InspectorEntryDescriptor> {
        self.inspector_entries_fn
            .map(|f| f(registry))
            .unwrap_or_default()
    }

    pub(crate) fn apply_props(
        &self,
        commands: &mut Commands,
        entity: Entity,
        props: &HashMap<String, String>,
        registry: &InspectorEditorRegistry,
        control_registry: Option<&InspectorControlRegistry>,
        theme: Option<&Theme>,
    ) {
        if let Some(apply) = self.apply_props_fn {
            apply(commands, entity, props, registry, control_registry, theme);
        }
    }

    pub fn default_inspector_value(&self) -> Option<Box<dyn bevy::reflect::PartialReflect>> {
        self.default_inspector_value_fn.map(|f| f())
    }
}

/// A trait which allows a type to generate its [`WidgetRegistration`]
/// for registration into the [`WidgetRegistry`].
///
/// This trait is **automatically implemented** for items using [`#[derive(Widget)]`](derive@crate::widget::Widget).
pub trait GetWidgetRegistration {
    /// Returns the default [`WidgetRegistration`] for this type
    fn get_widget_registration() -> WidgetRegistration;
    /// This method is called by [`WidgetRegistry::register`] to register any other required types.
    fn register_widget_dependencies(_registry: &mut WidgetRegistry) {}
}

/// This trait is implemented for items using [`#[derive(Widget)]`](derive@crate::widget::Widget).
pub trait Widget
where
    Self: Sized,
{
    fn category() -> &'static str;

    fn name() -> &'static str;
}

pub trait DefaultWidgetBuilder {
    fn spawn_default(commands: &mut Commands, theme: Option<&Theme>) -> WidgetSpawnResult;
}

#[derive(Component)]
pub struct PopupLayerHost;

#[derive(Component)]
pub struct PopupLayerRoot;

#[derive(Component)]
struct GlobalPopupLayerRoot;

#[derive(Resource, Default)]
pub struct GlobalPopupLayerState {
    pub root: Option<Entity>,
}

const GLOBAL_POPUP_LAYER_Z_INDEX: i32 = 1_000_000;

pub fn resolve_popup_parent(
    entity: Entity,
    parents: &Query<&ChildOf>,
    children: &Query<&Children>,
    popup_hosts: &Query<(), With<PopupLayerHost>>,
    popup_roots: &Query<(), With<PopupLayerRoot>>,
) -> Option<Entity> {
    let mut current = entity;
    loop {
        if popup_hosts.contains(current) {
            if let Ok(host_children) = children.get(current) {
                for child in host_children.iter() {
                    if popup_roots.contains(child) {
                        return Some(child);
                    }
                }
            }
            return Some(current);
        }
        let Ok(parent) = parents.get(current) else {
            return None;
        };
        current = parent.parent();
    }
}

fn ensure_global_popup_layer_root(
    mut commands: Commands,
    mut state: ResMut<GlobalPopupLayerState>,
) {
    if state.root.is_some() {
        return;
    }

    let root = commands
        .spawn((
            Name::new("Global Popup Root"),
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                right: px(0.0),
                top: px(0.0),
                bottom: px(0.0),
                width: percent(100.0),
                height: percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
            GlobalZIndex(GLOBAL_POPUP_LAYER_Z_INDEX),
            PopupLayerRoot,
            GlobalPopupLayerRoot,
        ))
        .id();
    state.root = Some(root);
}

/// Used to select the orientation of a scrollbar, slider, or other oriented control.
#[derive(Default, Debug, Reflect, Clone, Copy, PartialEq)]
pub enum ControlOrientation {
    Horizontal,
    #[default]
    Vertical,
}

#[derive(Reflect, Clone, Default, Debug, PartialEq, ShowInInspector)]
pub struct WidgetStyle {
    // display
    #[property(header = "Display")]
    pub display: Display,
    pub visibility: Visibility,
    pub overflow: Overflow,
    #[property(end_header)]
    pub overflow_clip_margin: OverflowClipMargin,

    // position
    #[property(header = "Position", label = "Position")]
    pub position_type: PositionType,
    pub left: Val,
    pub right: Val,
    pub top: Val,
    #[property(end_header)]
    pub bottom: Val,

    // flex
    #[property(header = "Flex")]
    pub flex_basis: Val,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    #[property(label = "Direction")]
    pub flex_direction: FlexDirection,
    #[property(end_header)]
    pub flex_wrap: FlexWrap,

    // alignment
    #[property(header = "Alignment")]
    pub align_items: AlignItems,
    pub justify_items: JustifyItems,
    pub align_self: AlignSelf,
    pub justify_self: JustifySelf,
    pub align_content: AlignContent,
    #[property(end_header)]
    pub justify_content: JustifyContent,

    // size
    #[property(header = "Size", min = 1.0)]
    pub width: Val,
    #[property(min = 1.0)]
    pub height: Val,
    #[property(min = 1.0)]
    pub min_width: Val,
    #[property(min = 1.0)]
    pub min_height: Val,
    #[property(min = 1.0)]
    pub max_width: Val,
    #[property(min = 1.0, end_header)]
    pub max_height: Val,

    // box model
    #[property(header = "Box Model")]
    pub box_sizing: BoxSizing,
    pub margin: UiRect,
    #[property(min = 0.0, end_header)]
    pub padding: UiRect,

    // background
    #[property(header = "Appearance")]
    pub background_color: Color,

    // border
    #[property(header = "Border")]
    pub border: UiRect,
    pub border_radius: BorderRadius,
    #[property(end_header)]
    pub border_color: BorderColor,

    // transform
    #[property(header = "Transform", end_header)]
    pub transform: UiTransform,
}

impl WidgetStyle {
    pub fn to_node(&self) -> Node {
        Node {
            display: self.display,
            box_sizing: self.box_sizing,
            position_type: self.position_type,
            left: self.left,
            right: self.right,
            top: self.top,
            bottom: self.bottom,
            overflow: self.overflow,
            overflow_clip_margin: self.overflow_clip_margin,
            flex_direction: self.flex_direction,
            flex_wrap: self.flex_wrap,
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: self.flex_basis,
            align_items: self.align_items,
            justify_items: self.justify_items,
            align_self: self.align_self,
            justify_self: self.justify_self,
            align_content: self.align_content,
            justify_content: self.justify_content,
            width: self.width,
            height: self.height,
            min_width: self.min_width,
            min_height: self.min_height,
            max_width: self.max_width,
            max_height: self.max_height,
            margin: self.margin,
            padding: self.padding,
            border: self.border,
            ..default()
        }
    }

    pub fn apply_to_entity(&self, commands: &mut Commands, entity: Entity) {
        commands.entity(entity).insert((
            self.to_node(),
            self.visibility,
            BackgroundColor(self.background_color),
            self.border_radius,
            self.border_color,
            self.transform,
        ));
    }
}

pub(crate) fn spawn_blueprint_widget_content(
    registry: &WidgetRegistry,
    inspector_registry: &InspectorEditorRegistry,
    control_registry: Option<&InspectorControlRegistry>,
    commands: &mut Commands,
    widget_path: &str,
    style: &WidgetStyle,
    props: &HashMap<String, String>,
    theme: Option<&Theme>,
) -> Option<WidgetSpawnResult> {
    let registration = registry.get_widget_by_path(widget_path)?;
    let spawn = registration.spawn_default(commands, theme);
    registration.apply_props(
        commands,
        spawn.root,
        props,
        inspector_registry,
        control_registry,
        theme,
    );
    if style != &WidgetStyle::default() {
        style.apply_to_entity(commands, spawn.root);
    }
    Some(spawn)
}

fn widget_inspector_entries<T>(registry: &InspectorEditorRegistry) -> Vec<InspectorEntryDescriptor>
where
    T: Reflect + Default + 'static,
{
    registry.entries_for::<T>()
}

fn default_widget_inspector_value<T>() -> Box<dyn bevy::reflect::PartialReflect>
where
    T: Reflect + Default + 'static,
{
    Box::new(T::default())
}

fn apply_widget_props<T>(
    commands: &mut Commands,
    entity: Entity,
    props: &HashMap<String, String>,
    registry: &InspectorEditorRegistry,
    control_registry: Option<&InspectorControlRegistry>,
    theme: Option<&Theme>,
) where
    T: Component + Reflect + Default + Clone + 'static,
{
    let mut value = T::default();
    let entries = registry.entries_for::<T>();
    let reflect: &mut dyn bevy::reflect::PartialReflect = &mut value;
    for entry in entries {
        let InspectorEntryDescriptor::Field(field) = entry else {
            continue;
        };
        let Some(raw) = props.get(&field.field_path) else {
            continue;
        };
        let Some(target) = read_reflect_path_mut(reflect, &field.field_path) else {
            continue;
        };
        let _ = control_registry.is_some_and(|control_registry| {
            control_registry.apply_serialized_value(field.editor, target, raw)
        }) || apply_serialized_editor_value(field.editor, target, raw, theme);
    }
    commands.entity(entity).insert(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_registration_exposes_button_inspector_entries() {
        let widget_registry = WidgetRegistry::new();
        let inspector_registry = InspectorEditorRegistry::default();
        assert!(
            !inspector_registry
                .entries_for::<common::button::ButtonWidget>()
                .is_empty(),
            "button inspector entries should exist before widget registration is queried"
        );
        let direct_registration =
            <common::button::ButtonWidget as GetWidgetRegistration>::get_widget_registration();
        assert!(
            direct_registration
                .inspector_entries(&inspector_registry)
                .len()
                > 0,
            "button registration returned by derive should carry inspector support"
        );
        let registration = widget_registry
            .get_widget_by_path("common/button")
            .expect("button registration should exist");
        assert!(
            !registration
                .inspector_entries(&inspector_registry)
                .is_empty(),
            "button widget should expose inspector entries"
        );
    }

    #[test]
    fn widget_registration_exposes_number_field_inspector_entries_without_manual_whitelist() {
        let widget_registry = WidgetRegistry::new();
        let inspector_registry = InspectorEditorRegistry::default();
        let registration = widget_registry
            .get_widget_by_path("input/number_field")
            .expect("number field registration should exist");
        assert!(
            !registration
                .inspector_entries(&inspector_registry)
                .is_empty(),
            "number field should expose inspector entries from automatic widget registration"
        );
    }
}

// pub struct TextStyle {
//     pub text_font: TextFont,
//     pub text_color: Color,
//     pub text_layout: TextLayout,
// }
