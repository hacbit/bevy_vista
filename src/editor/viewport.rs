use bevy::asset::prelude::*;
use bevy::ecs::prelude::*;
use bevy::input::{ButtonInput, keyboard::KeyCode};
use bevy::ui::prelude::*;
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    window::PrimaryWindow,
};

use crate::grid::GridUiMaterial;
use crate::theme::{EditorTheme, ThemeBoundary, ThemeScope, ViewportThemeState};
use crate::widget::{Dropdown, DropdownBuilder, DropdownChange, LabelBuilder};

use super::*;

#[derive(Component)]
pub(crate) struct ViewportOverlay;

#[derive(Component)]
pub(crate) struct ElementsContainer;

#[derive(Component)]
pub(crate) struct ViewportToolbar;

#[derive(Component)]
pub(crate) struct ViewportThemeDropdown;

#[derive(Component)]
pub(crate) struct ViewportCanvas;

#[derive(Component)]
pub(crate) struct ViewportGridBackground;

#[derive(Component)]
pub(crate) struct CanvasTopSizeLabel;

#[derive(Component)]
pub(crate) struct CanvasLeftSizeLabel;

#[derive(Component)]
pub(crate) struct CanvasTopSizeText;

#[derive(Component)]
pub(crate) struct CanvasLeftSizeText;

#[derive(Component, Copy, Clone)]
pub(crate) struct CanvasResizeHandle {
    axis_x: bool,
    axis_y: bool,
}

#[derive(Component, Default)]
pub(crate) struct CanvasResizeDragState {
    dragging: bool,
    origin_width: f32,
    origin_height: f32,
}

#[derive(Component, Clone)]
pub(crate) struct CanvasWidgetInstance;

#[derive(Component, Default)]
pub(crate) struct CanvasWidgetDragState {
    dragging: bool,
}

#[derive(Component, Copy, Clone)]
pub(crate) struct CanvasWidgetHandle {
    owner: Entity,
}

#[derive(Component, Reflect)]
pub(crate) struct CanvasController {
    pub zoom_factor: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub zoom_speed: f32,
}

impl Default for CanvasController {
    fn default() -> Self {
        Self {
            zoom_factor: 1.0,
            min_zoom: 0.5,
            max_zoom: 1.5,
            zoom_speed: 0.1,
        }
    }
}

pub(super) fn init_viewport_panel(
    mut commands: Commands,
    viewport: Single<Entity, With<Viewport>>,
    mut ui_materials: ResMut<Assets<GridUiMaterial>>,
    editor_theme: Res<EditorTheme>,
    viewport_theme: Res<ViewportThemeState>,
    canvas_info: Res<VistaEditorCanvasInfo>,
) {
    let theme = &editor_theme.0;
    commands
        .entity(*viewport)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_direction = FlexDirection::Column;
        });

    let theme_label = commands
        .spawn((
            Name::new("Viewport Theme Label"),
            LabelBuilder::new()
                .text("Theme")
                .font_size(theme.typography.body_medium.font.font_size)
                .color(theme.palette.on_surface)
                .build(),
        ))
        .id();
    let theme_dropdown = DropdownBuilder::new()
        .width(px(180.0))
        .options(viewport_theme.options())
        .selected(viewport_theme.selected_index())
        .build(&mut commands, Some(theme));
    commands
        .entity(theme_dropdown)
        .insert((Name::new("Viewport Theme Dropdown"), ViewportThemeDropdown));

    let toolbar = commands
        .spawn((
            Name::new("Viewport Toolbar"),
            Node {
                width: percent(100.0),
                min_height: px(36.0),
                padding: UiRect::axes(px(8.0), px(6.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                border: UiRect::bottom(px(1.0)),
                ..default()
            },
            BackgroundColor(theme.palette.surface),
            BorderColor::all(theme.palette.outline_variant),
            ViewportToolbar,
        ))
        .add_children(&[theme_label, theme_dropdown])
        .id();

    let overlay = commands
        .spawn((
            Name::new("Viewport Overlay"),
            Node {
                border: UiRect::all(px(2.0)),
                width: percent(100.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                overflow: Overflow::clip(),
                position_type: PositionType::Relative,
                ..default()
            },
            ViewportOverlay,
            BorderColor::all(theme.palette.outline_variant),
        ))
        .id();

    let viewport_grid = commands
        .spawn((
            Name::new("Viewport Grid Background"),
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
            MaterialNode(ui_materials.add(GridUiMaterial::default())),
            ViewportGridBackground,
        ))
        .id();

    let top_label_text = commands
        .spawn((
            Name::new("Canvas Top Size Text"),
            Text::new(format!("WIDTH: {:.0} px", canvas_info.canvas_size.x)),
            TextLayout::new_with_no_wrap(),
            TextFont {
                font: theme.typography.title_medium.font.font.clone(),
                font_size: 18.0,
                ..default()
            },
            TextColor(theme.palette.on_surface_muted.with_alpha(0.9)),
            CanvasTopSizeText,
        ))
        .id();

    let top_ruler = commands
        .spawn((
            Name::new("Canvas Top Size Label"),
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                top: px(0.0),
                width: px(180.0),
                height: px(26.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            CanvasTopSizeLabel,
        ))
        .add_child(top_label_text)
        .id();

    let left_label_text = commands
        .spawn((
            Name::new("Canvas Left Size Text"),
            Text::new(format!("HEIGHT: {:.0} px", canvas_info.canvas_size.y)),
            TextLayout::new_with_no_wrap(),
            TextFont {
                font: theme.typography.title_medium.font.font.clone(),
                font_size: 18.0,
                ..default()
            },
            TextColor(theme.palette.on_surface_muted.with_alpha(0.9)),
            UiTransform::from_rotation(Rot2::radians(-std::f32::consts::FRAC_PI_2)),
            CanvasLeftSizeText,
        ))
        .id();

    let left_ruler = commands
        .spawn((
            Name::new("Canvas Left Size Label"),
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                top: px(0.0),
                width: px(26.0),
                height: px(180.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            CanvasLeftSizeLabel,
        ))
        .add_child(left_label_text)
        .id();

    let viewport_canvas = commands
        .spawn((
            Name::new("Viewport Canvas"),
            Node {
                top: px(100.0),
                left: px(100.0),
                position_type: PositionType::Absolute,
                width: px(canvas_info.canvas_size.x.max(160.0)),
                height: px(canvas_info.canvas_size.y.max(120.0)),
                border: UiRect::all(px(1.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(theme.palette.outline),
            BackgroundColor(theme.palette.background),
            CanvasController::default(),
            CanvasResizeDragState::default(),
            ViewportCanvas,
        ))
        .id();

    let elements_container = commands
        .spawn((
            Name::new("Elements Container"),
            Node {
                left: px(0.0),
                top: px(0.0),
                width: percent(100.0),
                height: percent(100.0),
                padding: UiRect::all(px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8.0),
                position_type: PositionType::Absolute,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ElementsContainer,
            ThemeBoundary,
        ))
        .observe(on_elements_container_click)
        .id();
    if let Some(theme) = viewport_theme.active_theme() {
        commands
            .entity(elements_container)
            .insert(ThemeScope(theme.clone()));
    }

    let right_handle = spawn_canvas_resize_handle(
        &mut commands,
        "Canvas Resize Handle Right",
        Node {
            position_type: PositionType::Absolute,
            right: px(-3.0),
            top: px(0.0),
            bottom: px(12.0),
            width: px(6.0),
            ..default()
        },
        CanvasResizeHandle {
            axis_x: true,
            axis_y: false,
        },
    );
    let bottom_handle = spawn_canvas_resize_handle(
        &mut commands,
        "Canvas Resize Handle Bottom",
        Node {
            position_type: PositionType::Absolute,
            left: px(0.0),
            right: px(12.0),
            bottom: px(-3.0),
            height: px(6.0),
            ..default()
        },
        CanvasResizeHandle {
            axis_x: false,
            axis_y: true,
        },
    );
    let corner_handle = spawn_canvas_resize_handle(
        &mut commands,
        "Canvas Resize Handle Corner",
        Node {
            position_type: PositionType::Absolute,
            right: px(-4.0),
            bottom: px(-4.0),
            width: px(12.0),
            height: px(12.0),
            ..default()
        },
        CanvasResizeHandle {
            axis_x: true,
            axis_y: true,
        },
    );

    commands.entity(viewport_canvas).add_children(&[
        elements_container,
        right_handle,
        bottom_handle,
        corner_handle,
    ]);
    commands
        .entity(overlay)
        .add_children(&[viewport_grid, top_ruler, left_ruler, viewport_canvas]);
    commands.entity(*viewport).add_children(&[toolbar, overlay]);
}

pub(super) fn sync_viewport_theme_scope(
    mut commands: Commands,
    viewport_theme: Res<ViewportThemeState>,
    elements_container: Single<Entity, With<ElementsContainer>>,
) {
    if !viewport_theme.is_changed() {
        return;
    }

    commands.entity(*elements_container).remove::<ThemeScope>();
    if let Some(theme) = viewport_theme.active_theme() {
        commands
            .entity(*elements_container)
            .insert(ThemeScope(theme.clone()));
    }
}

pub(super) fn sync_viewport_canvas_defaults(
    canvas_info: Res<VistaEditorCanvasInfo>,
    grid: Res<VistaEditorGridInfo>,
    mut canvases: Query<&mut Node, With<ViewportCanvas>>,
    mut backgrounds: ResMut<Assets<GridUiMaterial>>,
    background_handles: Query<&MaterialNode<GridUiMaterial>, With<ViewportGridBackground>>,
) {
    if canvas_info.is_changed() {
        for mut canvas in canvases.iter_mut() {
            canvas.width = px(canvas_info.canvas_size.x.max(160.0));
            canvas.height = px(canvas_info.canvas_size.y.max(120.0));
        }
    }

    let grid_changed = grid.is_changed();

    if !grid_changed {
        return;
    }

    for material in background_handles.iter() {
        if let Some(grid_material) = backgrounds.get_mut(&material.0) {
            grid_material.grid_params.x = grid.cell_size.max(2.0);
            grid_material.grid_params.y = 1.0;
            grid_material.grid_params.z = 1.0;
            grid_material.grid_params.w = grid.major_frequency.max(1) as f32;
            grid_material.offset = Vec2::ZERO;
            grid_material.anti_alias = 1.0;
        }
    }
}

pub(super) fn sync_viewport_grid_material(
    editor_theme: Res<EditorTheme>,
    mut backgrounds: ResMut<Assets<GridUiMaterial>>,
    background_handles: Query<&MaterialNode<GridUiMaterial>, With<ViewportGridBackground>>,
) {
    if !editor_theme.is_changed() {
        return;
    }

    for material in background_handles.iter() {
        if let Some(grid_material) = backgrounds.get_mut(&material.0) {
            grid_material.grid_color_primary = editor_theme
                .0
                .palette
                .outline
                .with_alpha(0.42)
                .to_linear()
                .to_f32_array()
                .into();
            grid_material.grid_color_secondary = editor_theme
                .0
                .palette
                .outline_variant
                .with_alpha(0.22)
                .to_linear()
                .to_f32_array()
                .into();
        }
    }
}

pub(super) fn sync_viewport_grid_alignment(
    canvas: Single<(&Node, &ComputedNode, &UiTransform, &CanvasController), With<ViewportCanvas>>,
    mut backgrounds: ResMut<Assets<GridUiMaterial>>,
    background_handles: Query<&MaterialNode<GridUiMaterial>, With<ViewportGridBackground>>,
) {
    let (canvas_node, canvas_computed, canvas_transform, controller) = *canvas;
    let width = val_px_or_default(canvas_node.width, 800.0);
    let height = val_px_or_default(canvas_node.height, 600.0);
    let left = val_px_or_default(canvas_node.left, 0.0);
    let top = val_px_or_default(canvas_node.top, 0.0);
    let translation = match canvas_transform.translation {
        Val2 {
            x: Val::Px(x),
            y: Val::Px(y),
        } => Vec2::new(x, y),
        _ => Vec2::ZERO,
    };
    let scale = controller.zoom_factor.max(0.0001);

    // UiTransform scaling is centered on the canvas, so the rendered top-left
    // shifts by half of the scaled size delta.
    let scaled_origin = Vec2::new(
        left + translation.x - width * (scale - 1.0) * 0.5,
        top + translation.y - height * (scale - 1.0) * 0.5,
    );
    let scaled_origin_physical = scaled_origin * canvas_computed.inverse_scale_factor().recip();

    for material in background_handles.iter() {
        if let Some(grid_material) = backgrounds.get_mut(&material.0) {
            grid_material.offset = -scaled_origin_physical;
            grid_material.grid_params.z = scale;
        }
    }
}

pub(super) fn sync_viewport_toolbar(
    viewport_theme: Res<ViewportThemeState>,
    mut dropdowns: Query<&mut Dropdown, With<ViewportThemeDropdown>>,
) {
    if !viewport_theme.is_changed() {
        return;
    }

    let options = viewport_theme.options();
    let selected = viewport_theme.selected_index();
    for mut dropdown in dropdowns.iter_mut() {
        dropdown.options = options.clone();
        dropdown.selected = selected;
    }
}

pub(super) fn sync_canvas_size_label_positions(
    canvas: Single<
        (&Node, &UiTransform, &CanvasController),
        (
            With<ViewportCanvas>,
            Without<CanvasTopSizeLabel>,
            Without<CanvasLeftSizeLabel>,
        ),
    >,
    mut top_label: Single<&mut Node, (With<CanvasTopSizeLabel>, Without<CanvasLeftSizeLabel>)>,
    mut left_label: Single<&mut Node, (With<CanvasLeftSizeLabel>, Without<CanvasTopSizeLabel>)>,
) {
    let (canvas_node, canvas_transform, controller) = *canvas;
    let scale = controller.zoom_factor.max(0.0001);
    let width = val_px_or_default(canvas_node.width, 800.0).max(0.0);
    let height = val_px_or_default(canvas_node.height, 600.0).max(0.0);
    let left = val_px_or_default(canvas_node.left, 0.0);
    let top = val_px_or_default(canvas_node.top, 0.0);
    let translation = match canvas_transform.translation {
        Val2 {
            x: Val::Px(x),
            y: Val::Px(y),
        } => Vec2::new(x, y),
        _ => Vec2::ZERO,
    };
    let scaled_origin = Vec2::new(
        left + translation.x - width * (scale - 1.0) * 0.5,
        top + translation.y - height * (scale - 1.0) * 0.5,
    );
    let scaled_size = Vec2::new(width * scale, height * scale);

    **top_label = Node {
        position_type: PositionType::Absolute,
        left: px((scaled_origin.x + scaled_size.x * 0.5 - 32.0).max(0.0)),
        top: px((scaled_origin.y - 24.0).max(0.0)),
        width: px(64.0),
        height: px(22.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    **left_label = Node {
        position_type: PositionType::Absolute,
        left: px((scaled_origin.x - 40.0).max(0.0)),
        top: px((scaled_origin.y + scaled_size.y * 0.5 - 11.0).max(0.0)),
        width: px(44.0),
        height: px(22.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
}

pub(super) fn sync_canvas_size_label_text(
    canvas_info: Res<VistaEditorCanvasInfo>,
    mut top_text: Single<&mut Text, (With<CanvasTopSizeText>, Without<CanvasLeftSizeText>)>,
    mut left_text: Single<&mut Text, (With<CanvasLeftSizeText>, Without<CanvasTopSizeText>)>,
) {
    if !canvas_info.is_changed() {
        return;
    }

    top_text.0 = format!("WIDTH: {:.0} px", canvas_info.canvas_size.x.max(0.0));
    left_text.0 = format!("HEIGHT: {:.0} px", canvas_info.canvas_size.y.max(0.0));
}

pub(super) fn apply_viewport_toolbar_changes(
    mut changes: MessageReader<DropdownChange>,
    dropdowns: Query<(), With<ViewportThemeDropdown>>,
    mut viewport_theme: ResMut<ViewportThemeState>,
) {
    let mut next_selected = None;
    for change in changes.read() {
        if dropdowns.contains(change.entity) {
            next_selected = Some(change.selected);
        }
    }

    let Some(next_selected) = next_selected else {
        return;
    };
    if viewport_theme.selected_index() != next_selected {
        viewport_theme.set_selected_index(next_selected);
    }
}

pub(super) fn spawn_canvas_widget_instance(
    commands: &mut Commands,
    parent: Entity,
    content: Entity,
    widget_id: &str,
) -> Entity {
    let wrapper = commands
        .spawn((
            Name::new(format!("Widget Instance [{widget_id}]")),
            Node {
                width: percent(100.0),
                border: UiRect::all(px(1.0)),
                ..default()
            },
            BorderColor::all(Color::NONE),
            BorderRadius::all(px(4.0)),
            CanvasWidgetInstance,
            CanvasWidgetDragState::default(),
        ))
        .id();

    let handle = commands
        .spawn((
            Name::new("Canvas Widget Handle"),
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                right: px(0.0),
                top: px(0.0),
                bottom: px(0.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            CanvasWidgetHandle { owner: wrapper },
        ))
        .observe(on_canvas_widget_handle_click)
        .observe(on_canvas_widget_handle_drag_start)
        .observe(on_canvas_widget_handle_drag)
        .observe(on_canvas_widget_handle_drag_end)
        .observe(on_canvas_widget_handle_drag_cancel)
        .id();

    commands.entity(wrapper).add_children(&[content, handle]);
    commands.entity(parent).add_child(wrapper);
    wrapper
}

pub(super) fn toggle_preview_mode_with_key(
    key_input: Res<ButtonInput<KeyCode>>,
    mut options: ResMut<VistaEditorViewOptions>,
    mut selection: ResMut<VistaEditorSelection>,
) {
    if key_input.just_pressed(KeyCode::F5) {
        options.is_preview_mode = !options.is_preview_mode;
        if options.is_preview_mode {
            selection.selected_entity = None;
        }
    }
}

pub(super) fn update_canvas_widget_handles_for_mode(
    options: Res<VistaEditorViewOptions>,
    mut handles: Query<&mut Node, With<CanvasWidgetHandle>>,
) {
    if !options.is_changed() {
        return;
    }
    let display = if options.is_preview_mode {
        Display::None
    } else {
        Display::Flex
    };
    for mut node in handles.iter_mut() {
        node.display = display;
    }
}

pub(super) fn update_canvas_widget_selection_visual(
    selection: Res<VistaEditorSelection>,
    options: Res<VistaEditorViewOptions>,
    editor_theme: Res<EditorTheme>,
    mut widgets: Query<(Entity, &mut BorderColor), With<CanvasWidgetInstance>>,
) {
    if !selection.is_changed() && !options.is_changed() {
        return;
    }
    let selected = if options.is_preview_mode {
        None
    } else {
        selection.selected_entity
    };
    let selected_color = editor_theme.0.palette.primary;
    for (entity, mut border) in widgets.iter_mut() {
        border.set_all(if Some(entity) == selected {
            selected_color
        } else {
            Color::NONE
        });
    }
}

const SCROLL_UNIT_CONVERSION_FACTOR: f32 = 100.0;

pub(super) fn handle_canvas_zooming(
    mouse_scroll: Res<AccumulatedMouseScroll>,
    window: Single<&Window, With<PrimaryWindow>>,
    viewport: Query<(&ComputedNode, &UiGlobalTransform), With<ViewportOverlay>>,
    mut canvas: Query<(
        &Node,
        &mut UiTransform,
        &UiGlobalTransform,
        &mut CanvasController,
    )>,
) {
    let Ok((computed_node, global_transform)) = viewport.single() else {
        return;
    };

    // Check if mouse is inside viewport
    let Some(physical_cursor_pos) = window.physical_cursor_position() else {
        return;
    };
    if !computed_node.contains_point(*global_transform, physical_cursor_pos) {
        return;
    }

    let Ok((canvas_node, mut canvas_transform, canvas_global_trans, mut controller)) =
        canvas.single_mut()
    else {
        return;
    };

    // Zoom with mouse scroll
    let zoom_amount = match mouse_scroll.unit {
        MouseScrollUnit::Line => mouse_scroll.delta.y,
        MouseScrollUnit::Pixel => mouse_scroll.delta.y / SCROLL_UNIT_CONVERSION_FACTOR,
    } * controller.zoom_speed;

    let previous_zoom = controller.zoom_factor;
    controller.zoom_factor =
        (controller.zoom_factor + zoom_amount).clamp(controller.min_zoom, controller.max_zoom);
    let next_zoom = controller.zoom_factor;
    let applied_zoom = next_zoom - previous_zoom;
    canvas_transform.scale = Vec2::splat(controller.zoom_factor);

    // offset correction when scaling
    if applied_zoom == 0.0 {
        return;
    }
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let center_pos = (canvas_global_trans.transform_point2(Vec2::ZERO).as_dvec2()
        / window.scale_factor() as f64)
        .as_vec2();
    let zoom_ratio = next_zoom / previous_zoom.max(0.0001);
    let next_center_pos = cursor_pos - (cursor_pos - center_pos) * zoom_ratio;
    let center_delta = next_center_pos - center_pos;
    if let Val2 {
        x: Val::Px(left),
        y: Val::Px(top),
    } = canvas_transform.translation
    {
        let mut translation = Vec2::new(left + center_delta.x, top + center_delta.y);
        clamp_canvas_translation(
            canvas_node,
            &mut translation,
            controller.zoom_factor,
            computed_node.size() * computed_node.inverse_scale_factor(),
        );
        canvas_transform.translation = Val2 {
            x: px(translation.x),
            y: px(translation.y),
        };
    }
}

pub(super) fn handle_canvas_panning(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    window: Single<&Window, With<PrimaryWindow>>,
    viewport: Query<(&ComputedNode, &UiGlobalTransform), With<ViewportOverlay>>,
    mut canvas: Query<(&Node, &mut UiTransform, &CanvasController), With<ViewportCanvas>>,
) {
    if !(mouse_buttons.pressed(MouseButton::Middle) || mouse_buttons.pressed(MouseButton::Right))
        || mouse_motion.delta == Vec2::ZERO
    {
        return;
    }

    let Ok((viewport_node, viewport_transform)) = viewport.single() else {
        return;
    };
    let Some(cursor) = window.physical_cursor_position() else {
        return;
    };
    if !viewport_node.contains_point(*viewport_transform, cursor) {
        return;
    }

    let Ok((canvas_node, mut canvas_transform, controller)) = canvas.single_mut() else {
        return;
    };
    if let Val2 {
        x: Val::Px(left),
        y: Val::Px(top),
    } = canvas_transform.translation
    {
        let mut translation = Vec2::new(left + mouse_motion.delta.x, top + mouse_motion.delta.y);
        clamp_canvas_translation(
            canvas_node,
            &mut translation,
            controller.zoom_factor,
            viewport_node.size() * viewport_node.inverse_scale_factor(),
        );
        canvas_transform.translation = Val2::px(translation.x, translation.y);
    }
}

fn spawn_canvas_resize_handle(
    commands: &mut Commands,
    name: &str,
    node: Node,
    handle: CanvasResizeHandle,
) -> Entity {
    commands
        .spawn((
            Name::new(name.to_owned()),
            node,
            BackgroundColor(Color::NONE),
            handle,
        ))
        .observe(on_canvas_resize_drag_start)
        .observe(on_canvas_resize_drag)
        .observe(on_canvas_resize_drag_end)
        .observe(on_canvas_resize_drag_cancel)
        .id()
}

fn on_canvas_widget_handle_click(
    mut event: On<Pointer<Click>>,
    options: Res<VistaEditorViewOptions>,
    handles: Query<&CanvasWidgetHandle>,
    mut selection: ResMut<VistaEditorSelection>,
) {
    if options.is_preview_mode {
        return;
    }
    let Ok(handle) = handles.get(event.event_target()) else {
        return;
    };
    selection.selected_entity = Some(handle.owner);
    event.propagate(false);
}

fn on_canvas_widget_handle_drag_start(
    mut event: On<Pointer<DragStart>>,
    options: Res<VistaEditorViewOptions>,
    handles: Query<&CanvasWidgetHandle>,
    mut widgets: Query<(&Node, &mut CanvasWidgetDragState), With<CanvasWidgetInstance>>,
    mut selection: ResMut<VistaEditorSelection>,
) {
    if options.is_preview_mode {
        return;
    }
    let Ok(handle) = handles.get(event.event_target()) else {
        return;
    };
    let Ok((node, mut drag)) = widgets.get_mut(handle.owner) else {
        return;
    };
    if node.position_type != PositionType::Absolute {
        selection.selected_entity = Some(handle.owner);
        event.propagate(false);
        return;
    }
    drag.dragging = true;
    selection.selected_entity = Some(handle.owner);
    event.propagate(false);
}

fn on_canvas_widget_handle_drag(
    mut event: On<Pointer<Drag>>,
    options: Res<VistaEditorViewOptions>,
    handles: Query<&CanvasWidgetHandle>,
    mut widgets: Query<(&mut Node, &CanvasWidgetDragState), With<CanvasWidgetInstance>>,
) {
    if options.is_preview_mode {
        return;
    }
    let Ok(handle) = handles.get(event.event_target()) else {
        return;
    };
    let Ok((mut node, drag)) = widgets.get_mut(handle.owner) else {
        return;
    };
    if !drag.dragging || node.position_type != PositionType::Absolute {
        return;
    }
    if let Val::Px(left) = node.left {
        node.left = px(left + event.event().delta.x);
    }
    if let Val::Px(top) = node.top {
        node.top = px(top + event.event().delta.y);
    }
    event.propagate(false);
}

fn on_canvas_widget_handle_drag_end(
    mut event: On<Pointer<DragEnd>>,
    options: Res<VistaEditorViewOptions>,
    handles: Query<&CanvasWidgetHandle>,
    mut widgets: Query<&mut CanvasWidgetDragState, With<CanvasWidgetInstance>>,
) {
    if options.is_preview_mode {
        return;
    }
    let Ok(handle) = handles.get(event.event_target()) else {
        return;
    };
    if let Ok(mut drag) = widgets.get_mut(handle.owner) {
        drag.dragging = false;
        event.propagate(false);
    }
}

fn on_canvas_widget_handle_drag_cancel(
    mut event: On<Pointer<Cancel>>,
    options: Res<VistaEditorViewOptions>,
    handles: Query<&CanvasWidgetHandle>,
    mut widgets: Query<&mut CanvasWidgetDragState, With<CanvasWidgetInstance>>,
) {
    if options.is_preview_mode {
        return;
    }
    let Ok(handle) = handles.get(event.event_target()) else {
        return;
    };
    if let Ok(mut drag) = widgets.get_mut(handle.owner) {
        drag.dragging = false;
        event.propagate(false);
    }
}

fn on_canvas_resize_drag_start(
    mut event: On<Pointer<DragStart>>,
    handles: Query<&CanvasResizeHandle>,
    parents: Query<&ChildOf>,
    mut canvases: Query<(&Node, &mut CanvasResizeDragState), With<ViewportCanvas>>,
) {
    if handles.get(event.event_target()).is_err() {
        return;
    }
    let Ok(parent) = parents.get(event.event_target()) else {
        return;
    };
    let Ok((node, mut drag)) = canvases.get_mut(parent.parent()) else {
        return;
    };
    drag.dragging = true;
    drag.origin_width = val_px_or_default(node.width, 1280.0);
    drag.origin_height = val_px_or_default(node.height, 720.0);
    event.propagate(false);
}

fn on_canvas_resize_drag(
    mut event: On<Pointer<Drag>>,
    handles: Query<&CanvasResizeHandle>,
    parents: Query<&ChildOf>,
    mut canvases: Query<(&mut Node, &CanvasResizeDragState), With<ViewportCanvas>>,
    mut canvas_info: ResMut<VistaEditorCanvasInfo>,
) {
    let Ok(handle) = handles.get(event.event_target()) else {
        return;
    };
    let Ok(parent) = parents.get(event.event_target()) else {
        return;
    };
    let Ok((mut node, drag)) = canvases.get_mut(parent.parent()) else {
        return;
    };
    if !drag.dragging {
        return;
    }

    if handle.axis_x {
        let next_width = (drag.origin_width + event.event().distance.x).max(160.0);
        node.width = px(next_width);
        canvas_info.canvas_size.x = next_width;
    }
    if handle.axis_y {
        let next_height = (drag.origin_height + event.event().distance.y).max(120.0);
        node.height = px(next_height);
        canvas_info.canvas_size.y = next_height;
    }
    event.propagate(false);
}

fn on_canvas_resize_drag_end(
    mut event: On<Pointer<DragEnd>>,
    parents: Query<&ChildOf>,
    mut canvases: Query<&mut CanvasResizeDragState, With<ViewportCanvas>>,
) {
    let Ok(parent) = parents.get(event.event_target()) else {
        return;
    };
    if let Ok(mut drag) = canvases.get_mut(parent.parent()) {
        drag.dragging = false;
        event.propagate(false);
    }
}

fn on_canvas_resize_drag_cancel(
    mut event: On<Pointer<Cancel>>,
    parents: Query<&ChildOf>,
    mut canvases: Query<&mut CanvasResizeDragState, With<ViewportCanvas>>,
) {
    let Ok(parent) = parents.get(event.event_target()) else {
        return;
    };
    if let Ok(mut drag) = canvases.get_mut(parent.parent()) {
        drag.dragging = false;
        event.propagate(false);
    }
}

fn on_elements_container_click(
    mut event: On<Pointer<Click>>,
    options: Res<VistaEditorViewOptions>,
    mut selection: ResMut<VistaEditorSelection>,
) {
    if options.is_preview_mode {
        return;
    }
    selection.selected_entity = None;
    event.propagate(false);
}

fn val_px_or_default(v: Val, fallback: f32) -> f32 {
    match v {
        Val::Px(x) => x,
        _ => fallback,
    }
}

fn clamp_canvas_translation(
    canvas_node: &Node,
    translation: &mut Vec2,
    scale: f32,
    viewport_size: Vec2,
) {
    const VISIBLE_MARGIN: f32 = 48.0;

    let width = val_px_or_default(canvas_node.width, 800.0);
    let height = val_px_or_default(canvas_node.height, 600.0);
    let left = val_px_or_default(canvas_node.left, 0.0);
    let top = val_px_or_default(canvas_node.top, 0.0);

    let scaled_width = width * scale;
    let scaled_height = height * scale;
    let origin_shift = Vec2::new(width * (scale - 1.0) * 0.5, height * (scale - 1.0) * 0.5);

    let min_top_left = Vec2::new(
        VISIBLE_MARGIN - scaled_width,
        VISIBLE_MARGIN - scaled_height,
    );
    let max_top_left = Vec2::new(
        viewport_size.x - VISIBLE_MARGIN,
        viewport_size.y - VISIBLE_MARGIN,
    );

    let current_top_left = Vec2::new(left, top) + *translation - origin_shift;
    let clamped_top_left = current_top_left.clamp(min_top_left, max_top_left);
    *translation = clamped_top_left - Vec2::new(left, top) + origin_shift;
}
