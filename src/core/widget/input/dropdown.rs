use bevy::picking::hover::Hovered;

use crate::core::theme::resolve_theme_or_global;

use super::*;

pub struct DropdownPlugin;

const DROPDOWN_OPTION_HEIGHT: f32 = 24.0;
const DROPDOWN_MENU_PADDING_Y: f32 = 8.0;
const DROPDOWN_MIN_MENU_HEIGHT: f32 = 48.0;
const DROPDOWN_EDGE_MARGIN: f32 = 8.0;
const DROPDOWN_SCROLL_STEP: f32 = 24.0;
const DROPDOWN_HORIZONTAL_PADDING: f32 = 8.0;
const DROPDOWN_CARET_WIDTH: f32 = 14.0;
const DROPDOWN_TEXT_ESTIMATE_FACTOR: f32 = 0.58;

impl Plugin for DropdownPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DropdownChange>()
            .add_systems(Update, scroll_hovered_dropdown_menus)
            .add_systems(
                PostUpdate,
                (
                    close_dropdowns_on_outside_click,
                    sync_dropdown_popup_presence,
                    sync_dropdown_visuals,
                    sync_dropdown_popup_layout,
                    sync_dropdown_interaction,
                ),
            );
    }
}

#[derive(Component, Reflect, Clone, Widget, ShowInInspector)]
#[widget("input/dropdown", children = "exact(0)")]
#[builder(DropdownBuilder)]
pub struct Dropdown {
    #[property(hidden)]
    pub options: Vec<String>,
    #[property(hidden)]
    pub selected: usize,
    #[property(hidden)]
    pub expanded: bool,
    #[property(label = "Disabled")]
    pub disabled: bool,
    #[property(label = "Max Popup Width", min = 120.0)]
    pub max_popup_width: f32,
    #[property(label = "Font Size", min = 1.0)]
    pub font_size: f32,
    #[property(hidden)]
    pub max_visible_items: usize,
}

impl Default for Dropdown {
    fn default() -> Self {
        Self {
            options: vec!["Option A".to_owned(), "Option B".to_owned()],
            selected: 0,
            expanded: false,
            disabled: false,
            max_visible_items: 8,
            max_popup_width: 320.0,
            font_size: 13.0,
        }
    }
}

#[derive(Component)]
struct DropdownHeaderLabel;

#[derive(Component)]
struct DropdownHeader;

#[derive(Component)]
struct DropdownMenu;

#[derive(Component)]
struct DropdownCaret;

#[derive(Component, Copy, Clone)]
struct DropdownPopup;

#[derive(Component)]
struct DropdownOption {
    index: usize,
}

#[derive(Component, Copy, Clone)]
struct DropdownOwnedBy(Entity);

#[derive(Component, Clone)]
struct DropdownParts {
    header: Entity,
    header_label: Entity,
    caret: Entity,
    popup: Option<Entity>,
}

#[derive(Component, Copy, Clone)]
struct DropdownColors {
    normal_bg: Color,
    hovered_bg: Color,
    pressed_bg: Color,
    border: Color,
    menu_bg: Color,
    text: Color,
    selected_bg: Color,
    disabled_bg: Color,
}

#[derive(Message, EntityEvent)]
pub struct DropdownChange {
    pub entity: Entity,
    pub selected: usize,
    pub value: String,
}

#[derive(Clone)]
pub struct DropdownBuilder {
    dropdown: Dropdown,
    width: Val,
}

impl Default for DropdownBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DropdownBuilder {
    pub fn new() -> Self {
        Self {
            dropdown: Dropdown::default(),
            width: px(160.0),
        }
    }

    pub fn options(mut self, options: Vec<String>) -> Self {
        self.dropdown.options = options;
        self.dropdown.selected = 0;
        self
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.dropdown.selected = selected;
        self
    }

    pub fn width(mut self, width: Val) -> Self {
        self.width = width;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.dropdown.disabled = disabled;
        self
    }

    pub fn max_visible_items(mut self, max_visible_items: usize) -> Self {
        self.dropdown.max_visible_items = max_visible_items.max(1);
        self
    }

    pub fn max_popup_width(mut self, max_popup_width: f32) -> Self {
        self.dropdown.max_popup_width = max_popup_width.max(120.0);
        self
    }

    pub fn build(self, commands: &mut Commands, theme: Option<&Theme>) -> Entity {
        let mut dropdown = self.dropdown;
        let colors = dropdown_colors(theme);
        let font = theme
            .map(|t| t.typography.body_medium.font.clone())
            .unwrap_or(TextFont::from_font_size(13.0));
        dropdown.font_size = font.font_size;
        let selected_label = dropdown
            .options
            .get(dropdown.selected)
            .cloned()
            .unwrap_or_else(|| "Select".to_owned());

        let root = commands
            .spawn((
                Node {
                    width: self.width,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                dropdown.clone(),
                colors,
            ))
            .id();

        let header_label = commands
            .spawn((
                Name::new("Dropdown Header Label"),
                Node {
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                DropdownHeaderLabel,
                DropdownOwnedBy(root),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(selected_label),
                    TextLayout::new_with_no_wrap(),
                    font.clone(),
                    TextColor(colors.text),
                ));
            })
            .id();

        let caret = commands
            .spawn((
                Name::new("Dropdown Caret"),
                Node {
                    width: px(14.0),
                    height: px(14.0),
                    ..default()
                },
                DropdownCaret,
                DropdownOwnedBy(root),
                Icons::ArrowRight,
                UiTransform::from_rotation(Rot2::IDENTITY),
            ))
            .id();

        let header = commands
            .spawn((
                Name::new("Dropdown Header"),
                Button,
                Interaction::default(),
                Node {
                    width: percent(100.0),
                    min_height: px(28.0),
                    padding: UiRect::axes(px(8.0), px(4.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(colors.normal_bg),
                BorderColor::all(colors.border),
                BorderRadius::all(px(4.0)),
                colors,
                DropdownHeader,
                DropdownOwnedBy(root),
            ))
            .add_children(&[header_label, caret])
            .observe(on_dropdown_header_click)
            .id();

        commands.entity(root).insert(DropdownParts {
            header,
            header_label,
            caret,
            popup: None,
        });
        commands.entity(root).add_child(header);
        root
    }
}

impl DefaultWidgetBuilder for DropdownBuilder {
    fn spawn_default(
        commands: &mut Commands,
        theme: Option<&crate::core::theme::Theme>,
    ) -> WidgetSpawnResult {
        DropdownBuilder::new().build(commands, theme).into()
    }
}

fn on_dropdown_header_click(
    mut event: On<Pointer<Click>>,
    owners: Query<&DropdownOwnedBy>,
    mut dropdowns: Query<&mut Dropdown>,
) {
    let Ok(owner) = owners.get(event.entity) else {
        return;
    };
    let Ok(mut dropdown) = dropdowns.get_mut(owner.0) else {
        return;
    };
    if dropdown.disabled {
        return;
    }
    dropdown.expanded = !dropdown.expanded;
    event.propagate(false);
}

fn on_dropdown_option_click(
    mut event: On<Pointer<Click>>,
    options: Query<(&DropdownOption, &DropdownOwnedBy)>,
    mut dropdowns: Query<&mut Dropdown>,
    mut out: MessageWriter<DropdownChange>,
) {
    let Ok((option, owner)) = options.get(event.entity) else {
        return;
    };
    let Ok(mut dropdown) = dropdowns.get_mut(owner.0) else {
        return;
    };
    if dropdown.disabled || option.index >= dropdown.options.len() {
        return;
    }
    dropdown.selected = option.index;
    dropdown.expanded = false;
    out.write(DropdownChange {
        entity: owner.0,
        selected: option.index,
        value: dropdown.options[option.index].clone(),
    });
    event.propagate(false);
}

fn sync_dropdown_visuals(
    dropdowns: Query<(&Dropdown, &DropdownParts, &DropdownColors), Changed<Dropdown>>,
    menu_children: Query<&Children, With<DropdownMenu>>,
    options: Query<&DropdownOption>,
    children_query: Query<&Children>,
    mut text_query: Query<&mut Text>,
    mut transforms: Query<&mut UiTransform>,
    mut backgrounds: Query<&mut BackgroundColor>,
) {
    for (dropdown, parts, colors) in dropdowns.iter() {
        if let Some(text_entity) =
            first_text_descendant(parts.header_label, &children_query, &text_query)
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.0 = dropdown
                .options
                .get(dropdown.selected)
                .cloned()
                .unwrap_or_else(|| "Select".to_owned());
        }

        if let Ok(mut bg) = backgrounds.get_mut(parts.header) {
            bg.0 = if dropdown.disabled {
                colors.disabled_bg
            } else {
                colors.normal_bg
            };
        }

        if let Some(menu) = parts.popup
            && let Ok(children) = menu_children.get(menu)
        {
            for child in children.iter() {
                let Ok(option) = options.get(child) else {
                    continue;
                };
                if let Ok(mut bg) = backgrounds.get_mut(child) {
                    bg.0 = if dropdown.disabled {
                        colors.disabled_bg
                    } else if option.index == dropdown.selected {
                        colors.selected_bg
                    } else {
                        colors.menu_bg
                    };
                }
            }
        }

        if let Ok(mut transform) = transforms.get_mut(parts.caret) {
            transform.rotation = if dropdown.expanded {
                Rot2::FRAC_PI_2
            } else {
                Rot2::IDENTITY
            };
        }
    }
}

fn sync_dropdown_interaction(
    mut headers: Query<
        (
            &Interaction,
            &DropdownColors,
            &mut BackgroundColor,
            &ChildOf,
        ),
        (With<DropdownHeader>, Changed<Interaction>),
    >,
    mut options: Query<
        (
            &Interaction,
            &DropdownColors,
            &mut BackgroundColor,
            &DropdownOption,
            &DropdownOwnedBy,
        ),
        (
            With<DropdownOption>,
            Without<DropdownHeader>,
            Changed<Interaction>,
        ),
    >,
    dropdowns: Query<&Dropdown>,
) {
    for (interaction, colors, mut background, parent) in headers.iter_mut() {
        let Ok(dropdown) = dropdowns.get(parent.parent()) else {
            continue;
        };
        background.0 = if dropdown.disabled {
            colors.disabled_bg
        } else {
            match *interaction {
                Interaction::Pressed => colors.pressed_bg,
                Interaction::Hovered => colors.hovered_bg,
                Interaction::None => colors.normal_bg,
            }
        };
    }

    for (interaction, colors, mut background, option, owner) in options.iter_mut() {
        let Ok(dropdown) = dropdowns.get(owner.0) else {
            continue;
        };
        let base_color = if dropdown.disabled {
            colors.disabled_bg
        } else if option.index == dropdown.selected {
            colors.selected_bg
        } else {
            colors.menu_bg
        };
        background.0 = if dropdown.disabled {
            base_color
        } else {
            match *interaction {
                Interaction::Pressed => colors.pressed_bg,
                Interaction::Hovered => colors.hovered_bg,
                Interaction::None => base_color,
            }
        };
    }
}

fn sync_dropdown_popup_layout(
    mut commands: Commands,
    window: Single<&Window>,
    dropdowns: Query<(Entity, &Dropdown, &DropdownParts)>,
    header_layout: Query<(&ComputedNode, &UiGlobalTransform), With<DropdownHeader>>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    popup_hosts: Query<(), With<PopupLayerHost>>,
    popup_roots: Query<(), With<PopupLayerRoot>>,
    global_popup_layer: Res<GlobalPopupLayerState>,
    popup_layout: Query<&ComputedNode>,
    ui_transforms: Query<&UiGlobalTransform>,
    mut menu_nodes: Query<&mut Node, (With<DropdownMenu>, With<DropdownPopup>)>,
) {
    let scale_factor = window.scale_factor();
    for (root, dropdown, parts) in dropdowns.iter() {
        let Some(menu) = parts.popup else {
            continue;
        };
        let Some(popup_parent) =
            resolve_popup_parent(root, &parents, &children, &popup_hosts, &popup_roots)
                .or(global_popup_layer.root)
                .or_else(|| Some(topmost_ancestor(root, &parents)))
        else {
            continue;
        };
        if parents
            .get(menu)
            .map(|parent| parent.parent() != popup_parent)
            .unwrap_or(true)
        {
            commands.entity(popup_parent).add_child(menu);
        }
        let Ok((header_node, header_transform)) = header_layout.get(parts.header) else {
            continue;
        };
        let Ok(layer_transform) = ui_transforms.get(popup_parent) else {
            continue;
        };
        let Ok(layer_node) = popup_layout.get(popup_parent) else {
            continue;
        };
        let Ok(mut menu_node) = menu_nodes.get_mut(menu) else {
            continue;
        };

        let header_origin = (header_transform.transform_point2(Vec2::ZERO).as_dvec2()
            / scale_factor as f64)
            .as_vec2();
        let layer_origin = (layer_transform.transform_point2(Vec2::ZERO).as_dvec2()
            / scale_factor as f64)
            .as_vec2();
        let header_size = header_node.size() / scale_factor;
        let layer_size = layer_node.size() / scale_factor;
        let header_top_left = header_origin - header_size * 0.5;
        let layer_top_left = layer_origin - layer_size * 0.5;
        let local_origin = header_top_left - layer_top_left;
        let desired_height = (dropdown
            .options
            .len()
            .min(dropdown.max_visible_items.max(1)) as f32)
            * DROPDOWN_OPTION_HEIGHT
            + DROPDOWN_MENU_PADDING_Y;
        let below_space =
            (layer_size.y - (local_origin.y + header_size.y) - DROPDOWN_EDGE_MARGIN).max(0.0);
        let above_space = (local_origin.y - DROPDOWN_EDGE_MARGIN).max(0.0);
        let open_upward = below_space < desired_height && above_space > below_space;
        let available_height = if open_upward {
            above_space
        } else {
            below_space
        };
        let menu_height = desired_height
            .min(available_height.max(DROPDOWN_MIN_MENU_HEIGHT))
            .max(DROPDOWN_OPTION_HEIGHT + DROPDOWN_MENU_PADDING_Y);
        let menu_top = if open_upward {
            local_origin.y - menu_height
        } else {
            local_origin.y + header_size.y
        };
        let desired_width = estimated_dropdown_popup_width(dropdown).max(header_size.x);
        let max_width = dropdown
            .max_popup_width
            .min((layer_size.x - DROPDOWN_EDGE_MARGIN * 2.0).max(header_size.x));
        let menu_width = desired_width.min(max_width).max(header_size.x);
        let menu_left = local_origin.x.clamp(
            DROPDOWN_EDGE_MARGIN,
            (layer_size.x - menu_width - DROPDOWN_EDGE_MARGIN).max(DROPDOWN_EDGE_MARGIN),
        );

        menu_node.left = px(menu_left);
        menu_node.top = px(menu_top.max(DROPDOWN_EDGE_MARGIN));
        menu_node.width = px(menu_width);
        menu_node.max_height = px(menu_height);
        menu_node.display = Display::Flex;
    }
}

fn dropdown_colors(theme: Option<&Theme>) -> DropdownColors {
    match theme {
        Some(t) => DropdownColors {
            normal_bg: t.palette.surface,
            hovered_bg: t.palette.surface_variant,
            pressed_bg: t.palette.outline_variant,
            border: t.palette.outline,
            menu_bg: t.palette.surface,
            text: t.palette.on_surface,
            selected_bg: t.palette.primary_container,
            disabled_bg: t.palette.disabled_container,
        },
        None => DropdownColors {
            normal_bg: Color::srgb(0.15, 0.15, 0.15),
            hovered_bg: Color::srgb(0.2, 0.2, 0.2),
            pressed_bg: Color::srgb(0.25, 0.25, 0.25),
            border: Color::srgb(0.35, 0.35, 0.35),
            menu_bg: Color::srgb(0.14, 0.14, 0.14),
            text: Color::srgb(0.9, 0.9, 0.9),
            selected_bg: Color::srgb(0.22, 0.35, 0.52),
            disabled_bg: Color::srgb(0.1, 0.1, 0.1),
        },
    }
}

fn topmost_ancestor(mut entity: Entity, parents: &Query<&ChildOf>) -> Entity {
    while let Ok(parent) = parents.get(entity) {
        entity = parent.parent();
    }
    entity
}

fn estimated_dropdown_popup_width(dropdown: &Dropdown) -> f32 {
    let longest = dropdown
        .options
        .iter()
        .map(|option| option.chars().count() as f32)
        .fold(0.0, f32::max);
    longest * dropdown.font_size * DROPDOWN_TEXT_ESTIMATE_FACTOR
        + DROPDOWN_HORIZONTAL_PADDING * 2.0
        + DROPDOWN_CARET_WIDTH
        + 24.0
}

fn first_text_descendant(
    root: Entity,
    children: &Query<&Children>,
    texts: &Query<&mut Text>,
) -> Option<Entity> {
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if texts.contains(entity) {
            return Some(entity);
        }
        if let Ok(kids) = children.get(entity) {
            stack.extend(kids.iter());
        }
    }
    None
}

fn scroll_hovered_dropdown_menus(
    mut mouse_wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut menus: Query<
        (&Hovered, &mut ScrollPosition, &ComputedNode),
        (With<DropdownMenu>, With<DropdownPopup>),
    >,
) {
    let mut delta_y = 0.0;
    for event in mouse_wheel.read() {
        delta_y += match event.unit {
            bevy::input::mouse::MouseScrollUnit::Line => event.y * DROPDOWN_SCROLL_STEP,
            bevy::input::mouse::MouseScrollUnit::Pixel => event.y,
        };
    }
    if delta_y == 0.0 {
        return;
    }

    for (hovered, mut scroll, computed) in menus.iter_mut() {
        if !hovered.get() {
            continue;
        }
        let max_scroll = (computed.content_size().y - computed.size().y).max(0.0);
        scroll.y = (scroll.y - delta_y).clamp(0.0, max_scroll);
    }
}

fn close_dropdowns_on_outside_click(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    mut dropdowns: Query<(&mut Dropdown, &DropdownParts)>,
    header_layout: Query<(&ComputedNode, &UiGlobalTransform), With<DropdownHeader>>,
    menu_layout: Query<
        (&ComputedNode, &UiGlobalTransform),
        (With<DropdownMenu>, With<DropdownPopup>),
    >,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.physical_cursor_position() else {
        return;
    };

    for (mut dropdown, parts) in dropdowns.iter_mut() {
        if !dropdown.expanded {
            continue;
        }

        let header_hit = header_layout
            .get(parts.header)
            .ok()
            .is_some_and(|(node, transform)| node.contains_point(*transform, cursor));
        if header_hit {
            continue;
        }

        let menu_hit = parts.popup.is_some_and(|menu| {
            menu_layout
                .get(menu)
                .ok()
                .is_some_and(|(node, transform)| node.contains_point(*transform, cursor))
        });
        if menu_hit {
            continue;
        }

        dropdown.expanded = false;
    }
}

fn sync_dropdown_popup_presence(
    mut commands: Commands,
    mut dropdowns: Query<
        (Entity, &Dropdown, &DropdownColors, &mut DropdownParts),
        Changed<Dropdown>,
    >,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    popup_hosts: Query<(), With<PopupLayerHost>>,
    popup_roots: Query<(), With<PopupLayerRoot>>,
    global_popup_layer: Res<GlobalPopupLayerState>,
    scopes: Query<&ThemeScope>,
    boundaries: Query<(), With<ThemeBoundary>>,
    global_theme: Option<Res<Theme>>,
) {
    for (root, dropdown, colors, mut parts) in dropdowns.iter_mut() {
        if let Some(existing_popup) = parts.popup.take() {
            commands.entity(existing_popup).despawn();
        }

        if !dropdown.expanded {
            continue;
        }

        let popup_parent =
            resolve_popup_parent(root, &parents, &children, &popup_hosts, &popup_roots)
                .or(global_popup_layer.root)
                .unwrap_or_else(|| topmost_ancestor(root, &parents));

        let theme = resolve_theme_or_global(
            root,
            &parents,
            &scopes,
            &boundaries,
            global_theme.as_deref(),
        );
        let font = theme
            .map(|t| t.typography.body_medium.font.clone())
            .unwrap_or(TextFont::from_font_size(13.0));

        let option_entities = dropdown
            .options
            .iter()
            .enumerate()
            .map(|(index, option)| {
                commands
                    .spawn((
                        Name::new(format!("Dropdown Option [{option}]")),
                        Button,
                        Interaction::default(),
                        Node {
                            width: percent(100.0),
                            min_height: px(DROPDOWN_OPTION_HEIGHT),
                            padding: UiRect::axes(px(8.0), px(4.0)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(if index == dropdown.selected {
                            colors.selected_bg
                        } else {
                            colors.menu_bg
                        }),
                        DropdownOption { index },
                        DropdownOwnedBy(root),
                        *colors,
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Node {
                                width: percent(100.0),
                                overflow: Overflow::clip_x(),
                                ..default()
                            },
                            children![(
                                Text::new(option.clone()),
                                TextLayout::new_with_no_wrap(),
                                font.clone(),
                                TextColor(colors.text),
                            )],
                        ));
                    })
                    .observe(on_dropdown_option_click)
                    .id()
            })
            .collect::<Vec<_>>();

        let popup = commands
            .spawn((
                Name::new("Dropdown Menu"),
                Node {
                    width: percent(100.0),
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    right: px(0.0),
                    top: px(32.0),
                    max_height: px(dropdown.max_visible_items as f32 * DROPDOWN_OPTION_HEIGHT
                        + DROPDOWN_MENU_PADDING_Y),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(px(1.0)),
                    overflow: Overflow::scroll_y(),
                    scrollbar_width: 6.0,
                    ..default()
                },
                BackgroundColor(colors.menu_bg),
                BorderColor::all(colors.border),
                BorderRadius::all(px(4.0)),
                ZIndex(10),
                Hovered::default(),
                DropdownMenu,
                DropdownPopup,
                DropdownOwnedBy(root),
            ))
            .add_children(&option_entities)
            .id();

        commands.entity(popup_parent).add_child(popup);
        parts.popup = Some(popup);
    }
}
