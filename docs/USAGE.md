# bevy_vista Usage Guide

This guide focuses on practical integration patterns for `bevy_vista`.

## 1. Pick a Layer

`bevy_vista` now has three public layers.

### 1.1 `core`

Use `bevy_vista::core` when you only need shared types:

- widgets and widget registration
- inspector metadata and drivers
- themes
- `.vista.ron` asset/document types

Plugin:

- `VistaUiCorePlugin`

Prelude:

- `bevy_vista::core::prelude::*`

### 1.2 `runtime`

Use `bevy_vista::runtime` when you want runtime document workflows without the
editor overlay.

Plugin:

- `VistaUiRuntimePlugin`

Prelude:

- `bevy_vista::runtime::prelude::*`

Main runtime API:

- `WidgetDocUtility`

### 1.3 `editor`

Use `bevy_vista::editor` when you want the editor overlay.

Plugin:

- `VistaUiEditorPlugin`

Prelude:

- `bevy_vista::editor::prelude::*`

### 1.4 Full setup

If you want both runtime document APIs and the editor overlay, use:

- `VistaUiPlugin`
- `bevy_vista::prelude::*`

## 2. Installation

Use a `bevy_vista` version that matches your Bevy minor version.

```toml
[dependencies]
bevy = "0.17"
bevy_vista = "0.17.1"
```

## 3. Basic Setup

### 3.1 Full editor + runtime

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

### 3.2 Runtime only

```rust
use bevy::prelude::*;
use bevy_vista::runtime::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VistaUiRuntimePlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, IsDefaultUiCamera));
}
```

### 3.3 Editor only

```rust
use bevy::prelude::*;
use bevy_vista::editor::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VistaUiEditorPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, IsDefaultUiCamera));
}
```

## 4. Editor State Control

The editor layer inserts state resources you can change at runtime:

- `VistaEditorActive(pub bool)`
- `VistaEditorExpanded(pub bool)`
- `VistaEditorMode::{Floating, Fullscreen}`
- `VistaEditorViewOptions`
- `VistaEditorCanvasInfo`
- `VistaEditorGridInfo`

Example hotkeys:

```rust
use bevy::prelude::*;
use bevy_vista::editor::prelude::*;

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

## 5. `.vista.ron` Asset Workflow

Editor toolbar provides `Save`, `Save As`, and `Load`.
Default asset root is `assets/ui/`.

### 5.1 Build and serialize an asset

```rust
use bevy::platform::collections::HashMap;
use bevy::ui::Val;
use bevy_vista::core::asset::{VISTA_UI_ASSET_VERSION, VistaUiAsset, VistaUiNodeAsset};
use bevy_vista::core::widget::WidgetStyle;

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
            slot: None,
            children: vec![],
        }],
    };

    asset.to_ron_string_pretty().unwrap()
}
```

### 5.2 Deserialize and spawn directly

```rust
use bevy::prelude::*;
use bevy_vista::core::asset::VistaUiAsset;
use bevy_vista::core::inspector::InspectorEditorRegistry;
use bevy_vista::core::widget::WidgetRegistry;

fn spawn_from_ron(
    commands: &mut Commands,
    parent: Entity,
    ron: &str,
    widget_registry: &WidgetRegistry,
    inspector_registry: &InspectorEditorRegistry,
) {
    let asset = VistaUiAsset::from_ron_str(ron).unwrap();
    asset
        .spawn_into(commands, parent, widget_registry, inspector_registry, None)
        .unwrap();
}
```

## 6. Runtime Document API

`WidgetDocUtility` is the main high-level runtime API.

The intended model is:

1. load a document
2. mutate the document before spawning
3. spawn an instance
4. later either:
   - mutate the document again and call `flush`
   - or mutate live widget components directly

### 6.1 Load and spawn a saved document

```rust
use bevy::prelude::*;
use bevy_vista::runtime::prelude::*;

fn setup(
    mut commands: Commands,
    mut docs: WidgetDocUtility,
) {
    commands.spawn((Camera2d, IsDefaultUiCamera));

    let root = commands.spawn_empty().id();
    let doc_id = docs
        .load_path("assets/ui/runtime_widget_doc_demo.vista.ron")
        .unwrap();

    let _instance_id = docs.spawn(doc_id, root).unwrap();
}
```

### 6.2 Modify document data before spawning

```rust
use bevy::prelude::*;
use bevy_vista::runtime::prelude::*;

fn setup(
    mut commands: Commands,
    mut docs: WidgetDocUtility,
) {
    let root = commands.spawn_empty().id();
    let doc_id = docs
        .load_path("assets/ui/runtime_widget_doc_demo.vista.ron")
        .unwrap();

    let title = docs.query_first_by_name(doc_id, "title").unwrap();
    docs.with_named_widget_mut::<LabelWidget>(doc_id, Some("title"), |label| {
        label.text = "Loaded at runtime".to_owned();
    })
    .unwrap();
    docs.with_style_mut(doc_id, title, |style| {
        style.width = px(360.0);
    })
    .unwrap();

    let _instance_id = docs.spawn(doc_id, root).unwrap();
}
```

### 6.3 Re-apply document changes to a live instance

```rust
use bevy::prelude::*;
use bevy_vista::runtime::prelude::*;

fn update_doc(
    mut docs: WidgetDocUtility,
    demo: Res<MyDemoState>,
) {
    let button = docs.query_first_by_name(demo.doc_id, "cta").unwrap();
    docs.with_style_mut(demo.doc_id, button, |style| {
        style.display = Display::None;
    })
    .unwrap();
    docs.flush(demo.instance_id).unwrap();
}

#[derive(Resource)]
struct MyDemoState {
    doc_id: WidgetDocId,
    instance_id: WidgetDocInstanceId,
}
```

### 6.4 Mutate live widgets directly

This path changes the spawned component only. It does not write back into the
stored document.

```rust
use bevy::prelude::*;
use bevy_vista::runtime::prelude::*;

fn highlight_title(
    mut docs: WidgetDocUtility,
    mut labels: WidgetDocLiveMut<LabelWidget>,
    demo: Res<MyDemoState>,
) {
    docs.with_named_live_widget_mut(demo.instance_id, "title", &mut labels, |label| {
        label.color = Color::srgb(1.0, 0.4, 0.2);
    })
    .unwrap();
}

#[derive(Resource)]
struct MyDemoState {
    instance_id: WidgetDocInstanceId,
}
```

Reference example:

- [`examples/runtime_load_asset.rs`](../examples/runtime_load_asset.rs)

## 7. Custom Widgets

`bevy_vista` supports derive-based widget registration via `inventory`.

### 7.1 Requirements

Typical custom widget requirements:

- `#[derive(Widget)]`
- `#[widget("category/name")]`
- `#[builder(YourBuilderType)]`
- optional `#[derive(ShowInInspector)]`

For inspector editing support, the widget component should generally be:

- `Component + Reflect + Default + Clone`

### 7.2 Example

```rust
use bevy::prelude::*;
use bevy_vista::core::prelude::*;

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
    fn spawn_default(commands: &mut Commands, _theme: Option<&Theme>) -> WidgetSpawnResult {
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
            .into()
    }
}
```

## 8. Custom Inspector Drivers

The inspector driver system is now designed for external extension.

Typical flow:

1. implement `InspectorDriver`
2. register it with `app.register_inspector_driver(...)`
3. bind a type to the custom driver in `InspectorEditorRegistry`

Because driver dispatch is registry-based, you can replace the default rendering
for an existing Rust type without defining a custom widget type.

Reference example:

- [`examples/custom_inspector_driver.rs`](../examples/custom_inspector_driver.rs)

### 8.1 Minimal shape

```rust
use bevy::prelude::*;
use bevy::reflect::PartialReflect;
use bevy_vista::editor::prelude::*;
use bevy_vista::runtime::prelude::*;

struct MyBoolDriver;

impl InspectorDriver for MyBoolDriver {
    fn id(&self) -> InspectorDriverId {
        "my_bool_driver"
    }

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        DropdownBuilder::new()
            .placeholder(field.label.clone())
            .options(["False", "True"])
            .build(commands, theme)
    }

    fn install_runtime(&self, builder: &mut InspectorDriverRuntimeBuilder) {
        builder.on_apply(apply_bool_changes);
        builder.on_sync(sync_bool_values);
    }

    fn serialize(&self, field: &dyn PartialReflect) -> Option<String> {
        read_bool_field(field).map(|value| value.to_string())
    }

    fn apply_serialized(&self, field: &mut dyn PartialReflect, raw: &str) -> bool {
        match raw {
            "true" => write_bool_field(field, true),
            "false" => write_bool_field(field, false),
            _ => false,
        }
    }
}
```

## 9. `ShowInInspector` Reference

`#[derive(ShowInInspector)]` reads metadata from `#[property(...)]` on named
struct fields.

Current supported options:

- `label = "..."`
  custom display label
- `hidden`
  omit the field from the generated inspector metadata
- `min = 0.0`
  numeric minimum used by editing controls
- `header = "..."`
  start a new header/foldout group and implicitly close the previous open group
- `default_open = false`
  only meaningful together with `header = "..."`
- `end_header`
  closes the current header group after this field

Important details:

- use a single `#[property(...)]` attribute per field
- options are comma-separated inside that attribute
- the derive currently supports named-field structs only
- a new `header = "..."` implicitly closes the previous open header group

Example:

```rust
use bevy_vista::core::prelude::*;

#[derive(ShowInInspector)]
struct CardProps {
    #[property(header = "Layout", default_open = true, label = "Width", min = 0.0)]
    width: f32,

    #[property(label = "Visible", end_header)]
    visible: bool,

    #[property(hidden)]
    internal_id: u32,
}
```

## 10. Examples

- `cargo run --example quick_vista_editor`
- `cargo run --example quick_full_editor`
- `cargo run --example runtime_load_asset`
- `cargo run --example custom_inspector_driver`
- `cargo run --example layout_widgets`
- `cargo run --example text_field_showcase`
- `cargo run --example editor_icons_gallery`
