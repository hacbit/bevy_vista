use bevy::{input::mouse::{MouseScrollUnit, MouseWheel}, picking::hover::{HoverMap, Hovered}};

use super::*;

/// Plugin that adds the observers for the [`ScrollView`] widget.
pub struct ScrollViewPlugin;

impl Plugin for ScrollViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(scrollbar_on_pointer_down)
            .add_observer(scrollbar_on_drag_start)
            .add_observer(scrollbar_on_drag_end)
            .add_observer(scrollbar_on_drag_cancel)
            .add_observer(scrollbar_on_drag)
            .add_systems(
                PostUpdate,
                (
                    update_scrollbar_visibility,
                    update_scrollbar_thumb,
                    update_scrollbar_thumb_color,
                ),
            )
            // use mouse wheel control
            .init_resource::<ScrollOptions>()
            .add_systems(Update, send_scroll_events.run_if(is_enable_mouse_wheel))
            .add_observer(on_scroll_handler)
            // auto init
            .add_systems(
                Update,
                (
                    detect_scrollbar_added,
                    auto_initialize_scrollbar.after(detect_scrollbar_added),
                ),
            );
    }
}

#[derive(Resource)]
pub struct ScrollOptions {
    /// Enable that you can use mouse wheel to control items
    pub enable_mouse_wheel: bool,
}

impl Default for ScrollOptions {
    fn default() -> Self {
        Self {
            enable_mouse_wheel: true,
        }
    }
}

fn is_enable_mouse_wheel(options: Res<ScrollOptions>) -> bool {
    options.enable_mouse_wheel
}

#[derive(Component, Reflect, Clone, Widget, ShowInInspector)]
#[widget("layout/scroll_view", children = "any", slots = "content")]
#[builder(ScrollViewBuilder)]
pub struct ScrollView {
    #[property(label = "Horizontal Bar")]
    pub horizontal_bar: ScrollbarVisibility,
    #[property(label = "Vertical Bar")]
    pub vertical_bar: ScrollbarVisibility,
    #[property(label = "Min Thumb Length", min = 1.0)]
    pub min_thumb_length: f32,
    #[property(label = "Content Padding")]
    pub content_padding: UiRect,
    #[property(label = "Scroll Area Background")]
    pub scroll_area_bg: Option<Color>,
}

impl Default for ScrollView {
    fn default() -> Self {
        Self {
            horizontal_bar: ScrollbarVisibility::Show,
            vertical_bar: ScrollbarVisibility::Show,
            min_thumb_length: 8.0,
            content_padding: UiRect::all(px(4)),
            scroll_area_bg: None,
        }
    }
}

#[derive(Debug, Default, Reflect, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarVisibility {
    Show,
    Hide,
    #[default]
    Auto,
}

#[derive(Clone)]
pub struct ScrollViewBuilder {
    scroll_view: ScrollView,
    width: Val,
    height: Val,
}

impl Default for ScrollViewBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollViewBuilder {
    pub fn new() -> Self {
        Self {
            scroll_view: ScrollView::default(),
            width: Val::Percent(100.),
            height: Val::Percent(100.),
        }
    }

    pub fn width(mut self, width: Val) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Val) -> Self {
        self.height = height;
        self
    }

    pub fn show_horizontal(mut self, show: bool) -> Self {
        self.scroll_view.horizontal_bar = if show {
            ScrollbarVisibility::Show
        } else {
            ScrollbarVisibility::Hide
        };
        self
    }

    pub fn show_vertical(mut self, show: bool) -> Self {
        self.scroll_view.vertical_bar = if show {
            ScrollbarVisibility::Show
        } else {
            ScrollbarVisibility::Hide
        };
        self
    }

    pub fn horizontal_bar(mut self, visibility: ScrollbarVisibility) -> Self {
        self.scroll_view.horizontal_bar = visibility;
        self
    }

    pub fn vertical_bar(mut self, visibility: ScrollbarVisibility) -> Self {
        self.scroll_view.vertical_bar = visibility;
        self
    }

    pub fn min_thumb_length(mut self, min: f32) -> Self {
        self.scroll_view.min_thumb_length = min.max(1.0);
        self
    }

    pub fn content_padding(mut self, padding: UiRect) -> Self {
        self.scroll_view.content_padding = padding;
        self
    }

    pub fn scroll_area_bg(mut self, bg: Color) -> Self {
        self.scroll_view.scroll_area_bg = Some(bg);
        self
    }

    fn spawn_content_root(commands: &mut Commands) -> Entity {
        commands
            .spawn((
                Name::new("Scroll View Content"),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Auto,
                    min_width: Val::Px(0.0),
                    min_height: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ))
            .id()
    }

    pub fn build_with_entities(
        self,
        commands: &mut Commands,
        children: impl IntoIterator<Item = Entity>,
    ) -> Entity {
        let root = commands
            .spawn((
                Node {
                    width: self.width,
                    height: self.height,
                    ..default()
                },
                self.scroll_view,
            ))
            .id();
        let children: Vec<Entity> = children.into_iter().collect();
        commands.entity(root).add_children(&children);
        root
    }
}

impl DefaultWidgetBuilder for ScrollViewBuilder {
    fn spawn_default(
        commands: &mut Commands,
        _theme: Option<&crate::theme::Theme>,
    ) -> WidgetSpawnResult {
        let content = Self::spawn_content_root(commands);
        let root = ScrollViewBuilder::new()
            .width(px(240.0))
            .height(px(140.0))
            .build_with_entities(commands, [content]);
        WidgetSpawnResult::new(root).with_slot("content", content)
    }
}

///
#[derive(Component, Debug, Reflect)]
pub struct Scrollbar {
    /// Entity being scrolled.
    pub target: Entity,
    pub orientation: ControlOrientation,
    pub min_thumb_length: f32,
    pub visibility: ScrollbarVisibility,
}

/// Marker component to indicate that will automatically init scrollbar
#[derive(Component)]
struct ScrollViewPendingInit;

/// Marker component to indicate that the entity is a scrollbar thumb (the moving, draggable part of
/// the scrollbar). This should be a child of the scrollbar entity.
#[derive(Component, Debug, Reflect)]
#[require(CoreScrollbarDragState)]
#[reflect(Component)]
pub struct CoreScrollbarThumb;

/// Component used to manage the state of a scrollbar during dragging.
/// This component is automatically inserted on the thumb entity.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct CoreScrollbarDragState {
    /// Whether the scrollbar is currently being dragged.
    pub dragging: bool,
    /// The value of the scrollbar when dragging started.
    drag_origin: f32,
}

/// Ui scrolling event.
#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct Scroll {
    entity: Entity,
    delta: Vec2,
}

const LINE_HEIGHT: f32 = 21.;

fn send_scroll_events(
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    for mouse_wheel in mouse_wheel_reader.read() {
        let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);

        if mouse_wheel.unit == MouseScrollUnit::Line {
            delta *= LINE_HEIGHT;
        }

        if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            std::mem::swap(&mut delta.x, &mut delta.y);
        }

        for pointer_map in hover_map.values() {
            for entity in pointer_map.keys().copied() {
                commands.trigger(Scroll { entity, delta });
            }
        }
    }
}

fn on_scroll_handler(
    mut scroll: On<Scroll>,
    mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode), With<ScrollArea>>,
) {
    let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
        return;
    };

    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();

    let delta = &mut scroll.delta;
    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0. {
        // Is this node already scrolled all the way in the direction of the scroll?
        let max = if delta.x > 0. {
            scroll_position.x >= max_offset.x
        } else {
            scroll_position.x <= 0.
        };

        if !max {
            scroll_position.x += delta.x;
            // Consume the X portion of the scroll delta.
            delta.x = 0.;
        }
    }

    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
        // Is this node already scrolled all the way in the direction of the scroll?
        let max = if delta.y > 0. {
            scroll_position.y >= max_offset.y
        } else {
            scroll_position.y <= 0.
        };

        if !max {
            scroll_position.y += delta.y;
            // Consume the Y portion of the scroll delta.
            delta.y = 0.;
        }
    }

    // Stop propagating when the delta is fully consumed.
    if *delta == Vec2::ZERO {
        scroll.propagate(false);
    }
}

fn scrollbar_on_pointer_down(
    mut ev: On<Pointer<Press>>,
    q_thumb: Query<&ChildOf, With<CoreScrollbarThumb>>,
    mut q_scrollbar: Query<(
        &Scrollbar,
        &ComputedNode,
        &ComputedUiRenderTargetInfo,
        &UiGlobalTransform,
    )>,
    mut q_scroll_pos: Query<(&mut ScrollPosition, &ComputedNode), Without<Scrollbar>>,
    ui_scale: Res<UiScale>,
) {
    if q_thumb.contains(ev.entity) {
        // If they click on the thumb, do nothing. This will be handled by the drag event.
        ev.propagate(false);
    } else if let Ok((scrollbar, node, node_target, transform)) = q_scrollbar.get_mut(ev.entity) {
        // If they click on the scrollbar track, page up or down.
        ev.propagate(false);

        // Convert to widget-local coordinates.
        let local_pos = transform.try_inverse().unwrap().transform_point2(
            ev.event().pointer_location.position * node_target.scale_factor() / ui_scale.0,
        ) + node.size() * 0.5;

        // Bail if we don't find the target entity.
        let Ok((mut scroll_pos, scroll_content)) = q_scroll_pos.get_mut(scrollbar.target) else {
            return;
        };

        // Convert the click coordinates into a scroll position. If it's greater than the
        // current scroll position, scroll forward by one step (visible size) otherwise scroll
        // back.
        let visible_size = scroll_content.size() * scroll_content.inverse_scale_factor;
        let content_size = scroll_content.content_size() * scroll_content.inverse_scale_factor;
        let max_range = (content_size - visible_size).max(Vec2::ZERO);

        fn adjust_scroll_pos(scroll_pos: &mut f32, click_pos: f32, step: f32, range: f32) {
            *scroll_pos =
                (*scroll_pos + if click_pos > *scroll_pos { step } else { -step }).clamp(0., range);
        }

        match scrollbar.orientation {
            ControlOrientation::Horizontal => {
                if node.size().x > 0. {
                    let click_pos = local_pos.x * content_size.x / node.size().x;
                    adjust_scroll_pos(&mut scroll_pos.x, click_pos, visible_size.x, max_range.x);
                }
            }
            ControlOrientation::Vertical => {
                if node.size().y > 0. {
                    let click_pos = local_pos.y * content_size.y / node.size().y;
                    adjust_scroll_pos(&mut scroll_pos.y, click_pos, visible_size.y, max_range.y);
                }
            }
        }
    }
}

fn scrollbar_on_drag_start(
    mut ev: On<Pointer<DragStart>>,
    mut q_thumb: Query<(&ChildOf, &mut CoreScrollbarDragState), With<CoreScrollbarThumb>>,
    q_scrollbar: Query<&Scrollbar>,
    q_scroll_area: Query<&ScrollPosition>,
) {
    if let Ok((ChildOf(thumb_parent), mut drag)) = q_thumb.get_mut(ev.entity) {
        ev.propagate(false);
        if let Ok(scrollbar) = q_scrollbar.get(*thumb_parent)
            && let Ok(scroll_area) = q_scroll_area.get(scrollbar.target)
        {
            drag.dragging = true;
            drag.drag_origin = match scrollbar.orientation {
                ControlOrientation::Horizontal => scroll_area.x,
                ControlOrientation::Vertical => scroll_area.y,
            };
        }
    }
}

fn scrollbar_on_drag(
    mut ev: On<Pointer<Drag>>,
    mut q_thumb: Query<(&ChildOf, &mut CoreScrollbarDragState), With<CoreScrollbarThumb>>,
    mut q_scrollbar: Query<(&ComputedNode, &Scrollbar)>,
    mut q_scroll_pos: Query<(&mut ScrollPosition, &ComputedNode), Without<Scrollbar>>,
    ui_scale: Res<UiScale>,
) {
    if let Ok((ChildOf(thumb_parent), drag)) = q_thumb.get_mut(ev.entity)
        && let Ok((node, scrollbar)) = q_scrollbar.get_mut(*thumb_parent)
    {
        ev.propagate(false);
        let Ok((mut scroll_pos, scroll_content)) = q_scroll_pos.get_mut(scrollbar.target) else {
            return;
        };

        if drag.dragging {
            let distance = ev.event().distance / ui_scale.0;
            let visible_size = scroll_content.size() * scroll_content.inverse_scale_factor;
            let content_size = scroll_content.content_size() * scroll_content.inverse_scale_factor;
            let scrollbar_size = (node.size() * node.inverse_scale_factor).max(Vec2::ONE);

            match scrollbar.orientation {
                ControlOrientation::Horizontal => {
                    let range = (content_size.x - visible_size.x).max(0.);
                    scroll_pos.x = (drag.drag_origin
                        + (distance.x * content_size.x) / scrollbar_size.x)
                        .clamp(0., range);
                }
                ControlOrientation::Vertical => {
                    let range = (content_size.y - visible_size.y).max(0.);
                    scroll_pos.y = (drag.drag_origin
                        + (distance.y * content_size.y) / scrollbar_size.y)
                        .clamp(0., range);
                }
            };
        }
    }
}

fn scrollbar_on_drag_end(
    mut ev: On<Pointer<DragEnd>>,
    mut q_thumb: Query<&mut CoreScrollbarDragState, With<CoreScrollbarThumb>>,
) {
    if let Ok(mut drag) = q_thumb.get_mut(ev.entity) {
        ev.propagate(false);
        if drag.dragging {
            drag.dragging = false;
        }
    }
}

fn scrollbar_on_drag_cancel(
    mut ev: On<Pointer<Cancel>>,
    mut q_thumb: Query<&mut CoreScrollbarDragState, With<CoreScrollbarThumb>>,
) {
    if let Ok(mut drag) = q_thumb.get_mut(ev.entity) {
        ev.propagate(false);
        if drag.dragging {
            drag.dragging = false;
        }
    }
}

fn update_scrollbar_thumb(
    q_scroll_area: Query<(&ScrollPosition, &ComputedNode)>,
    q_scrollbar: Query<(&Scrollbar, &ComputedNode, &Children)>,
    mut q_thumb: Query<&mut Node, With<CoreScrollbarThumb>>,
) {
    for (scrollbar, scrollbar_node, children) in q_scrollbar.iter() {
        let Ok(scroll_area) = q_scroll_area.get(scrollbar.target) else {
            continue;
        };

        // Size of the visible scrolling area.
        let visible_size = scroll_area.1.size() * scroll_area.1.inverse_scale_factor;

        // Size of the scrolling content.
        let content_size = scroll_area.1.content_size() * scroll_area.1.inverse_scale_factor;

        // Length of the scrollbar track.
        let track_length = scrollbar_node.size() * scrollbar_node.inverse_scale_factor;

        fn size_and_pos(
            content_size: f32,
            visible_size: f32,
            track_length: f32,
            min_size: f32,
            offset: f32,
        ) -> (f32, f32) {
            let thumb_size = if content_size > visible_size {
                (track_length * visible_size / content_size)
                    .max(min_size)
                    .min(track_length)
            } else {
                track_length
            };

            let thumb_pos = if content_size > visible_size {
                offset * (track_length - thumb_size) / (content_size - visible_size)
            } else {
                0.
            };

            (thumb_size, thumb_pos)
        }

        for child in children {
            if let Ok(mut thumb) = q_thumb.get_mut(*child) {
                match scrollbar.orientation {
                    ControlOrientation::Horizontal => {
                        let (thumb_size, thumb_pos) = size_and_pos(
                            content_size.x,
                            visible_size.x,
                            track_length.x,
                            scrollbar.min_thumb_length,
                            scroll_area.0.x,
                        );

                        thumb.top = Val::Px(0.);
                        thumb.bottom = Val::Px(0.);
                        thumb.left = Val::Px(thumb_pos);
                        thumb.width = Val::Px(thumb_size);
                    }
                    ControlOrientation::Vertical => {
                        let (thumb_size, thumb_pos) = size_and_pos(
                            content_size.y,
                            visible_size.y,
                            track_length.y,
                            scrollbar.min_thumb_length,
                            scroll_area.0.y,
                        );

                        thumb.left = Val::Px(0.);
                        thumb.right = Val::Px(0.);
                        thumb.top = Val::Px(thumb_pos);
                        thumb.height = Val::Px(thumb_size);
                    }
                };
            }
        }
    }
}

fn update_scrollbar_visibility(
    mut q_scrollbar: Query<(&mut Node, &Scrollbar)>,
    q_scroll_area: Query<&ComputedNode>,
) {
    for (mut bar_node, scrollbar) in q_scrollbar.iter_mut() {
        let display = match scrollbar.visibility {
            ScrollbarVisibility::Show => Display::Flex,
            ScrollbarVisibility::Hide => Display::None,
            ScrollbarVisibility::Auto => {
                let Ok(area) = q_scroll_area.get(scrollbar.target) else {
                    continue;
                };
                let visible = area.size() * area.inverse_scale_factor;
                let content = area.content_size() * area.inverse_scale_factor;
                let need_scroll = match scrollbar.orientation {
                    ControlOrientation::Horizontal => content.x > visible.x + 0.5,
                    ControlOrientation::Vertical => content.y > visible.y + 0.5,
                };
                if need_scroll {
                    Display::Flex
                } else {
                    Display::None
                }
            }
        };

        if bar_node.display != display {
            bar_node.display = display;
        }
    }
}

const SCROLLBAR_THUMB_COLOR: Color = Color::srgb(0.486, 0.486, 0.529);

fn update_scrollbar_thumb_color(
    mut q_thumb: Query<
        (&mut BackgroundColor, &Hovered, &CoreScrollbarDragState),
        (
            With<CoreScrollbarThumb>,
            Or<(Changed<Hovered>, Changed<CoreScrollbarDragState>)>,
        ),
    >,
) {
    for (mut thumb_bg, Hovered(is_hovering), drag) in q_thumb.iter_mut() {
        let color: Color = if *is_hovering || drag.dragging {
            // If hovering, use a lighter color
            SCROLLBAR_THUMB_COLOR.lighter(0.3)
        } else {
            // Default color for the slider
            SCROLLBAR_THUMB_COLOR
        }
        .into();

        if thumb_bg.0 != color {
            // Update the color of the thumb
            thumb_bg.0 = color;
        }
    }
}

fn detect_scrollbar_added(
    mut commands: Commands,
    query: Query<(Entity, &ScrollView), (With<Node>, Added<ScrollView>)>,
) {
    for (entity, scroll) in query {
        commands.entity(entity).insert(ScrollViewPendingInit);
        if scroll.min_thumb_length <= 0.0 {
            commands.entity(entity).insert(ScrollView {
                min_thumb_length: 8.0,
                ..scroll.clone()
            });
        }
    }
}

const SCROLL_AREA_BG_COLOR: Color = Color::srgb(0.224, 0.224, 0.243);

/// Marker component (automatically added)
#[derive(Component)]
pub struct ScrollArea;

fn auto_initialize_scrollbar(
    mut commands: Commands,
    query: Query<(Entity, &mut Node, &Children, &ScrollView), With<ScrollViewPendingInit>>,
) {
    for (entity, mut node, children, scroll_view) in query {
        // spawn scroll area container
        let scroll_area_id = commands
            .spawn((
                Name::new("Scroll Area"),
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: scroll_view.content_padding,
                    overflow: Overflow::scroll(),
                    ..default()
                },
                BackgroundColor(scroll_view.scroll_area_bg.unwrap_or(SCROLL_AREA_BG_COLOR)),
                ScrollArea,
            ))
            .id();

        node.display = Display::Grid;
        node.grid_template_columns =
            vec![RepeatedGridTrack::flex(1, 1.), RepeatedGridTrack::auto(1)];
        node.grid_template_rows = vec![RepeatedGridTrack::flex(1, 1.), RepeatedGridTrack::auto(1)];
        node.row_gap = px(2);
        node.column_gap = px(2);

        commands
            .entity(entity)
            .clear_children()
            .add_child(scroll_area_id)
            .with_children(|parent| {
                // vertical scrollbar
                parent.spawn((
                    Node {
                        min_width: px(8),
                        grid_row: GridPlacement::start(1),
                        grid_column: GridPlacement::start(2),
                        display: if matches!(scroll_view.vertical_bar, ScrollbarVisibility::Hide) {
                            Display::None
                        } else {
                            Display::Flex
                        },
                        ..default()
                    },
                    Scrollbar {
                        orientation: ControlOrientation::Vertical,
                        target: scroll_area_id,
                        min_thumb_length: scroll_view.min_thumb_length,
                        visibility: scroll_view.vertical_bar,
                    },
                    Children::spawn(Spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        BorderRadius::all(px(4)),
                        Hovered::default(),
                        BackgroundColor(SCROLLBAR_THUMB_COLOR),
                        CoreScrollbarThumb,
                    ))),
                ));

                // Horizontal scrollbar
                parent.spawn((
                    Node {
                        min_height: px(8),
                        grid_row: GridPlacement::start(2),
                        grid_column: GridPlacement::start(1),
                        display: if matches!(scroll_view.horizontal_bar, ScrollbarVisibility::Hide)
                        {
                            Display::None
                        } else {
                            Display::Flex
                        },
                        ..default()
                    },
                    Scrollbar {
                        orientation: ControlOrientation::Horizontal,
                        target: scroll_area_id,
                        min_thumb_length: scroll_view.min_thumb_length,
                        visibility: scroll_view.horizontal_bar,
                    },
                    Children::spawn(Spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        BorderRadius::all(px(4)),
                        Hovered::default(),
                        BackgroundColor(SCROLLBAR_THUMB_COLOR),
                        CoreScrollbarThumb,
                    ))),
                ));
            });

        commands.entity(scroll_area_id).add_children(children);

        commands.entity(entity).remove::<ScrollViewPendingInit>();
    }
}
