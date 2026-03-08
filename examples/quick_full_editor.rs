use bevy::prelude::*;
use bevy_vista::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VistaUiPlugin)
        .add_plugins((
            bevy_egui::EguiPlugin::default(),
            bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut active: ResMut<VistaEditorActive>,
    mut expanded: ResMut<VistaEditorExpanded>,
    mut mode: ResMut<VistaEditorMode>,
) {
    commands.spawn((Camera2d, IsDefaultUiCamera));

    **active = true;
    **expanded = true;
    *mode = VistaEditorMode::Fullscreen;
}
