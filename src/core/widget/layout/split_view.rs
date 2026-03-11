use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};

use crate::core::theme::resolve_theme_or_global;

use super::*;

pub struct SplitViewPlugin;

impl Plugin for SplitViewPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Reflect, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitViewAxis {
    #[default]
    Horizontal,
    Vertical,
}

impl SplitViewAxis {
    fn divider_axis(self) -> DividerAxis {
        match self {
            Self::Horizontal => DividerAxis::Vertical,
            Self::Vertical => DividerAxis::Horizontal,
        }
    }

    fn divider_cursor(self) -> CursorIcon {
        match self {
            Self::Horizontal => CursorIcon::System(SystemCursorIcon::ColResize),
            Self::Vertical => CursorIcon::System(SystemCursorIcon::RowResize),
        }
    }

    fn flex_direction(self) -> FlexDirection {
        match self {
            Self::Horizontal => FlexDirection::Row,
            Self::Vertical => FlexDirection::Column,
        }
    }
}

#[derive(Component, Reflect, Clone, Widget, ShowInInspector)]
#[widget("layout/split_view", children = "exact(2)", slots = "first,second")]
#[builder(SplitViewBuilder)]
pub struct SplitView {
    #[property(label = "Axis")]
    pub axis: SplitViewAxis,
    #[property(label = "First Size")]
    pub default_first_size: Val,
    #[property(label = "Divider Size", min = 1.0)]
    pub divider_size: Val,
    #[property(label = "Min First", min = 0.0)]
    pub min_first_size: Val,
    #[property(label = "Min Second", min = 0.0)]
    pub min_second_size: Val,
    #[property(label = "Draggable")]
    pub draggable: bool,
}

impl Default for SplitView {
    fn default() -> Self {
        Self {
            axis: SplitViewAxis::Horizontal,
            default_first_size: Val::Percent(30.0),
            divider_size: Val::Px(2.0),
            min_first_size: Val::Px(100.0),
            min_second_size: Val::Px(100.0),
            draggable: true,
        }
    }
}

#[derive(Component, Clone, Copy)]
pub(crate) struct SplitViewPanels {
    pub(crate) first: Entity,
    pub(crate) second: Entity,
}

#[derive(Component, Default)]
struct SplitViewDividerDragState {
    dragging: bool,
    origin_first_size: f32,
}

#[derive(Clone)]
pub struct SplitViewBuilder {
    pub split_view: SplitView,
    pub width: Val,
    pub height: Val,
}

impl Default for SplitViewBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SplitViewBuilder {
    pub fn new() -> Self {
        Self {
            split_view: SplitView::default(),
            width: Val::Auto,
            height: Val::Auto,
        }
    }

    pub fn axis(mut self, axis: SplitViewAxis) -> Self {
        self.split_view.axis = axis;
        self
    }

    pub fn default_first_size(mut self, size: Val) -> Self {
        self.split_view.default_first_size = size;
        self
    }

    pub fn divider_size(mut self, size: Val) -> Self {
        self.split_view.divider_size = size;
        self
    }

    pub fn min_first_size(mut self, size: Val) -> Self {
        self.split_view.min_first_size = size;
        self
    }

    pub fn min_second_size(mut self, size: Val) -> Self {
        self.split_view.min_second_size = size;
        self
    }

    pub fn draggable(mut self, draggable: bool) -> Self {
        self.split_view.draggable = draggable;
        self
    }

    pub fn width(mut self, width: Val) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Val) -> Self {
        self.height = height;
        self
    }

    pub fn build<F: Bundle, S: Bundle>(
        self,
        commands: &mut Commands,
        first: F,
        second: S,
        theme: Option<&Theme>,
    ) -> Entity {
        let first_entity = commands.spawn((Name::new("Split First"), first)).id();
        let second_entity = commands.spawn((Name::new("Split Second"), second)).id();
        self.build_with_entities(commands, first_entity, second_entity, theme)
            .root
    }

    pub fn build_with_entities(
        self,
        commands: &mut Commands,
        first_entity: Entity,
        second_entity: Entity,
        theme: Option<&Theme>,
    ) -> WidgetSpawnResult {
        let split_view = self.split_view;
        apply_panel_layout(commands, first_entity, second_entity, &split_view);

        let divider = DividerBuilder::new()
            .axis(split_view.axis.divider_axis())
            .thickness(split_view.divider_size)
            .build(theme);

        let divider_entity = commands
            .spawn((divider, SplitViewDividerDragState::default()))
            .observe(set_cursor_on_over)
            .observe(reset_cursor_on_out)
            .observe(start_split_divider_drag)
            .observe(drag_split_divider)
            .observe(end_split_divider_drag)
            .observe(cancel_split_divider_drag)
            .id();

        let root = commands
            .spawn((
                Node {
                    width: self.width,
                    height: self.height,
                    flex_direction: split_view.axis.flex_direction(),
                    flex_grow: 1.0,
                    ..default()
                },
                split_view,
                SplitViewPanels {
                    first: first_entity,
                    second: second_entity,
                },
            ))
            .add_children(&[first_entity, divider_entity, second_entity])
            .id();

        WidgetSpawnResult::new(root)
            .with_slot("first", first_entity)
            .with_slot("second", second_entity)
    }
}

impl DefaultWidgetBuilder for SplitViewBuilder {
    fn spawn_default(
        commands: &mut Commands,
        theme: Option<&crate::core::theme::Theme>,
    ) -> WidgetSpawnResult {
        let first = commands
            .spawn((
                Node::default(),
                BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.7)),
            ))
            .id();
        let second = commands
            .spawn((
                Node::default(),
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
            ))
            .id();
        SplitViewBuilder::new()
            .width(px(260.0))
            .height(px(120.0))
            .build_with_entities(commands, first, second, theme)
    }
}

fn apply_panel_layout(
    commands: &mut Commands,
    first_panel: Entity,
    second_panel: Entity,
    split_view: &SplitView,
) {
    let default_first_size = split_view.default_first_size;
    let min_first_size = split_view.min_first_size;
    let min_second_size = split_view.min_second_size;

    match split_view.axis {
        SplitViewAxis::Horizontal => {
            commands
                .entity(first_panel)
                .entry::<Node>()
                .and_modify(move |mut node| {
                    node.width = default_first_size;
                    node.height = Val::Percent(100.0);
                    node.min_width = min_first_size;
                    node.min_height = Val::Auto;
                    node.flex_grow = 0.0;
                    node.flex_shrink = 0.0;
                });
            commands
                .entity(first_panel)
                .entry::<Node>()
                .or_insert(Node {
                    width: default_first_size,
                    height: Val::Percent(100.0),
                    min_width: min_first_size,
                    min_height: Val::Auto,
                    flex_grow: 0.0,
                    flex_shrink: 0.0,
                    ..default()
                });

            commands
                .entity(second_panel)
                .entry::<Node>()
                .and_modify(move |mut node| {
                    node.width = Val::Auto;
                    node.height = Val::Percent(100.0);
                    node.min_width = min_second_size;
                    node.min_height = Val::Auto;
                    node.flex_basis = Val::Auto;
                    node.flex_grow = 1.0;
                    node.flex_shrink = 1.0;
                });
            commands
                .entity(second_panel)
                .entry::<Node>()
                .or_insert(Node {
                    width: Val::Auto,
                    height: Val::Percent(100.0),
                    min_width: min_second_size,
                    min_height: Val::Auto,
                    flex_basis: Val::Auto,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    ..default()
                });
        }
        SplitViewAxis::Vertical => {
            commands
                .entity(first_panel)
                .entry::<Node>()
                .and_modify(move |mut node| {
                    node.width = Val::Percent(100.0);
                    node.height = default_first_size;
                    node.min_width = Val::Auto;
                    node.min_height = min_first_size;
                    node.flex_grow = 0.0;
                    node.flex_shrink = 0.0;
                });
            commands
                .entity(first_panel)
                .entry::<Node>()
                .or_insert(Node {
                    width: Val::Percent(100.0),
                    height: default_first_size,
                    min_width: Val::Auto,
                    min_height: min_first_size,
                    flex_grow: 0.0,
                    flex_shrink: 0.0,
                    ..default()
                });

            commands
                .entity(second_panel)
                .entry::<Node>()
                .and_modify(move |mut node| {
                    node.width = Val::Percent(100.0);
                    node.height = Val::Auto;
                    node.min_width = Val::Auto;
                    node.min_height = min_second_size;
                    node.flex_basis = Val::Auto;
                    node.flex_grow = 1.0;
                    node.flex_shrink = 1.0;
                });
            commands
                .entity(second_panel)
                .entry::<Node>()
                .or_insert(Node {
                    width: Val::Percent(100.0),
                    height: Val::Auto,
                    min_width: Val::Auto,
                    min_height: min_second_size,
                    flex_basis: Val::Auto,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    ..default()
                });
        }
    }
}

fn set_cursor_on_over(
    event: On<Pointer<Over>>,
    mut commands: Commands,
    window: Single<Entity, With<PrimaryWindow>>,
    mut dividers: Query<(&ChildOf, &Divider, &mut BackgroundColor)>,
    split_views: Query<&SplitView>,
    parents: Query<&ChildOf>,
    scopes: Query<&ThemeScope>,
    boundaries: Query<(), With<ThemeBoundary>>,
    global_theme: Option<Res<Theme>>,
) {
    if let Ok((child_of, divider, mut bg_color)) = dividers.get_mut(event.event_target())
        && let Ok(split_view) = split_views.get(child_of.parent())
    {
        let theme = resolve_theme_or_global(
            event.event_target(),
            &parents,
            &scopes,
            &boundaries,
            global_theme.as_deref(),
        );
        commands
            .entity(*window)
            .insert(split_view.axis.divider_cursor());
        bg_color.0 = resolve_divider_hover_color(theme, divider);
    }
}

fn reset_cursor_on_out(
    event: On<Pointer<Out>>,
    mut commands: Commands,
    window: Single<Entity, With<PrimaryWindow>>,
    mut dividers: Query<(&Divider, &mut BackgroundColor)>,
    parents: Query<&ChildOf>,
    scopes: Query<&ThemeScope>,
    boundaries: Query<(), With<ThemeBoundary>>,
    global_theme: Option<Res<Theme>>,
) {
    commands.entity(*window).remove::<CursorIcon>();
    if let Ok((divider, mut bg_color)) = dividers.get_mut(event.event_target()) {
        let theme = resolve_theme_or_global(
            event.event_target(),
            &parents,
            &scopes,
            &boundaries,
            global_theme.as_deref(),
        );
        bg_color.0 = resolve_divider_color(theme, divider);
    }
}

fn start_split_divider_drag(
    mut event: On<Pointer<DragStart>>,
    mut dividers: Query<(&ChildOf, &mut SplitViewDividerDragState), With<Divider>>,
    split_views: Query<(&SplitView, &SplitViewPanels, &ComputedNode)>,
    panel_nodes: Query<&Node>,
    panel_computed: Query<&ComputedNode>,
) {
    let Ok((child_of, mut drag_state)) = dividers.get_mut(event.event_target()) else {
        return;
    };

    let split_view_entity = child_of.parent();
    let Ok((split_view, panels, split_computed)) = split_views.get(split_view_entity) else {
        return;
    };
    if !split_view.draggable {
        return;
    }

    let split_size = split_computed.size() * split_computed.inverse_scale_factor();
    let full_axis_size = match split_view.axis {
        SplitViewAxis::Horizontal => split_size.x,
        SplitViewAxis::Vertical => split_size.y,
    };
    let divider_px = resolve_val_to_px(split_view.divider_size, full_axis_size, 2.0);
    let available = (full_axis_size - divider_px).max(0.0);

    let from_node =
        panel_nodes
            .get(panels.first)
            .ok()
            .and_then(|first_node| match split_view.axis {
                SplitViewAxis::Horizontal => match first_node.width {
                    Val::Px(px) => Some(px),
                    _ => None,
                },
                SplitViewAxis::Vertical => match first_node.height {
                    Val::Px(px) => Some(px),
                    _ => None,
                },
            });

    drag_state.origin_first_size = from_node
        .or_else(|| {
            current_first_size_from_computed(split_view.axis, panels.first, &panel_computed)
        })
        .unwrap_or_else(|| {
            resolve_val_to_px(split_view.default_first_size, available, available * 0.3)
        })
        .max(0.0);
    drag_state.dragging = true;
    event.propagate(false);
}

fn drag_split_divider(
    mut event: On<Pointer<Drag>>,
    dividers: Query<(&ChildOf, &SplitViewDividerDragState), With<Divider>>,
    split_views: Query<(&SplitView, &SplitViewPanels, &ComputedNode)>,
    mut panel_nodes: Query<&mut Node>,
) {
    let Ok((child_of, drag_state)) = dividers.get(event.event_target()) else {
        return;
    };
    if !drag_state.dragging {
        return;
    }

    let split_view_entity = child_of.parent();
    let Ok((split_view, panels, split_computed)) = split_views.get(split_view_entity) else {
        return;
    };
    if !split_view.draggable {
        return;
    }

    let split_size = split_computed.size() * split_computed.inverse_scale_factor();
    let full_axis_size = match split_view.axis {
        SplitViewAxis::Horizontal => split_size.x,
        SplitViewAxis::Vertical => split_size.y,
    };
    if full_axis_size <= 0.0 {
        return;
    }

    let divider_px = resolve_val_to_px(split_view.divider_size, full_axis_size, 2.0);
    let available = (full_axis_size - divider_px).max(0.0);
    if available <= 1.0 {
        return;
    }

    let delta = match split_view.axis {
        SplitViewAxis::Horizontal => event.event().distance.x,
        SplitViewAxis::Vertical => event.event().distance.y,
    };

    let min_first = resolve_val_to_px(split_view.min_first_size, available, 0.0).max(0.0);
    let min_second = resolve_val_to_px(split_view.min_second_size, available, 0.0).max(0.0);
    let max_first = (available - min_second).max(min_first);
    let next_first = (drag_state.origin_first_size + delta).clamp(min_first, max_first);

    if let Ok(mut first_node) = panel_nodes.get_mut(panels.first) {
        match split_view.axis {
            SplitViewAxis::Horizontal => first_node.width = Val::Px(next_first),
            SplitViewAxis::Vertical => first_node.height = Val::Px(next_first),
        }
    }
    if let Ok(mut second_node) = panel_nodes.get_mut(panels.second) {
        second_node.flex_basis = Val::Auto;
        second_node.flex_grow = 1.0;
        second_node.flex_shrink = 1.0;
    }

    event.propagate(false);
}

fn end_split_divider_drag(
    mut event: On<Pointer<DragEnd>>,
    mut dividers: Query<&mut SplitViewDividerDragState, With<Divider>>,
) {
    if let Ok(mut state) = dividers.get_mut(event.event_target()) {
        state.dragging = false;
        event.propagate(false);
    }
}

fn cancel_split_divider_drag(
    mut event: On<Pointer<Cancel>>,
    mut dividers: Query<&mut SplitViewDividerDragState, With<Divider>>,
) {
    if let Ok(mut state) = dividers.get_mut(event.event_target()) {
        state.dragging = false;
        event.propagate(false);
    }
}

fn current_first_size_from_computed(
    axis: SplitViewAxis,
    first_panel: Entity,
    computed: &Query<&ComputedNode>,
) -> Option<f32> {
    let computed = computed.get(first_panel).ok()?;
    let size = computed.size() * computed.inverse_scale_factor();
    Some(match axis {
        SplitViewAxis::Horizontal => size.x.max(0.0),
        SplitViewAxis::Vertical => size.y.max(0.0),
    })
}

fn resolve_val_to_px(val: Val, axis_size: f32, fallback: f32) -> f32 {
    match val {
        Val::Px(px) => px,
        Val::Percent(percent) => axis_size * percent * 0.01,
        _ => fallback,
    }
}
