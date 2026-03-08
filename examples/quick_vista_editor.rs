use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_vista::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VistaUiPlugin)
        .add_plugins((EguiPlugin::default(), WorldInspectorPlugin::new()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_vista_editor_active,
                toggle_vista_editor_expanded,
                toggle_vista_editor_fullscreen,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, IsDefaultUiCamera));
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

fn toggle_vista_editor_fullscreen(
    input: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<VistaEditorMode>,
) {
    if input.just_pressed(KeyCode::F3) {
        *mode = match *mode {
            VistaEditorMode::Floating => VistaEditorMode::Fullscreen,
            VistaEditorMode::Fullscreen => VistaEditorMode::Floating,
        };
    }
}
