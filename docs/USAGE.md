# bevy_vista Usage Guide

This guide focuses on practical integration patterns for `bevy_vista` in a Bevy project.

## 1. Getting Started

### 1.1 Add dependency

Use a `bevy_vista` version that matches your Bevy minor version.

```toml
[dependencies]
bevy = "0.17"
bevy_vista = "0.17"
```

### 1.2 Enable the editor plugin

```rust
use bevy::prelude::*;
use bevy_vista::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VistaUiPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, IsDefaultUiCamera));
}
```

`VistaUiPlugin` initializes editor UI, widget registry, theme resources, icons, and `.vista.ron` asset support.

## 2. Runtime Control

The plugin inserts editor state resources you can modify at runtime:

- `VistaEditorActive(pub bool)`
- `VistaEditorExpanded(pub bool)`
- `VistaEditorMode::{Floating, Fullscreen}`
- `VistaEditorViewOptions`
- `VistaEditorCanvasInfo`
- `VistaEditorGridInfo`

Example hotkeys:

```rust
use bevy::prelude::*;
use bevy_vista::prelude::*;

fn editor_hotkeys(
    input: Res<ButtonInput<KeyCode>>,
    mut active: ResMut<VistaEditorActive>,
    mut expanded: ResMut<VistaEditorExpanded>,
    mut mode: ResMut<VistaEditorMode>,
) {
    if input.just_pressed(KeyCode::F1) {
        **active = !**active;
    }
    if input.just_pressed(KeyCode::F2) {
        **expanded = !**expanded;
    }
    if input.just_pressed(KeyCode::F3) {
        *mode = match *mode {
            VistaEditorMode::Floating => VistaEditorMode::Fullscreen,
            VistaEditorMode::Fullscreen => VistaEditorMode::Floating,
        };
    }
}
```

## 3. Asset Workflow (`.vista.ron`)

Editor toolbar provides `Save`, `Save As`, and `Load`.
Default asset root is `assets/ui/`.

### 3.1 Build and serialize an asset

```rust
use bevy::platform::collections::HashMap;
use bevy::ui::Val;
use bevy_vista::asset::{VISTA_UI_ASSET_VERSION, VistaUiAsset, VistaUiNodeAsset};
use bevy_vista::widget::WidgetStyle;

fn to_ron_string() -> String {
    let mut style = WidgetStyle::default();
    style.width = Val::Px(320.0);

    let asset = VistaUiAsset {
        version: VISTA_UI_ASSET_VERSION,
        roots: vec![1],
        nodes: vec![VistaUiNodeAsset {
            id: 1,
            name: "Root Button".to_owned(),
            widget_path: "common/button".to_owned(),
            style,
            props: HashMap::new(),
            children: vec![],
        }],
    };

    asset
        .to_ron_string_pretty()
        .expect("failed to encode vista asset")
}
```

### 3.2 Deserialize and spawn to UI tree

```rust
use bevy::prelude::*;
use bevy_vista::asset::VistaUiAsset;
use bevy_vista::inspector::InspectorEditorRegistry;
use bevy_vista::widget::WidgetRegistry;

fn spawn_from_ron(
    commands: &mut Commands,
    parent: Entity,
    ron: &str,
    widget_registry: &WidgetRegistry,
    inspector_registry: &InspectorEditorRegistry,
) {
    let asset = VistaUiAsset::from_ron_str(ron).expect("invalid ron");
    asset
        .spawn_into(commands, parent, widget_registry, inspector_registry, None)
        .expect("failed to spawn vista ui");
}
```

## 4. Custom Widgets

`bevy_vista` supports derive-based widget registration via `inventory`.

### 4.1 Derive requirements

- `#[derive(Widget)]` with `#[widget("category/name")]`
- `#[builder(YourBuilderType)]`
- Optional inspector metadata: `#[derive(ShowInInspector)]`

For inspector editing support, your widget type should satisfy:

- `Component + Reflect + Default + Clone + ShowInInspector`

### 4.2 Example

```rust
use bevy::prelude::*;
use bevy_vista::prelude::*;

#[derive(Component, Reflect, Default, Clone, Widget, ShowInInspector)]
#[reflect(Component)]
#[widget("custom/my_label")]
#[builder(MyLabelBuilder)]
pub struct MyLabel {
    #[property(label = "Text")]
    pub text: String,

    #[property(label = "Font Size", min = 1.0)]
    pub font_size: f32,
}

pub struct MyLabelBuilder;

impl DefaultWidgetBuilder for MyLabelBuilder {
    fn spawn_default(commands: &mut Commands, _theme: Option<&Theme>) -> Entity {
        commands
            .spawn((
                Name::new("MyLabel"),
                MyLabel {
                    text: "Hello Vista".to_owned(),
                    font_size: 18.0,
                },
                Node::default(),
            ))
            .id()
    }
}
```

## 5. Examples

```bash
cargo run --example quick_vista_editor
cargo run --example quick_full_editor
cargo run --example layout_widgets
cargo run --example text_field_showcase
cargo run --example editor_icons_gallery
```

