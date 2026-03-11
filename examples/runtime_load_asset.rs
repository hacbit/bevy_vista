use std::path::PathBuf;

use bevy::prelude::*;
use bevy_vista::runtime::prelude::*;

#[derive(Resource)]
struct RuntimeDocDemo {
    doc_id: WidgetDocId,
    instance_id: WidgetDocInstanceId,
    cta_node: BlueprintNodeId,
    hidden: bool,
    title_live_accent: bool,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VistaUiRuntimePlugin)
        .insert_resource(Theme::quick(
            "Runtime Demo",
            Color::srgb(0.20, 0.58, 0.86),
            ThemeMode::Dark,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_cta_visibility, toggle_title_live_color))
        .run();
}

fn setup(mut commands: Commands, mut docs: WidgetDocUtility) {
    commands.spawn((Camera2d, IsDefaultUiCamera));

    let root = commands
        .spawn((
            Name::new("Runtime UI Root"),
            Node {
                width: percent(100.0),
                height: percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .id();

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/ui/runtime_widget_doc_demo.vista.ron");
    let doc_id = docs.load_path(&path).expect("runtime ui asset should load");

    let cta_node = docs
        .query_first::<ButtonWidget>(doc_id, Some("cta"))
        .expect("button widget should be registered")
        .expect("cta node should exist");

    docs.with_named_widget_mut::<LabelWidget>(doc_id, Some("title"), |label| {
        label.text = "Loaded from .vista.ron".to_owned();
    })
    .expect("title widget should update");
    docs.with_style_mut(doc_id, cta_node, |style| {
        style.margin.top = px(18.0);
    })
    .expect("cta style should update");

    let instance_id = docs.spawn(doc_id, root).expect("ui document should spawn");
    let button_entity = docs
        .entity_of::<ButtonWidget>(instance_id, Some("cta"))
        .expect("button widget should be registered")
        .expect("cta entity should exist after spawn");
    info!("Spawned CTA button entity: {button_entity:?}");

    commands.insert_resource(RuntimeDocDemo {
        doc_id,
        instance_id,
        cta_node,
        hidden: false,
        title_live_accent: false,
    });
}

fn toggle_cta_visibility(
    input: Res<ButtonInput<KeyCode>>,
    state: Option<ResMut<RuntimeDocDemo>>,
    mut docs: WidgetDocUtility,
) {
    let Some(mut state) = state else {
        return;
    };
    if !input.just_pressed(KeyCode::Space) {
        return;
    }

    state.hidden = !state.hidden;
    let hidden = state.hidden;
    docs.with_style_mut(state.doc_id, state.cta_node, |style| {
        style.display = if hidden { Display::None } else { Display::Flex };
    })
    .expect("cta style should update");
    docs.flush(state.instance_id)
        .expect("runtime ui document should flush");
}

fn toggle_title_live_color(
    input: Res<ButtonInput<KeyCode>>,
    state: Option<ResMut<RuntimeDocDemo>>,
    docs: WidgetDocUtility,
    mut labels: WidgetDocLiveMut<LabelWidget>,
) {
    let Some(mut state) = state else {
        return;
    };
    if !input.just_pressed(KeyCode::Enter) {
        return;
    }

    state.title_live_accent = !state.title_live_accent;
    let color = if state.title_live_accent {
        Color::srgb(0.96, 0.68, 0.22)
    } else {
        Color::srgb(0.9, 0.9, 0.9)
    };
    docs.with_named_live_widget_mut::<LabelWidget>(
        state.instance_id,
        Some("title"),
        &mut labels,
        |label| {
            label.color = color;
        },
    )
    .expect("title live widget should update");
}
