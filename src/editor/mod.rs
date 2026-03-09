use bevy::app::App;
use bevy::ecs::{name::Name, system::Commands};
use bevy::ui::{Node, Val};
use bevy::{prelude::*, state::state::FreelyMutableState};

pub(crate) mod blueprint;
mod foldable;
mod hierarchy;
mod inspector;
mod title_draggable;
mod toolbar;
mod viewport;
mod widget_lib;

use crate::{
    grid::GridUiMaterial,
    prelude::*,
    theme::{EditorTheme, Theme, ThemeScope},
    widget::{SplitViewAxis, SplitViewBuilder},
};

pub(crate) fn init_editor_ui(app: &mut App) {
    use VistaEditorInitPhase::*;

    app.add_plugins(UiMaterialPlugin::<GridUiMaterial>::default())
        .init_resource::<widget_lib::WidgetLibDragState>()
        .init_resource::<blueprint::WidgetBlueprintDocument>()
        .init_resource::<blueprint::WidgetSchemaRegistry>()
        .init_resource::<blueprint::BlueprintRuntimeMap>()
        .init_resource::<toolbar::EditorDocumentPath>()
        .init_resource::<toolbar::EditorDocumentToolbarState>()
        .init_resource::<crate::inspector::InspectorEditorRegistry>()
        .init_resource::<inspector::InspectorPanelState>()
        .init_resource::<inspector::InspectorControlRegistry>()
        .init_resource::<hierarchy::HierarchyState>()
        .init_resource::<hierarchy::HierarchyDragState>()
        .init_resource::<hierarchy::HierarchyTreeCache>()
        .insert_state(Pending)
        // handle editor phase
        .add_systems(
            Update,
            change_state_to(BasicLayout)
                .run_if(in_state(Pending).and(resource_equals(VistaEditorActive(true)))),
        )
        .add_systems(
            OnEnter(BasicLayout),
            (
                spawn_editor_basic_layout,
                spawn_content_panels.after(spawn_editor_basic_layout),
                change_state_to(ElementNonInteractable),
            ),
        )
        .add_systems(
            OnEnter(ElementNonInteractable),
            (
                init_editor_title_bar,
                toolbar::init_main_toolbar,
                viewport::init_viewport_panel,
                widget_lib::init_widget_lib_panel,
                hierarchy::init_hierarchy_panel,
                inspector::init_inspector_panel,
                toolbar::init_status_bar,
                change_state_to(ElementInteractable),
            ),
        )
        .add_systems(
            OnEnter(ElementInteractable),
            (
                register_editor_foldable_observer,
                register_title_bar_draggable_observer,
                change_state_to(Finalize),
            ),
        )
        // some systems that are executed in Finalize phase
        .add_systems(
            Update,
            (
                (
                    toolbar::apply_document_toolbar_actions,
                    toolbar::sync_editor_toolbar_status,
                    widget_lib::attach_widget_lib_tree_item_observers,
                    viewport::apply_viewport_toolbar_changes,
                    viewport::sync_viewport_toolbar,
                    viewport::sync_viewport_theme_scope,
                    viewport::sync_viewport_canvas_defaults,
                    viewport::sync_viewport_grid_material,
                    viewport::sync_canvas_size_label_text,
                    blueprint::delete_selected_blueprint_node_shortcut,
                    blueprint::compile_blueprint_document,
                    hierarchy::refresh_hierarchy_panel,
                    hierarchy::attach_hierarchy_tree_item_observers,
                )
                    .chain(),
                (
                    inspector::apply_inspector_name_changes,
                    inspector::apply_inspector_numeric_changes,
                    inspector::apply_inspector_string_changes,
                    inspector::apply_inspector_dropdown_changes,
                    inspector::apply_inspector_checkbox_changes,
                    inspector::apply_inspector_color_changes,
                    inspector::refresh_inspector_panel,
                    inspector::sync_widget_property_section,
                    inspector::sync_inspector_numeric_controls,
                    inspector::sync_inspector_string_controls,
                    inspector::sync_inspector_dropdown_controls,
                    inspector::sync_inspector_checkbox_controls,
                    inspector::sync_inspector_color_controls,
                    inspector::sync_inspector_val_controls,
                    inspector::sync_inspector_vec2_controls,
                    inspector::sync_inspector_field_markers,
                )
                    .chain(),
                viewport::toggle_preview_mode_with_key,
                viewport::update_canvas_widget_handles_for_mode,
                viewport::update_canvas_widget_selection_visual,
                toggle_vista_editor_overlay_display.run_if(resource_changed::<VistaEditorActive>),
                sync_vista_editor_mode.run_if(resource_changed::<VistaEditorMode>),
                foldable::toggle_vista_editor_expanded.run_if(
                    resource_changed::<VistaEditorExpanded>.or(resource_changed::<VistaEditorMode>),
                ),
                (
                    viewport::handle_canvas_zooming,
                    viewport::handle_canvas_panning,
                    viewport::sync_viewport_grid_alignment,
                    viewport::sync_canvas_size_label_positions,
                )
                    .run_if(resource_equals(VistaEditorExpanded(true))),
            )
                .run_if(in_state(Finalize)),
        );
}

#[derive(States, Debug, Hash, PartialEq, Eq, Clone, Copy, Reflect)]
enum VistaEditorInitPhase {
    Pending,
    /// Only ui nodes and some required marker component.
    BasicLayout,
    /// Resource loading and Ui refinement.
    ElementNonInteractable,
    /// Add interactive features.
    ElementInteractable,
    Finalize,
}

fn change_state_to<S: FreelyMutableState + Copy>(
    next: S,
) -> impl FnMut(Option<ResMut<NextState<S>>>) {
    move |state| {
        if let Some(mut state) = state {
            state.set(next);
        }
    }
}

const EDITOR_DEFAULT_Z_INDEX: i32 = 114514;

const PANEL_BORDER_RADIUS: Val = Val::Px(5.);

fn editor_frame_border(theme: &Theme) -> Color {
    theme.palette.outline_variant
}

fn editor_inner_border(theme: &Theme) -> Color {
    theme.palette.outline
}

fn editor_title_bar_bg(theme: &Theme) -> Color {
    theme.palette.surface_variant.darker(0.18)
}

fn editor_title_text(theme: &Theme) -> Color {
    theme.palette.on_surface_muted
}

fn editor_content_bg(theme: &Theme) -> Color {
    theme.palette.surface
}

/// Collects marker components used in Vista Editor
#[allow(non_snake_case)]
pub mod VistaMarker {
    use super::*;

    #[derive(Component)]
    pub struct EditorRoot;

    #[derive(Component)]
    pub struct TitleBar;

    #[derive(Component)]
    pub struct TitleFoldButton;

    #[derive(Component)]
    pub struct ContentRoot;

    #[derive(Component)]
    pub struct PopupRoot;

    #[derive(Component)]
    pub struct MainToolbar;

    #[derive(Component)]
    pub struct WidgetLib;

    #[derive(Component)]
    pub struct Hierarchy;

    #[derive(Component)]
    pub struct Viewport;

    #[derive(Component)]
    pub struct Inspector;
}

use VistaMarker::*;

fn spawn_editor_basic_layout(
    mut commands: Commands,
    overlay: Option<Single<&EditorRoot>>,
    active: Res<VistaEditorActive>,
    expanded: Res<VistaEditorExpanded>,
    mode: Res<VistaEditorMode>,
    editor_theme: Res<EditorTheme>,
) {
    if overlay.is_some() {
        warn!("Vista Editor exists, do not spawn twice!");
        return;
    }

    let is_fullscreen = matches!(*mode, VistaEditorMode::Fullscreen);
    let theme = &editor_theme.0;

    commands
        .spawn((
            Name::new("Vista Editor Root"),
            Node {
                flex_grow: 1.,
                flex_shrink: 1.,
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                left: if is_fullscreen { px(0.0) } else { Val::Auto },
                top: if is_fullscreen { px(0.0) } else { Val::Auto },
                right: if is_fullscreen { px(0.0) } else { Val::Auto },
                bottom: if is_fullscreen { px(0.0) } else { Val::Auto },
                border: UiRect::all(Val::Px(1.)),
                display: if **active {
                    Display::Flex
                } else {
                    Display::None
                },
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(EDITOR_DEFAULT_Z_INDEX),
            BorderColor::all(editor_frame_border(theme)),
            BorderRadius::all(PANEL_BORDER_RADIUS),
            EditorRoot,
            PopupLayerHost,
            ThemeScope(theme.clone()),
            BoxShadow(vec![ShadowStyle {
                color: theme.palette.shadow.with_alpha(0.78),
                x_offset: px(10.),
                y_offset: px(10.),
                spread_radius: px(5.),
                blur_radius: px(10.),
            }]),
            UiTransform::from_translation(if is_fullscreen {
                Val2::ZERO
            } else {
                Val2::px(20., 20.)
            }),
        ))
        .with_children(|parent| {
            // title bar
            parent.spawn((
                Name::new("Title Bar"),
                Node {
                    flex_grow: 0.,
                    flex_shrink: 1.,
                    flex_direction: FlexDirection::Row,
                    padding: UiRect::px(5., 20., 0., 0.),
                    ..default()
                },
                TitleBar,
                BackgroundColor(editor_title_bar_bg(theme)),
                BorderRadius::all(PANEL_BORDER_RADIUS),
            ));

            // content ui (expanded)
            parent.spawn((
                Name::new("Content"),
                Node {
                    width: if is_fullscreen {
                        percent(100.0)
                    } else {
                        px(800.0)
                    },
                    height: if is_fullscreen { Val::Auto } else { px(500.0) },
                    flex_grow: if is_fullscreen { 1.0 } else { 0.0 },
                    flex_shrink: if is_fullscreen { 1.0 } else { 0.0 },
                    flex_direction: FlexDirection::Column,
                    display: if is_fullscreen || **expanded {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    overflow: Overflow::clip(),
                    ..default()
                },
                ContentRoot,
                ZIndex(0),
                BackgroundColor(editor_content_bg(theme)),
                BorderRadius::bottom(PANEL_BORDER_RADIUS),
            ));

            parent.spawn((
                Name::new("Popup Root"),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    right: px(0.0),
                    top: px(0.0),
                    bottom: px(0.0),
                    width: percent(100.0),
                    height: percent(100.0),
                    ..default()
                },
                Pickable::IGNORE,
                ZIndex(1000),
                PopupLayerRoot,
                PopupRoot,
            ));
        });
}

fn spawn_content_panels(
    mut commands: Commands,
    content_ui: Single<Entity, With<ContentRoot>>,
    editor_theme: Res<EditorTheme>,
) {
    let editor_theme = &editor_theme.0;
    let theme = Some(editor_theme);

    let toolbar = commands
        .spawn((
            Name::new("Main Toolbar"),
            Node {
                flex_grow: 0.,
                flex_shrink: 0.,
                flex_direction: FlexDirection::Row,
                min_height: Val::Px(36.),
                padding: UiRect::axes(px(8.0), px(4.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                border: UiRect::bottom(Val::Px(1.)),
                ..default()
            },
            BackgroundColor(editor_theme.palette.surface_variant.darker(0.08)),
            BorderColor::all(editor_inner_border(editor_theme)),
            MainToolbar,
        ))
        .id();

    let widget_lib = commands
        .spawn((
            Name::new("Widget Lib"),
            Node {
                padding: px(4.).all(),
                ..default()
            },
            WidgetLib,
        ))
        .id();
    let hierarchy = commands
        .spawn((Name::new("Hierarchy"), Node::default(), Hierarchy))
        .id();
    let left_split = SplitViewBuilder::new()
        .axis(SplitViewAxis::Vertical)
        .build_with_entities(&mut commands, widget_lib, hierarchy, theme);

    let viewport = commands
        .spawn((Name::new("Viewport"), Node::default(), Viewport))
        .id();
    let inspector = commands
        .spawn((Name::new("Inspector"), Node::default(), Inspector))
        .id();
    let right_split = SplitViewBuilder::new()
        .default_first_size(Val::Percent(50.))
        .min_first_size(Val::Percent(30.))
        .min_second_size(Val::Px(100.))
        .build_with_entities(&mut commands, viewport, inspector, theme);

    let content_split = SplitViewBuilder::new()
        .default_first_size(Val::Percent(30.))
        .min_first_size(Val::Percent(20.))
        .min_second_size(Val::Percent(50.))
        .build_with_entities(&mut commands, left_split, right_split, theme);

    commands
        .entity(*content_ui)
        .add_children(&[toolbar, content_split]);
}

fn init_editor_title_bar(
    mut commands: Commands,
    title_bar: Single<Entity, With<TitleBar>>,
    editor_theme: Res<EditorTheme>,
) {
    let theme = &editor_theme.0;
    commands.entity(*title_bar).with_children(|parent| {
        // a button to toggle collapsed/expanded
        parent.spawn((
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(4.)),
                ..default()
            },
            TitleFoldButton,
            Icons::TriangleRight,
        ));

        // title text
        parent.spawn((
            Node {
                align_self: AlignSelf::Center,
                flex_direction: FlexDirection::Column,
                flex_grow: 1.,
                ..default()
            },
            children![(
                Node {
                    align_self: AlignSelf::Center,
                    margin: UiRect::px(4., 4., 2., 2.),
                    ..default()
                },
                Text("Vista Ui Editor".to_owned()),
                theme.typography.body_large.font.clone(),
                TextColor(editor_title_text(theme)),
            )],
        ));
    });
}

fn register_title_bar_draggable_observer(
    mut commands: Commands,
    title_bar: Single<Entity, With<TitleBar>>,
) {
    commands
        .entity(*title_bar)
        .observe(title_draggable::on_drag);
}

fn register_editor_foldable_observer(
    mut commands: Commands,
    fold_btn: Single<Entity, With<TitleFoldButton>>,
) {
    commands
        .entity(*fold_btn)
        .observe(foldable::on_over)
        .observe(foldable::on_out)
        .observe(foldable::on_click);
}

fn toggle_vista_editor_overlay_display(
    active: Res<VistaEditorActive>,
    mut overlay: Single<&mut Node, With<EditorRoot>>,
) {
    overlay.display = if **active {
        Display::Flex
    } else {
        Display::None
    }
}

fn sync_vista_editor_mode(
    mode: Res<VistaEditorMode>,
    mut expanded: ResMut<VistaEditorExpanded>,
    root: Single<(&mut Node, &mut UiTransform), With<EditorRoot>>,
    content: Single<&mut Node, (With<ContentRoot>, Without<EditorRoot>)>,
) {
    let (mut root_node, mut root_transform) = root.into_inner();
    let mut content_node = content.into_inner();

    match *mode {
        VistaEditorMode::Fullscreen => {
            **expanded = true;
            root_node.left = px(0.0);
            root_node.top = px(0.0);
            root_node.right = px(0.0);
            root_node.bottom = px(0.0);
            root_transform.translation = Val2::ZERO;

            content_node.width = percent(100.0);
            content_node.height = Val::Auto;
            content_node.flex_grow = 1.0;
            content_node.flex_shrink = 1.0;
            content_node.display = Display::Flex;
        }
        VistaEditorMode::Floating => {
            root_node.left = Val::Auto;
            root_node.top = Val::Auto;
            root_node.right = Val::Auto;
            root_node.bottom = Val::Auto;
            root_transform.translation = Val2::px(20.0, 20.0);

            content_node.width = px(800.0);
            content_node.height = px(500.0);
            content_node.flex_grow = 0.0;
            content_node.flex_shrink = 0.0;
            content_node.display = if **expanded {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}
