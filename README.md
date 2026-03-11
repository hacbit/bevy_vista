# bevy_vista

`bevy_vista` is a document-driven UI editor and runtime toolkit for Bevy UI.
It combines:

- an editor overlay built with pure Bevy UI
- inspector-driven widget editing
- `.vista.ron` serialization for reusable UI documents
- runtime APIs for loading, mutating, and spawning saved UI

## Latest Version

- `bevy_vista = "0.17.1"`
- compatible with `bevy = "0.17"`

## What's New

Recent additions in the current API surface:

- clearer `core` / `runtime` / `editor` module split
- runtime document utilities via `WidgetDocUtility`
- runtime asset loading and spawning example
- cleaner custom inspector driver registration flow
- slimmer custom inspector driver context API
- improved public exports and layered preludes

## Layered Architecture

`bevy_vista` is organized into three layers:

- `bevy_vista::core`
  shared widgets, themes, asset/document models, inspector metadata, and built-in drivers
- `bevy_vista::runtime`
  document loading, mutation, spawning, flushing, and live widget access
- `bevy_vista::editor`
  editor overlay, hierarchy, viewport, toolbar, and inspector panels

Plugin entry points:

- `VistaUiCorePlugin`
- `VistaUiRuntimePlugin`
- `VistaUiEditorPlugin`
- `VistaUiPlugin`
  full setup, combining editor and runtime layers

Prelude entry points:

- `bevy_vista::core::prelude::*`
- `bevy_vista::runtime::prelude::*`
- `bevy_vista::editor::prelude::*`
- `bevy_vista::prelude::*`

## Features

- editor UI based on pure Bevy UI
- floating + fullscreen editor modes
- viewport grid, zoom, pan, and preview workflow
- save / load `.vista.ron` UI documents
- `Widget` derive + auto-registration for custom widgets
- `ShowInInspector` derive for inspector-editable properties
- custom inspector driver registration
- runtime document APIs for document-first UI workflows
- 100+ editor icons

## Quick Start

```toml
[dependencies]
bevy = "0.17"
bevy_vista = "0.17.1"
```

Full editor + runtime:

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

Runtime only:

```rust
use bevy::prelude::*;
use bevy_vista::runtime::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VistaUiRuntimePlugin)
        .run();
}
```

## Runtime Document Workflow

The runtime layer keeps a document as the source of truth, then spawns one or
more live instances from it.

Typical flow:

1. `load_path`
2. mutate document data with `with_widget_mut` / `with_style_mut`
3. `spawn`
4. later `flush` after document-side changes
5. or mutate live widgets directly with `with_live_widget_mut`

Main API:

- `WidgetDocUtility`
- `WidgetDocId`
- `WidgetDocInstanceId`
- `WidgetDocLiveRef<T>`
- `WidgetDocLiveMut<T>`

See:

- [`examples/runtime_load_asset.rs`](./examples/runtime_load_asset.rs)
- [`assets/ui/runtime_widget_doc_demo.vista.ron`](./assets/ui/runtime_widget_doc_demo.vista.ron)

## Custom Inspector Drivers

Built-in field rendering is string-driver based, so external code can replace or
extend inspector rendering without modifying internal editor modules.

Typical customization flow:

1. implement `InspectorDriver`
2. register it with `app.register_inspector_driver(...)`
3. bind a Rust type to that driver through `InspectorEditorRegistry`

This now works with the streamlined runtime driver API instead of requiring
editor-private internals.

See:

- [`examples/custom_inspector_driver.rs`](./examples/custom_inspector_driver.rs)

## `ShowInInspector` Options

`#[derive(ShowInInspector)]` currently supports these field-level
`#[property(...)]` options:

- `label = "Text"`
  custom display label in the inspector
- `hidden`
  omit the field from the inspector
- `min = 0.0`
  numeric minimum used by runtime editing controls
- `header = "Layout"`
  start a grouped foldout section and implicitly close the previous open section
- `default_open = false`
  only valid together with `header = "..."`
- `end_header`
  closes the current header section explicitly

Example:

```rust
#[derive(ShowInInspector)]
struct DemoProps {
    #[property(header = "Layout", default_open = true, label = "Width", min = 0.0)]
    width: f32,

    #[property(label = "Visible", end_header)]
    visible: bool,
}
```

If a later field starts another `header = "..."` section, the previous section is
closed automatically even without `end_header`.

See the detailed guide in [`docs/USAGE.md`](./docs/USAGE.md) for a fuller
breakdown.

## Examples

- `cargo run --example quick_vista_editor`
- `cargo run --example quick_full_editor`
- `cargo run --example runtime_load_asset`
- `cargo run --example custom_inspector_driver`
- `cargo run --example layout_widgets`
- `cargo run --example text_field_showcase`
- `cargo run --example editor_icons_gallery`

## Documentation

- [Usage Guide](./docs/USAGE.md)
- crate docs on [docs.rs](https://docs.rs/bevy_vista)

## Screenshots

### Full editor layout

example: [`quick_full_editor.rs`](./examples/quick_full_editor.rs)

![full_editor](README/full_editor.png)

### Editor Icons

example: [`editor_icons_gallery.rs`](./examples/editor_icons_gallery.rs)

![editor_icons](README/editor_icons.png)
