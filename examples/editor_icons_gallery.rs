use bevy::prelude::*;
use bevy_vista::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Editor Icons Gallery".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(VistaUiPlugin)
        .add_systems(Startup, (setup_camera, setup_gallery))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, IsDefaultUiCamera));
}

fn setup_gallery(mut commands: Commands, theme: Res<Theme>) {
    let icons = Icons::reflected_variants();
    let header = spawn_header(&mut commands, &theme, icons.len());

    let cards: Vec<Entity> = icons
        .iter()
        .map(|(name, icon)| spawn_icon_card(&mut commands, &theme, name, *icon))
        .collect();

    let grid = commands
        .spawn((
            Name::new("Icons Grid"),
            Node {
                width: percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                align_content: AlignContent::FlexStart,
                column_gap: px(theme.spacing.md),
                row_gap: px(theme.spacing.md),
                ..default()
            },
        ))
        .id();
    commands.entity(grid).add_children(&cards);

    let scroll = ScrollViewBuilder::new()
        .width(percent(100.0))
        .height(percent(100.0))
        .show_horizontal(false)
        .vertical_bar(ScrollbarVisibility::Auto)
        .content_padding(UiRect::all(px(theme.spacing.md)))
        .scroll_area_bg(theme.palette.surface)
        .build_with_entities(&mut commands, [grid]);

    let root = commands
        .spawn((
            Name::new("Editor Icons Gallery"),
            Node {
                width: percent(100.0),
                height: percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(theme.spacing.md),
                padding: UiRect::all(px(theme.spacing.lg)),
                ..default()
            },
            BackgroundColor(theme.palette.background),
        ))
        .id();
    commands.entity(root).add_children(&[header, scroll]);
}

fn spawn_header(commands: &mut Commands, theme: &Theme, icon_count: usize) -> Entity {
    let title = commands
        .spawn((
            Text::new("Editor Icons"),
            theme.typography.title_large.font.clone(),
            TextColor(theme.palette.on_surface),
        ))
        .id();
    let subtitle = commands
        .spawn((
            Text::new(format!("{icon_count} icons")),
            theme.typography.body_small.font.clone(),
            TextColor(theme.palette.on_surface_muted),
        ))
        .id();

    let header = commands
        .spawn((
            Name::new("Gallery Header"),
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(theme.spacing.xs),
                padding: UiRect::axes(px(theme.spacing.sm), px(theme.spacing.xs)),
                ..default()
            },
        ))
        .id();
    commands.entity(header).add_children(&[title, subtitle]);
    header
}

fn spawn_icon_card(commands: &mut Commands, theme: &Theme, name: &str, icon: Icons) -> Entity {
    let icon_frame = commands
        .spawn((
            Name::new(format!("{name} Frame")),
            Node {
                width: px(48.0),
                height: px(48.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(theme.palette.surface_variant),
            BorderColor::all(theme.palette.outline_variant),
            BorderRadius::all(px(theme.radius.md)),
        ))
        .id();

    let image = commands
        .spawn((
            Node {
                width: px(34.0),
                height: px(34.0),
                ..default()
            },
            icon,
        ))
        .id();
    commands.entity(icon_frame).add_child(image);

    let title = commands
        .spawn((
            Text::new(humanize_icon_name(name)),
            theme.typography.label_medium.font.clone(),
            TextColor(theme.palette.on_surface),
        ))
        .id();
    let raw_name = commands
        .spawn((
            Text::new(name),
            theme.typography.label_small.font.clone(),
            TextColor(theme.palette.on_surface_muted),
        ))
        .id();

    let card = commands
        .spawn((
            Name::new(format!("Icon Card {name}")),
            Node {
                width: px(164.0),
                min_height: px(108.0),
                padding: UiRect::all(px(theme.spacing.md)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                row_gap: px(theme.spacing.sm),
                ..default()
            },
            BackgroundColor(theme.palette.surface),
            BorderColor::all(theme.palette.outline_variant),
            BorderRadius::all(px(theme.radius.lg)),
        ))
        .id();
    commands
        .entity(card)
        .add_children(&[icon_frame, title, raw_name]);
    card
}

fn humanize_icon_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut prev_is_lower_or_digit = false;

    for ch in name.chars() {
        if ch.is_ascii_uppercase() && prev_is_lower_or_digit {
            out.push(' ');
        }
        prev_is_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        out.push(ch);
    }

    out
}
