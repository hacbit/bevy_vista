use bevy::prelude::*;
use bevy_vista::prelude::*;
use bevy_vista::theme::{Theme, ThemeMode, ThemeSeed};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VistaUiPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (toggle_vista_editor_active, toggle_vista_editor_expanded),
        )
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((Camera2d, IsDefaultUiCamera));

    let font = assets.load("msyh.ttc");
    let seed =
        ThemeSeed::new("Vista", Color::srgb(0.23, 0.89, 0.40), ThemeMode::Dark).with_font(font);
    let theme = Theme::generate(seed);
    commands.insert_resource(theme.clone());

    spawn_labeled_field(
        &mut commands,
        &theme,
        "1) FixedTruncate: fixed width, overflow truncated",
        TextFieldBuilder::new()
            .placeholder("Fixed width truncate...")
            .width(px(320.0))
            .height(px(34.0))
            .layout_mode(TextFieldLayoutMode::FixedTruncate),
        90.0,
    );

    spawn_labeled_field(
        &mut commands,
        &theme,
        "2) AutoWidth: grows with input",
        TextFieldBuilder::new()
            .placeholder("Auto width...")
            .width(px(220.0))
            .height(px(34.0))
            .layout_mode(TextFieldLayoutMode::AutoWidth),
        170.0,
    );

    spawn_labeled_field(
        &mut commands,
        &theme,
        "3) AutoWrap: fixed width, auto height",
        TextFieldBuilder::new()
            .placeholder("Auto wrap in fixed width box...")
            .width(px(320.0))
            .height(px(34.0))
            .layout_mode(TextFieldLayoutMode::AutoWrap),
        250.0,
    );

    spawn_labeled_field(
        &mut commands,
        &theme,
        "4) MultiLine: Enter inserts newline",
        TextFieldBuilder::new()
            .placeholder("Multiline input...")
            .width(px(380.0))
            .height(px(72.0))
            .layout_mode(TextFieldLayoutMode::MultiLine),
        350.0,
    );
}

fn spawn_labeled_field(
    commands: &mut Commands,
    theme: &Theme,
    label: &str,
    builder: TextFieldBuilder,
    top: f32,
) {
    let label_style = &theme.typography.label_large;
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(70.0),
            top: px(top - 24.0),
            ..default()
        },
        Text::new(label),
        label_style.font.clone(),
        TextColor(theme.palette.on_surface),
    ));

    let field = builder.build(commands, Some(theme));
    commands
        .entity(field)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.position_type = PositionType::Absolute;
            node.left = px(70.0);
            node.top = px(top);
        });
}

fn toggle_vista_editor_active(
    input: Res<ButtonInput<KeyCode>>,
    mut active: ResMut<VistaEditorActive>,
) {
    if input.just_pressed(KeyCode::F1) {
        **active = !**active;
    }
}

fn toggle_vista_editor_expanded(
    input: Res<ButtonInput<KeyCode>>,
    mut expanded: ResMut<VistaEditorExpanded>,
) {
    if input.just_pressed(KeyCode::F2) {
        **expanded = !**expanded;
    }
}
