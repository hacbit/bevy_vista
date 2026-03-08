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
            PostStartup,
            (
                setup_foldout_example,
                setup_listview_example,
                setup_treeview_example,
            ),
        )
        .add_systems(
            Update,
            (toggle_vista_editor_active, toggle_vista_editor_expanded),
        )
        .run();
}

#[derive(Component)]
struct ExampleUiRoot;

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, IsDefaultUiCamera));

    commands.spawn((
        Node {
            width: px(1000.0),
            height: px(600.0),
            padding: UiRect::all(px(10.0)),
            ..default()
        },
        Name::new("UI Root"),
        BackgroundColor(Color::srgb(0.3, 0.7, 0.8)),
        ExampleUiRoot,
    ));
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

fn setup_foldout_example(
    mut commands: Commands,
    theme: Option<Res<Theme>>,
    root: Single<Entity, With<ExampleUiRoot>>,
) {
    let content = generate_content(&mut commands);

    let foldout = FoldoutBuilder::new("Foldout Example")
        .expanded(true)
        .width(px(200.0))
        .build_with_entity(&mut commands, content, theme.as_deref());

    commands.entity(*root).add_child(foldout);
}

fn generate_content(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                width: px(180.0),
                height: px(300.0),
                padding: UiRect::all(px(4.0)),
                overflow: Overflow::clip(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Children::spawn(SpawnIter((0..10).map(|i| {
                (
                    Node {
                        width: px(180.0),
                        height: px(30.0),
                        margin: UiRect::all(px(2.0)),
                        padding: UiRect::all(px(4.0)),
                        ..default()
                    },
                    Name::new(format!("Item {}", i)),
                    BackgroundColor(Color::srgb(0.8, 0.8, 0.8)),
                )
            }))),
        ))
        .id()
}

fn setup_listview_example(mut commands: Commands, root: Single<Entity, With<ExampleUiRoot>>) {
    let items = generate_list_items(&mut commands);

    let list_view = ListViewBuilder::new()
        .direction(FlexDirection::Column)
        .item_gap(4.0)
        .width(px(200.0))
        .height(px(400.0))
        .build_with_entities(&mut commands, items);

    commands.entity(*root).add_child(list_view);
}

fn generate_list_items(commands: &mut Commands) -> Vec<Entity> {
    (0..10)
        .map(|i| {
            // Create a simple node for each item
            // In a real application, you might want to include more complex content
            commands
                .spawn((
                    Node {
                        width: px(180.0),
                        height: px(30.0),
                        margin: UiRect::all(px(2.0)),
                        padding: UiRect::all(px(4.0)),
                        ..default()
                    },
                    Name::new(format!("Item {}", i)),
                    BackgroundColor(Color::srgb(0.8, 0.8, 0.8)),
                ))
                .id()
        })
        .collect()
}

fn setup_treeview_example(
    mut commands: Commands,
    root: Single<Entity, With<ExampleUiRoot>>,
    theme: Option<Res<Theme>>,
) {
    let tree_view = TreeViewBuilder::new()
        .indent(20.0)
        .item_gap(4.0)
        .width(px(200.0))
        .height(px(400.0))
        .build(&mut commands, generate_tree_nodes(), theme.as_deref());

    commands.entity(*root).add_child(tree_view);
}

fn generate_tree_nodes() -> impl IntoIterator<Item = TreeNodeBuilder> {
    (0..5)
        .map(|i| {
            TreeNodeBuilder::branch(
                format!("Branch {}", i),
                false,
                vec![
                    TreeNodeBuilder::leaf(format!("Leaf {}.1", i)),
                    TreeNodeBuilder::leaf(format!("Leaf {}.2", i)),
                    TreeNodeBuilder::branch(
                        format!("Branch {}.3", i),
                        false,
                        vec![
                            TreeNodeBuilder::leaf(format!("Leaf {}.3.1", i)),
                            TreeNodeBuilder::leaf(format!("Leaf {}.3.2", i)),
                        ],
                    ),
                ],
            )
        })
        .chain((5..8).map(|i| TreeNodeBuilder::leaf(format!("Leaf {}", i))))
}
