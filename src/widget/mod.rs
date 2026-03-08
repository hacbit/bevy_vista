//! default builder for widgets

use std::any::TypeId;

use bevy::app::{App, Plugin, PluginGroup, PluginGroupBuilder};
use bevy::color::prelude::*;
use bevy::ecs::prelude::*;
use bevy::picking::prelude::Pickable;
use bevy::prelude::Update;
use bevy::reflect::Reflect;
use bevy::text::prelude::*;
use bevy::ui::prelude::*;
use bevy::utils::{TypeIdMap, prelude::*};
use bevy::{camera::visibility::Visibility, platform::collections::HashMap};

use bevy_vista_macros::{ShowInInspector, Widget};

use crate as bevy_vista;

pub mod common;
pub use common::*;
pub mod input;
pub use input::*;
pub mod layout;
pub use layout::*;

pub struct DefaultUiWidgetsPlugins;

impl PluginGroup for DefaultUiWidgetsPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            // common
            .add(ButtonWidgetPlugin)
            // input
            .add(TextFieldPlugin)
            .add(NumericFieldsPlugin)
            .add(CheckboxPlugin)
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

    pub fn spawn_default_widget(
        &self,
        path: &str,
        commands: &mut Commands,
        theme: Option<&crate::theme::Theme>,
    ) -> Option<Entity> {
        self.get_widget_by_path(path)
            .map(|registration| registration.spawn_default(commands, theme))
    }
}

pub type WidgetId = (&'static str, &'static str);

pub struct WidgetRegistration {
    category: &'static str,
    name: &'static str,
    type_id: TypeId,
    spawn_default_fn: fn(&mut Commands, Option<&crate::theme::Theme>) -> Entity,
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
            spawn_default_fn: B::spawn_default,
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

    pub fn full_path(&self) -> String {
        format!("{}/{}", self.category, self.name)
    }

    pub fn spawn_default(
        &self,
        commands: &mut Commands,
        theme: Option<&crate::theme::Theme>,
    ) -> Entity {
        (self.spawn_default_fn)(commands, theme)
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
    fn spawn_default(commands: &mut Commands, theme: Option<&crate::theme::Theme>) -> Entity;
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
    pub display: Display,
    #[property(label = "Visible", editor = "visibility")]
    pub visibility: Visibility,
    pub overflow: Overflow,
    pub overflow_clip_margin: OverflowClipMargin,

    // position
    #[property(label = "Position")]
    pub position_type: PositionType,
    pub left: Val,
    pub right: Val,
    pub top: Val,
    pub bottom: Val,

    // flex
    pub flex_basis: Val,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    #[property(label = "Direction")]
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,

    // alignment
    #[property(label = "Align")]
    pub align_items: AlignItems,
    pub justify_items: JustifyItems,
    pub align_self: AlignSelf,
    pub justify_self: JustifySelf,
    pub align_content: AlignContent,
    #[property(label = "Justify")]
    pub justify_content: JustifyContent,

    // size
    #[property(min = 1.0)]
    pub width: Val,
    #[property(min = 1.0)]
    pub height: Val,
    #[property(min = 1.0)]
    pub min_width: Val,
    #[property(min = 1.0)]
    pub min_height: Val,
    #[property(min = 1.0)]
    pub max_width: Val,
    #[property(min = 1.0)]
    pub max_height: Val,

    // box model
    pub box_sizing: BoxSizing,
    pub margin: UiRect,
    #[property(min = 0.0)]
    pub padding: UiRect,

    // background
    #[property(label = "Background", editor = "color_preset")]
    pub background_color: Color,

    // border
    pub border: UiRect,
    pub border_radius: BorderRadius,
    pub border_color: BorderColor,

    // transform
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

pub fn spawn_blueprint_widget_content(
    registry: &WidgetRegistry,
    commands: &mut Commands,
    widget_path: &str,
    style: &WidgetStyle,
    theme: Option<&crate::theme::Theme>,
) -> Option<Entity> {
    let content = registry.spawn_default_widget(widget_path, commands, theme)?;
    if style != &WidgetStyle::default() {
        style.apply_to_entity(commands, content);
    }
    Some(content)
}

// pub struct TextStyle {
//     pub text_font: TextFont,
//     pub text_color: Color,
//     pub text_layout: TextLayout,
// }
