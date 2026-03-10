use bevy::{asset::{load_internal_asset, uuid_handle}, ecs::query::QueryFilter, render::render_resource::AsBindGroup, shader::ShaderRef};

use crate::theme::resolve_theme_or_global;

use super::*;

pub struct ColorFieldPlugin;

const COLOR_FIELD_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("9f7dc4bc-2e8e-4709-b215-cdfcd0e1c8f1");
const COLOR_FIELD_PALETTE_KIND: f32 = 0.0;
const COLOR_FIELD_HUE_KIND: f32 = 1.0;
const COLOR_FIELD_ALPHA_KIND: f32 = 2.0;
const COLOR_FIELD_PALETTE_SIZE: f32 = 180.0;
const COLOR_FIELD_HUE_WIDTH: f32 = 18.0;
const COLOR_FIELD_ALPHA_HEIGHT: f32 = 16.0;
const COLOR_FIELD_CURSOR_SIZE: f32 = 12.0;
const COLOR_FIELD_BAR_CURSOR_THICKNESS: f32 = 3.0;
const COLOR_FIELD_POPUP_HEIGHT_HINT: f32 = 300.0;

impl Plugin for ColorFieldPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            COLOR_FIELD_SHADER_HANDLE,
            "color_field.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(UiMaterialPlugin::<ColorFieldUiMaterial>::default())
            .add_message::<ColorFieldChange>()
            .add_systems(
                PostUpdate,
                (
                    apply_color_field_mode_changes,
                    apply_color_field_rgba_changes,
                    cleanup_orphaned_color_field_popups,
                    close_color_fields_on_outside_click,
                    sync_color_field_popup_presence,
                    sync_color_field_popup_layout,
                    sync_color_field_visuals,
                    sync_color_field_interaction,
                ),
            );
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct ColorFieldUiMaterial {
    #[uniform(0)]
    pub params0: Vec4,
    #[uniform(1)]
    pub params1: Vec4,
}

impl ColorFieldUiMaterial {
    fn palette(hue: f32, alpha: f32) -> Self {
        Self {
            params0: Vec4::new(COLOR_FIELD_PALETTE_KIND, hue, 0.0, 0.0),
            params1: Vec4::new(alpha, 0.0, 0.0, 0.0),
        }
    }

    fn hue() -> Self {
        Self {
            params0: Vec4::new(COLOR_FIELD_HUE_KIND, 0.0, 0.0, 0.0),
            params1: Vec4::ZERO,
        }
    }

    fn alpha(color: Color) -> Self {
        let srgb = color.to_srgba();
        Self {
            params0: Vec4::new(COLOR_FIELD_ALPHA_KIND, 0.0, 0.0, 0.0),
            params1: Vec4::new(1.0, srgb.red, srgb.green, srgb.blue),
        }
    }
}

impl UiMaterial for ColorFieldUiMaterial {
    fn fragment_shader() -> ShaderRef {
        COLOR_FIELD_SHADER_HANDLE.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ColorFieldMode {
    Rgba,
    RgbaU8,
    Hsla,
}

impl Default for ColorFieldMode {
    fn default() -> Self {
        Self::Rgba
    }
}

#[derive(Component, Reflect, Clone, Widget, ShowInInspector)]
#[widget("input/color_field", children = "exact(0)")]
#[builder(ColorFieldBuilder)]
pub struct ColorField {
    #[property(label = "Color")]
    pub color: Color,
    #[property(label = "Mode")]
    pub mode: ColorFieldMode,
    #[property(hidden)]
    pub expanded: bool,
    #[property(label = "Disabled")]
    pub disabled: bool,
}

impl Default for ColorField {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            mode: ColorFieldMode::Rgba,
            expanded: false,
            disabled: false,
        }
    }
}

#[derive(Message, EntityEvent)]
pub struct ColorFieldChange {
    pub entity: Entity,
    pub color: Color,
}

#[derive(Component)]
struct ColorFieldHeader;

#[derive(Component)]
struct ColorFieldSwatch;

#[derive(Component)]
struct ColorFieldSummaryLabel;

#[derive(Component)]
struct ColorFieldPopup;

#[derive(Component)]
struct ColorFieldOwnedBy(Entity);

#[derive(Component)]
struct ColorFieldRgbaField {
    index: usize,
}

#[derive(Component)]
struct ColorFieldModeDropdown;

#[derive(Component)]
struct ColorFieldPaletteArea;

#[derive(Component)]
struct ColorFieldHueArea;

#[derive(Component)]
struct ColorFieldAlphaArea;

#[derive(Component)]
struct ColorFieldPaletteCursor;

#[derive(Component)]
struct ColorFieldHueCursor;

#[derive(Component)]
struct ColorFieldAlphaCursor;

#[derive(Component, Clone)]
struct ColorFieldParts {
    header: Entity,
    swatch: Entity,
    summary_label: Entity,
    popup: Option<Entity>,
    palette_area: Option<Entity>,
    hue_area: Option<Entity>,
    alpha_area: Option<Entity>,
    palette_material: Option<Entity>,
    hue_material: Option<Entity>,
    alpha_material: Option<Entity>,
    palette_cursor: Option<Entity>,
    hue_cursor: Option<Entity>,
    alpha_cursor: Option<Entity>,
    mode_dropdown: Option<Entity>,
    rgba_fields: [Option<Entity>; 4],
    rgba_field_labels: [Option<Entity>; 4],
}

#[derive(Component, Copy, Clone)]
struct ColorFieldColors {
    normal_bg: Color,
    hovered_bg: Color,
    pressed_bg: Color,
    border: Color,
    text: Color,
    popup_bg: Color,
    disabled_bg: Color,
    chrome: Color,
}

#[derive(Clone)]
pub struct ColorFieldBuilder {
    field: ColorField,
    width: Val,
}

impl Default for ColorFieldBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorFieldBuilder {
    pub fn new() -> Self {
        Self {
            field: ColorField::default(),
            width: px(180.0),
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.field.color = color;
        self
    }

    pub fn mode(mut self, mode: ColorFieldMode) -> Self {
        self.field.mode = mode;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.field.disabled = disabled;
        self
    }

    pub fn width(mut self, width: Val) -> Self {
        self.width = width;
        self
    }

    pub fn build(self, commands: &mut Commands, theme: Option<&Theme>) -> Entity {
        let colors = color_field_colors(theme);
        let font = theme
            .map(|t| t.typography.body_medium.font.clone())
            .unwrap_or(TextFont::from_font_size(13.0));
        let root = commands
            .spawn((
                Node {
                    width: self.width,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                self.field.clone(),
                colors,
            ))
            .id();

        let swatch = commands
            .spawn((
                Name::new("Color Field Swatch"),
                Node {
                    width: px(18.0),
                    height: px(18.0),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(self.field.color),
                BorderColor::all(colors.border),
                BorderRadius::all(px(4.0)),
                ColorFieldSwatch,
                ColorFieldOwnedBy(root),
            ))
            .id();

        let summary_label = commands
            .spawn((
                Name::new("Color Field Summary"),
                Text::new(color_summary(self.field.color, self.field.mode)),
                TextLayout::new_with_no_wrap(),
                font.clone(),
                TextColor(colors.text),
                Node {
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                ColorFieldSummaryLabel,
                ColorFieldOwnedBy(root),
            ))
            .id();

        let header = commands
            .spawn((
                Name::new("Color Field Header"),
                Button,
                Interaction::default(),
                Node {
                    width: percent(100.0),
                    min_height: px(28.0),
                    padding: UiRect::axes(px(8.0), px(4.0)),
                    align_items: AlignItems::Center,
                    column_gap: px(8.0),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(colors.normal_bg),
                BorderColor::all(colors.border),
                BorderRadius::all(px(4.0)),
                colors,
                ColorFieldHeader,
                ColorFieldOwnedBy(root),
            ))
            .add_children(&[swatch, summary_label])
            .observe(on_color_field_header_click)
            .id();

        commands.entity(root).insert(ColorFieldParts {
            header,
            swatch,
            summary_label,
            popup: None,
            palette_area: None,
            hue_area: None,
            alpha_area: None,
            palette_material: None,
            hue_material: None,
            alpha_material: None,
            palette_cursor: None,
            hue_cursor: None,
            alpha_cursor: None,
            mode_dropdown: None,
            rgba_fields: [None; 4],
            rgba_field_labels: [None; 4],
        });
        commands.entity(root).add_child(header);
        root
    }
}

impl DefaultWidgetBuilder for ColorFieldBuilder {
    fn spawn_default(
        commands: &mut Commands,
        theme: Option<&crate::theme::Theme>,
    ) -> WidgetSpawnResult {
        ColorFieldBuilder::new().build(commands, theme).into()
    }
}

fn on_color_field_header_click(
    mut event: On<Pointer<Click>>,
    owners: Query<&ColorFieldOwnedBy>,
    mut fields: Query<&mut ColorField>,
) {
    let Ok(owner) = owners.get(event.entity) else {
        return;
    };
    let Ok(mut field) = fields.get_mut(owner.0) else {
        return;
    };
    if field.disabled {
        return;
    }
    field.expanded = !field.expanded;
    event.propagate(false);
}
fn on_color_field_palette_click(
    mut event: On<Pointer<Click>>,
    window: Single<&Window>,
    palette_areas: Query<
        (&ColorFieldOwnedBy, &ComputedNode, &UiGlobalTransform),
        With<ColorFieldPaletteArea>,
    >,
    mut fields: Query<&mut ColorField>,
    mut out: MessageWriter<ColorFieldChange>,
) {
    apply_color_field_picker(
        ColorFieldPickerTarget::Palette,
        event.event_target(),
        &window,
        &palette_areas,
        &mut fields,
        &mut out,
    );
    event.propagate(false);
}

fn on_color_field_palette_drag_start(
    mut event: On<Pointer<DragStart>>,
    window: Single<&Window>,
    palette_areas: Query<
        (&ColorFieldOwnedBy, &ComputedNode, &UiGlobalTransform),
        With<ColorFieldPaletteArea>,
    >,
    mut fields: Query<&mut ColorField>,
    mut out: MessageWriter<ColorFieldChange>,
) {
    apply_color_field_picker(
        ColorFieldPickerTarget::Palette,
        event.event_target(),
        &window,
        &palette_areas,
        &mut fields,
        &mut out,
    );
    event.propagate(false);
}

fn on_color_field_palette_drag(
    mut event: On<Pointer<Drag>>,
    window: Single<&Window>,
    palette_areas: Query<
        (&ColorFieldOwnedBy, &ComputedNode, &UiGlobalTransform),
        With<ColorFieldPaletteArea>,
    >,
    mut fields: Query<&mut ColorField>,
    mut out: MessageWriter<ColorFieldChange>,
) {
    apply_color_field_picker(
        ColorFieldPickerTarget::Palette,
        event.event_target(),
        &window,
        &palette_areas,
        &mut fields,
        &mut out,
    );
    event.propagate(false);
}

fn on_color_field_hue_click(
    mut event: On<Pointer<Click>>,
    window: Single<&Window>,
    hue_areas: Query<
        (&ColorFieldOwnedBy, &ComputedNode, &UiGlobalTransform),
        With<ColorFieldHueArea>,
    >,
    mut fields: Query<&mut ColorField>,
    mut out: MessageWriter<ColorFieldChange>,
) {
    apply_color_field_picker(
        ColorFieldPickerTarget::Hue,
        event.event_target(),
        &window,
        &hue_areas,
        &mut fields,
        &mut out,
    );
    event.propagate(false);
}

fn on_color_field_hue_drag_start(
    mut event: On<Pointer<DragStart>>,
    window: Single<&Window>,
    hue_areas: Query<
        (&ColorFieldOwnedBy, &ComputedNode, &UiGlobalTransform),
        With<ColorFieldHueArea>,
    >,
    mut fields: Query<&mut ColorField>,
    mut out: MessageWriter<ColorFieldChange>,
) {
    apply_color_field_picker(
        ColorFieldPickerTarget::Hue,
        event.event_target(),
        &window,
        &hue_areas,
        &mut fields,
        &mut out,
    );
    event.propagate(false);
}

fn on_color_field_hue_drag(
    mut event: On<Pointer<Drag>>,
    window: Single<&Window>,
    hue_areas: Query<
        (&ColorFieldOwnedBy, &ComputedNode, &UiGlobalTransform),
        With<ColorFieldHueArea>,
    >,
    mut fields: Query<&mut ColorField>,
    mut out: MessageWriter<ColorFieldChange>,
) {
    apply_color_field_picker(
        ColorFieldPickerTarget::Hue,
        event.event_target(),
        &window,
        &hue_areas,
        &mut fields,
        &mut out,
    );
    event.propagate(false);
}

fn on_color_field_alpha_click(
    mut event: On<Pointer<Click>>,
    window: Single<&Window>,
    alpha_areas: Query<
        (&ColorFieldOwnedBy, &ComputedNode, &UiGlobalTransform),
        With<ColorFieldAlphaArea>,
    >,
    mut fields: Query<&mut ColorField>,
    mut out: MessageWriter<ColorFieldChange>,
) {
    apply_color_field_picker(
        ColorFieldPickerTarget::Alpha,
        event.event_target(),
        &window,
        &alpha_areas,
        &mut fields,
        &mut out,
    );
    event.propagate(false);
}

fn on_color_field_alpha_drag_start(
    mut event: On<Pointer<DragStart>>,
    window: Single<&Window>,
    alpha_areas: Query<
        (&ColorFieldOwnedBy, &ComputedNode, &UiGlobalTransform),
        With<ColorFieldAlphaArea>,
    >,
    mut fields: Query<&mut ColorField>,
    mut out: MessageWriter<ColorFieldChange>,
) {
    apply_color_field_picker(
        ColorFieldPickerTarget::Alpha,
        event.event_target(),
        &window,
        &alpha_areas,
        &mut fields,
        &mut out,
    );
    event.propagate(false);
}

fn on_color_field_alpha_drag(
    mut event: On<Pointer<Drag>>,
    window: Single<&Window>,
    alpha_areas: Query<
        (&ColorFieldOwnedBy, &ComputedNode, &UiGlobalTransform),
        With<ColorFieldAlphaArea>,
    >,
    mut fields: Query<&mut ColorField>,
    mut out: MessageWriter<ColorFieldChange>,
) {
    apply_color_field_picker(
        ColorFieldPickerTarget::Alpha,
        event.event_target(),
        &window,
        &alpha_areas,
        &mut fields,
        &mut out,
    );
    event.propagate(false);
}

fn apply_color_field_mode_changes(
    mut changes: MessageReader<DropdownChange>,
    owners: Query<&ColorFieldOwnedBy, With<ColorFieldModeDropdown>>,
    mut fields: Query<&mut ColorField>,
) {
    for change in changes.read() {
        let Ok(owner) = owners.get(change.entity) else {
            continue;
        };
        let Ok(mut field) = fields.get_mut(owner.0) else {
            continue;
        };
        if field.disabled {
            continue;
        }
        field.mode = match change.selected {
            1 => ColorFieldMode::RgbaU8,
            2 => ColorFieldMode::Hsla,
            _ => ColorFieldMode::Rgba,
        };
    }
}

fn apply_color_field_rgba_changes(
    mut changes: MessageReader<NumberFieldChange>,
    rgba_fields: Query<(&ColorFieldRgbaField, &ColorFieldOwnedBy)>,
    mut fields: Query<&mut ColorField>,
    mut out: MessageWriter<ColorFieldChange>,
) {
    for change in changes.read() {
        let Ok((channel, owner)) = rgba_fields.get(change.entity) else {
            continue;
        };
        let Ok(mut field) = fields.get_mut(owner.0) else {
            continue;
        };
        if field.disabled {
            continue;
        }
        let mut values = color_mode_components(field.color, field.mode);
        let ranges = color_mode_ranges(field.mode);
        values[channel.index] = change
            .value
            .clamp(ranges[channel.index].0 as f64, ranges[channel.index].1 as f64)
            as f32;
        let next_color = color_from_mode_components(field.mode, values);
        if next_color != field.color {
            field.color = next_color;
            out.write(ColorFieldChange {
                entity: owner.0,
                color: next_color,
            });
        }
    }
}

fn sync_color_field_visuals(
    fields: Query<(&ColorField, &ColorFieldParts, &ColorFieldColors), Changed<ColorField>>,
    mut text_query: Query<&mut Text>,
    mut backgrounds: Query<&mut BackgroundColor>,
    mut materials: ResMut<Assets<ColorFieldUiMaterial>>,
    material_nodes: Query<&MaterialNode<ColorFieldUiMaterial>>,
    mut nodes: Query<&mut Node>,
    mut numeric_fields: Query<&mut NumberField>,
    mut dropdowns: Query<&mut Dropdown>,
) {
    for (field, parts, colors) in fields.iter() {
        if let Ok(mut bg) = backgrounds.get_mut(parts.swatch) {
            bg.0 = field.color;
        }
        if let Ok(mut bg) = backgrounds.get_mut(parts.header) {
            bg.0 = if field.disabled {
                colors.disabled_bg
            } else {
                colors.normal_bg
            };
        }
        if let Ok(mut text) = text_query.get_mut(parts.summary_label) {
            text.0 = color_summary(field.color, field.mode);
        }
        if let Some(mode_dropdown) = parts.mode_dropdown
            && let Ok(mut dropdown) = dropdowns.get_mut(mode_dropdown)
        {
            dropdown.options = color_mode_options();
            dropdown.selected = color_mode_selected(field.mode);
            dropdown.disabled = field.disabled;
        }

        let hsva = hsva_components(field.color);
        if let Some(palette_material) = parts.palette_material
            && let Ok(material_node) = material_nodes.get(palette_material)
            && let Some(material) = materials.get_mut(&material_node.0)
        {
            *material = ColorFieldUiMaterial::palette(hsva.hue / 360.0, hsva.alpha);
        }
        if let Some(alpha_material) = parts.alpha_material
            && let Ok(material_node) = material_nodes.get(alpha_material)
            && let Some(material) = materials.get_mut(&material_node.0)
        {
            *material = ColorFieldUiMaterial::alpha(color_without_alpha(field.color));
        }

        if let Some(cursor) = parts.palette_cursor
            && let Ok(mut node) = nodes.get_mut(cursor)
        {
            node.left =
                px(hsva.saturation * COLOR_FIELD_PALETTE_SIZE - COLOR_FIELD_CURSOR_SIZE * 0.5);
            node.top =
                px((1.0 - hsva.value) * COLOR_FIELD_PALETTE_SIZE - COLOR_FIELD_CURSOR_SIZE * 0.5);
        }
        if let Some(cursor) = parts.hue_cursor
            && let Ok(mut node) = nodes.get_mut(cursor)
        {
            node.top = px((1.0 - hsva.hue / 360.0) * COLOR_FIELD_PALETTE_SIZE
                - COLOR_FIELD_BAR_CURSOR_THICKNESS * 0.5);
        }
        if let Some(cursor) = parts.alpha_cursor
            && let Ok(mut node) = nodes.get_mut(cursor)
        {
            node.left =
                px(hsva.alpha * COLOR_FIELD_PALETTE_SIZE - COLOR_FIELD_BAR_CURSOR_THICKNESS * 0.5);
        }
        let values = color_mode_components(field.color, field.mode);
        let ranges = color_mode_ranges(field.mode);
        let labels = color_mode_labels(field.mode);
        for (index, field_entity) in parts.rgba_fields.iter().enumerate() {
            let Some(field_entity) = field_entity else {
                continue;
            };
            if let Ok(mut numeric) = numeric_fields.get_mut(*field_entity) {
                numeric.kind = color_mode_number_kind(field.mode, index);
                numeric.value = values[index] as f64;
                numeric.min = Some(ranges[index].0 as f64);
                numeric.max = Some(ranges[index].1 as f64);
                numeric.disabled = field.disabled;
            }
        }
        for (index, label_entity) in parts.rgba_field_labels.iter().enumerate() {
            let Some(label_entity) = label_entity else {
                continue;
            };
            if let Ok(mut text) = text_query.get_mut(*label_entity) {
                text.0 = labels[index].to_owned();
            }
        }
    }
}

fn sync_color_field_interaction(
    mut headers: Query<
        (
            &Interaction,
            &ColorFieldColors,
            &mut BackgroundColor,
            &ChildOf,
        ),
        (With<ColorFieldHeader>, Changed<Interaction>),
    >,
    fields: Query<&ColorField>,
) {
    for (interaction, colors, mut background, parent) in headers.iter_mut() {
        let Ok(field) = fields.get(parent.parent()) else {
            continue;
        };
        background.0 = if field.disabled {
            colors.disabled_bg
        } else {
            match *interaction {
                Interaction::Pressed => colors.pressed_bg,
                Interaction::Hovered => colors.hovered_bg,
                Interaction::None => colors.normal_bg,
            }
        };
    }
}
fn sync_color_field_popup_presence(
    mut commands: Commands,
    fields: Query<(Entity, &ColorField, &ColorFieldColors)>,
    mut parts_query: Query<&mut ColorFieldParts>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    popup_hosts: Query<(), With<PopupLayerHost>>,
    popup_roots: Query<(), With<PopupLayerRoot>>,
    global_popup_layer: Res<GlobalPopupLayerState>,
    scopes: Query<&ThemeScope>,
    boundaries: Query<(), With<ThemeBoundary>>,
    global_theme: Option<Res<Theme>>,
    mut ui_materials: ResMut<Assets<ColorFieldUiMaterial>>,
) {
    for (root, field, colors) in fields.iter() {
        let Ok(mut parts) = parts_query.get_mut(root) else {
            continue;
        };
        if !field.expanded {
            if let Some(existing_popup) = parts.popup.take() {
                commands.entity(existing_popup).despawn();
            }
            parts.palette_area = None;
            parts.hue_area = None;
            parts.alpha_area = None;
            parts.palette_material = None;
            parts.hue_material = None;
            parts.alpha_material = None;
            parts.palette_cursor = None;
            parts.hue_cursor = None;
            parts.alpha_cursor = None;
            parts.mode_dropdown = None;
            parts.rgba_fields = [None; 4];
            parts.rgba_field_labels = [None; 4];
            continue;
        }
        if parts.popup.is_some() {
            continue;
        }

        let popup_parent =
            resolve_popup_parent(root, &parents, &children, &popup_hosts, &popup_roots)
                .or(global_popup_layer.root)
                .unwrap_or_else(|| topmost_ancestor(root, &parents));
        let theme = resolve_theme_or_global(
            root,
            &parents,
            &scopes,
            &boundaries,
            global_theme.as_deref(),
        );

        let hsva = hsva_components(field.color);
        let palette_material =
            ui_materials.add(ColorFieldUiMaterial::palette(hsva.hue / 360.0, hsva.alpha));
        let hue_material = ui_materials.add(ColorFieldUiMaterial::hue());
        let alpha_material = ui_materials.add(ColorFieldUiMaterial::alpha(color_without_alpha(
            field.color,
        )));

        let palette_surface = commands
            .spawn((
                Name::new("Color Field Palette Surface"),
                MaterialNode(palette_material),
                Node {
                    width: percent(100.0),
                    height: percent(100.0),
                    ..default()
                },
                BorderRadius::all(px(6.0)),
                ColorFieldOwnedBy(root),
            ))
            .id();

        let palette_cursor = commands
            .spawn((
                Name::new("Color Field Palette Cursor"),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    top: px(0.0),
                    width: px(COLOR_FIELD_CURSOR_SIZE),
                    height: px(COLOR_FIELD_CURSOR_SIZE),
                    border: UiRect::all(px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::WHITE),
                BorderRadius::all(percent(50.0)),
                Outline::new(px(1.0), px(0.0), colors.chrome.with_alpha(0.8)),
                ColorFieldPaletteCursor,
                ColorFieldOwnedBy(root),
            ))
            .id();

        let palette_area = commands
            .spawn((
                Name::new("Color Field Palette Area"),
                Node {
                    width: px(COLOR_FIELD_PALETTE_SIZE),
                    height: px(COLOR_FIELD_PALETTE_SIZE),
                    position_type: PositionType::Relative,
                    overflow: Overflow::clip(),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(colors.chrome.with_alpha(0.18)),
                BorderColor::all(colors.border),
                BorderRadius::all(px(6.0)),
                ColorFieldPaletteArea,
                ColorFieldOwnedBy(root),
            ))
            .add_children(&[palette_surface, palette_cursor])
            .observe(on_color_field_palette_click)
            .observe(on_color_field_palette_drag_start)
            .observe(on_color_field_palette_drag)
            .id();

        let hue_surface = commands
            .spawn((
                Name::new("Color Field Hue Surface"),
                MaterialNode(hue_material),
                Node {
                    width: percent(100.0),
                    height: percent(100.0),
                    ..default()
                },
                BorderRadius::all(px(6.0)),
                ColorFieldOwnedBy(root),
            ))
            .id();

        let hue_cursor = commands
            .spawn((
                Name::new("Color Field Hue Cursor"),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(-3.0),
                    top: px(0.0),
                    width: px(COLOR_FIELD_HUE_WIDTH + 6.0),
                    height: px(COLOR_FIELD_BAR_CURSOR_THICKNESS),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                BorderRadius::all(px(2.0)),
                Outline::new(px(1.0), px(0.0), colors.chrome),
                ColorFieldHueCursor,
                ColorFieldOwnedBy(root),
            ))
            .id();

        let hue_area = commands
            .spawn((
                Name::new("Color Field Hue Area"),
                Node {
                    width: px(COLOR_FIELD_HUE_WIDTH),
                    height: px(COLOR_FIELD_PALETTE_SIZE),
                    position_type: PositionType::Relative,
                    overflow: Overflow::clip(),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(colors.chrome.with_alpha(0.18)),
                BorderColor::all(colors.border),
                BorderRadius::all(px(6.0)),
                ColorFieldHueArea,
                ColorFieldOwnedBy(root),
            ))
            .add_children(&[hue_surface, hue_cursor])
            .observe(on_color_field_hue_click)
            .observe(on_color_field_hue_drag_start)
            .observe(on_color_field_hue_drag)
            .id();

        let alpha_surface = commands
            .spawn((
                Name::new("Color Field Alpha Surface"),
                MaterialNode(alpha_material),
                Node {
                    width: percent(100.0),
                    height: percent(100.0),
                    ..default()
                },
                BorderRadius::all(px(6.0)),
                ColorFieldOwnedBy(root),
            ))
            .id();

        let alpha_cursor = commands
            .spawn((
                Name::new("Color Field Alpha Cursor"),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    top: px(-3.0),
                    width: px(COLOR_FIELD_BAR_CURSOR_THICKNESS),
                    height: px(COLOR_FIELD_ALPHA_HEIGHT + 6.0),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                BorderRadius::all(px(2.0)),
                Outline::new(px(1.0), px(0.0), colors.chrome),
                ColorFieldAlphaCursor,
                ColorFieldOwnedBy(root),
            ))
            .id();

        let alpha_area = commands
            .spawn((
                Name::new("Color Field Alpha Area"),
                Node {
                    width: px(COLOR_FIELD_PALETTE_SIZE),
                    height: px(COLOR_FIELD_ALPHA_HEIGHT),
                    position_type: PositionType::Relative,
                    overflow: Overflow::clip(),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(colors.chrome.with_alpha(0.18)),
                BorderColor::all(colors.border),
                BorderRadius::all(px(6.0)),
                ColorFieldAlphaArea,
                ColorFieldOwnedBy(root),
            ))
            .add_children(&[alpha_surface, alpha_cursor])
            .observe(on_color_field_alpha_click)
            .observe(on_color_field_alpha_drag_start)
            .observe(on_color_field_alpha_drag)
            .id();

        let mode_dropdown = DropdownBuilder::new()
            .width(percent(100.0))
            .options(color_mode_options())
            .selected(color_mode_selected(field.mode))
            .disabled(field.disabled)
            .build(&mut commands, theme);
        commands
            .entity(mode_dropdown)
            .insert((ColorFieldModeDropdown, ColorFieldOwnedBy(root)));

        let rgba_value_labels = color_mode_labels(field.mode);
        let rgba_values = color_mode_components(field.color, field.mode);
        let rgba_ranges = color_mode_ranges(field.mode);
        let mut rgba_fields = [None; 4];
        let mut rgba_field_labels = [None; 4];
        let rgba_row = commands
            .spawn((
                Name::new("Color Field RGBA Row"),
                Node {
                    width: percent(100.0),
                    column_gap: px(6.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
            .id();
        for index in 0..4 {
            let label_entity = commands
                .spawn((
                    Text::new(rgba_value_labels[index]),
                    TextLayout::new_with_no_wrap(),
                    TextFont::from_font_size(10.0),
                    TextColor(colors.text.with_alpha(0.75)),
                ))
                .id();
            let input = NumberFieldBuilder::new()
                .kind(color_mode_number_kind(field.mode, index))
                .width(px(52.0))
                .height(px(26.0))
                .value(rgba_values[index])
                .min(Some(rgba_ranges[index].0))
                .max(Some(rgba_ranges[index].1))
                .disabled(field.disabled)
                .build(&mut commands, theme);
            commands.entity(input).insert((
                Name::new(format!(
                    "Color Field RGBA Field {}",
                    rgba_value_labels[index]
                )),
                ColorFieldRgbaField { index },
                ColorFieldOwnedBy(root),
            ));
            rgba_fields[index] = Some(input);
            let group = commands
                .spawn((
                    Name::new(format!(
                        "Color Field RGBA Group {}",
                        rgba_value_labels[index]
                    )),
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(2.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .id();
            commands.entity(group).add_child(label_entity);
            commands.entity(group).add_child(input);
            commands.entity(rgba_row).add_child(group);
            rgba_field_labels[index] = Some(label_entity);
        }

        let picker_row = commands
            .spawn((
                Name::new("Color Field Picker Row"),
                Node {
                    width: percent(100.0),
                    column_gap: px(10.0),
                    ..default()
                },
            ))
            .add_children(&[palette_area, hue_area])
            .id();

        let popup = commands
            .spawn((
                Name::new("Color Field Popup"),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    top: px(32.0),
                    width: px(248.0),
                    padding: UiRect::all(px(10.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10.0),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(colors.popup_bg),
                BorderColor::all(colors.border),
                BorderRadius::all(px(8.0)),
                ZIndex(10),
                ColorFieldPopup,
                ColorFieldOwnedBy(root),
            ))
            .add_children(&[picker_row, alpha_area, mode_dropdown, rgba_row])
            .id();

        commands.entity(popup_parent).add_child(popup);
        parts.popup = Some(popup);
        parts.palette_area = Some(palette_area);
        parts.hue_area = Some(hue_area);
        parts.alpha_area = Some(alpha_area);
        parts.palette_material = Some(palette_surface);
        parts.hue_material = Some(hue_surface);
        parts.alpha_material = Some(alpha_surface);
        parts.palette_cursor = Some(palette_cursor);
        parts.hue_cursor = Some(hue_cursor);
        parts.alpha_cursor = Some(alpha_cursor);
        parts.mode_dropdown = Some(mode_dropdown);
        parts.rgba_fields = rgba_fields;
        parts.rgba_field_labels = rgba_field_labels;
    }
}

fn sync_color_field_popup_layout(
    mut commands: Commands,
    window: Single<&Window>,
    fields: Query<(Entity, &ColorFieldParts)>,
    header_layout: Query<(&ComputedNode, &UiGlobalTransform), With<ColorFieldHeader>>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    popup_hosts: Query<(), With<PopupLayerHost>>,
    popup_roots: Query<(), With<PopupLayerRoot>>,
    global_popup_layer: Res<GlobalPopupLayerState>,
    popup_layout: Query<&ComputedNode>,
    ui_transforms: Query<&UiGlobalTransform>,
    mut popup_nodes: Query<&mut Node, With<ColorFieldPopup>>,
) {
    let scale_factor = window.scale_factor();
    for (root, parts) in fields.iter() {
        let Some(popup) = parts.popup else {
            continue;
        };
        let Some(popup_parent) =
            resolve_popup_parent(root, &parents, &children, &popup_hosts, &popup_roots)
                .or(global_popup_layer.root)
                .or_else(|| Some(topmost_ancestor(root, &parents)))
        else {
            continue;
        };
        if parents
            .get(popup)
            .map(|parent| parent.parent() != popup_parent)
            .unwrap_or(true)
        {
            commands.entity(popup_parent).add_child(popup);
        }

        let Ok((header_node, header_transform)) = header_layout.get(parts.header) else {
            continue;
        };
        let Ok(layer_transform) = ui_transforms.get(popup_parent) else {
            continue;
        };
        let Ok(layer_node) = popup_layout.get(popup_parent) else {
            continue;
        };
        let Ok(mut popup_node) = popup_nodes.get_mut(popup) else {
            continue;
        };

        let header_origin = (header_transform.transform_point2(Vec2::ZERO).as_dvec2()
            / scale_factor as f64)
            .as_vec2();
        let layer_origin = (layer_transform.transform_point2(Vec2::ZERO).as_dvec2()
            / scale_factor as f64)
            .as_vec2();
        let header_size = header_node.size() / scale_factor;
        let layer_size = layer_node.size() / scale_factor;
        let header_top_left = header_origin - header_size * 0.5;
        let layer_top_left = layer_origin - layer_size * 0.5;
        let local_origin = header_top_left - layer_top_left;
        let popup_width = 248.0f32.min((layer_size.x - 16.0).max(220.0));
        let popup_height = COLOR_FIELD_POPUP_HEIGHT_HINT.min((layer_size.y - 16.0).max(180.0));
        let popup_left = local_origin
            .x
            .clamp(8.0, (layer_size.x - popup_width - 8.0).max(8.0));
        let space_below = layer_size.y - (local_origin.y + header_size.y) - 8.0;
        let space_above = local_origin.y - 8.0;
        let popup_top = if space_below < popup_height && space_above > space_below {
            (local_origin.y - popup_height - 6.0)
                .clamp(8.0, (layer_size.y - popup_height - 8.0).max(8.0))
        } else {
            (local_origin.y + header_size.y + 6.0)
                .clamp(8.0, (layer_size.y - popup_height - 8.0).max(8.0))
        };

        popup_node.left = px(popup_left);
        popup_node.top = px(popup_top);
        popup_node.width = px(popup_width);
        popup_node.display = Display::Flex;
    }
}
fn close_color_fields_on_outside_click(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    mut fields: Query<(&mut ColorField, &ColorFieldParts)>,
    header_layout: Query<(&ComputedNode, &UiGlobalTransform), With<ColorFieldHeader>>,
    popup_layout: Query<(&ComputedNode, &UiGlobalTransform), With<ColorFieldPopup>>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.physical_cursor_position() else {
        return;
    };
    for (mut field, parts) in fields.iter_mut() {
        if !field.expanded {
            continue;
        }
        let header_hit = header_layout
            .get(parts.header)
            .ok()
            .is_some_and(|(node, transform)| node.contains_point(*transform, cursor));
        if header_hit {
            continue;
        }
        let popup_hit = parts.popup.is_some_and(|popup| {
            popup_layout
                .get(popup)
                .ok()
                .is_some_and(|(node, transform)| node.contains_point(*transform, cursor))
        });
        if popup_hit {
            continue;
        }
        field.expanded = false;
    }
}

fn cleanup_orphaned_color_field_popups(
    mut commands: Commands,
    popups: Query<(Entity, &ColorFieldOwnedBy), With<ColorFieldPopup>>,
    fields: Query<(), With<ColorField>>,
) {
    for (popup, owner) in popups.iter() {
        if fields.contains(owner.0) {
            continue;
        }
        commands.entity(popup).despawn();
    }
}

#[derive(Clone, Copy)]
enum ColorFieldPickerTarget {
    Palette,
    Hue,
    Alpha,
}

fn apply_color_field_picker<F: QueryFilter>(
    target: ColorFieldPickerTarget,
    entity: Entity,
    window: &Window,
    picker_nodes: &Query<(&ColorFieldOwnedBy, &ComputedNode, &UiGlobalTransform), F>,
    fields: &mut Query<&mut ColorField>,
    out: &mut MessageWriter<ColorFieldChange>,
) {
    let Some(cursor) = window.physical_cursor_position() else {
        return;
    };
    let Ok((owner, node, transform)) = picker_nodes.get(entity) else {
        return;
    };
    let Some(local) = transform
        .try_inverse()
        .map(|inverse| inverse.transform_point2(cursor))
    else {
        return;
    };
    let size = node.size();
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    let uv = Vec2::new(local.x / size.x + 0.5, local.y / size.y + 0.5).clamp(Vec2::ZERO, Vec2::ONE);
    let Ok(mut field) = fields.get_mut(owner.0) else {
        return;
    };
    if field.disabled {
        return;
    }
    let mut hsva = hsva_components(field.color);
    match target {
        ColorFieldPickerTarget::Palette => {
            hsva.saturation = uv.x.clamp(0.0, 1.0);
            hsva.value = (1.0 - uv.y).clamp(0.0, 1.0);
        }
        ColorFieldPickerTarget::Hue => {
            hsva.hue = ((1.0 - uv.y).clamp(0.0, 1.0) * 360.0).rem_euclid(360.0);
        }
        ColorFieldPickerTarget::Alpha => {
            hsva.alpha = uv.x.clamp(0.0, 1.0);
        }
    }
    let next_color: Color = hsva.into();
    if next_color != field.color {
        field.color = next_color;
        out.write(ColorFieldChange {
            entity: owner.0,
            color: next_color,
        });
    }
}

fn color_field_colors(theme: Option<&Theme>) -> ColorFieldColors {
    match theme {
        Some(t) => ColorFieldColors {
            normal_bg: t.palette.surface,
            hovered_bg: t.palette.surface_variant,
            pressed_bg: t.palette.outline_variant,
            border: t.palette.outline,
            text: t.palette.on_surface,
            popup_bg: t.palette.surface,
            disabled_bg: t.palette.disabled_container,
            chrome: t.palette.outline_variant,
        },
        None => ColorFieldColors {
            normal_bg: Color::srgb(0.15, 0.15, 0.15),
            hovered_bg: Color::srgb(0.2, 0.2, 0.2),
            pressed_bg: Color::srgb(0.25, 0.25, 0.25),
            border: Color::srgb(0.35, 0.35, 0.35),
            text: Color::srgb(0.9, 0.9, 0.9),
            popup_bg: Color::srgb(0.12, 0.12, 0.12),
            disabled_bg: Color::srgb(0.1, 0.1, 0.1),
            chrome: Color::srgb(0.28, 0.28, 0.28),
        },
    }
}

fn hsva_components(color: Color) -> Hsva {
    Hsva::from(color)
}

fn color_without_alpha(color: Color) -> Color {
    let srgb = color.to_srgba();
    Color::srgba(srgb.red, srgb.green, srgb.blue, 1.0)
}

fn color_summary(color: Color, mode: ColorFieldMode) -> String {
    let _ = mode;
    let srgb = color.to_srgba();
    let r = (srgb.red.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (srgb.green.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (srgb.blue.clamp(0.0, 1.0) * 255.0).round() as u8;
    let a = (srgb.alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

fn color_mode_options() -> Vec<String> {
    vec!["RGBA".to_owned(), "RGBA (u8)".to_owned(), "HSLA".to_owned()]
}

fn color_mode_selected(mode: ColorFieldMode) -> usize {
    match mode {
        ColorFieldMode::Rgba => 0,
        ColorFieldMode::RgbaU8 => 1,
        ColorFieldMode::Hsla => 2,
    }
}

fn color_mode_labels(mode: ColorFieldMode) -> [&'static str; 4] {
    match mode {
        ColorFieldMode::Rgba | ColorFieldMode::RgbaU8 => ["R", "G", "B", "A"],
        ColorFieldMode::Hsla => ["H", "S", "L", "A"],
    }
}

fn color_mode_number_kind(mode: ColorFieldMode, index: usize) -> NumberKind {
    match mode {
        ColorFieldMode::RgbaU8 if index < 3 => NumberKind::U8,
        _ => NumberKind::F32,
    }
}

fn color_mode_ranges(mode: ColorFieldMode) -> [(f32, f32); 4] {
    match mode {
        ColorFieldMode::Rgba => [(0.0, 1.0), (0.0, 1.0), (0.0, 1.0), (0.0, 1.0)],
        ColorFieldMode::RgbaU8 => [(0.0, 255.0), (0.0, 255.0), (0.0, 255.0), (0.0, 1.0)],
        ColorFieldMode::Hsla => [(0.0, 360.0), (0.0, 100.0), (0.0, 100.0), (0.0, 1.0)],
    }
}

fn color_mode_components(color: Color, mode: ColorFieldMode) -> [f32; 4] {
    match mode {
        ColorFieldMode::Rgba => {
            let srgb = color.to_srgba();
            [srgb.red, srgb.green, srgb.blue, srgb.alpha]
        }
        ColorFieldMode::RgbaU8 => {
            let srgb = color.to_srgba();
            [
                srgb.red * 255.0,
                srgb.green * 255.0,
                srgb.blue * 255.0,
                srgb.alpha,
            ]
        }
        ColorFieldMode::Hsla => {
            let hsl = Hsla::from(color);
            [
                hsl.hue,
                hsl.saturation * 100.0,
                hsl.lightness * 100.0,
                hsl.alpha,
            ]
        }
    }
}

fn color_from_mode_components(mode: ColorFieldMode, values: [f32; 4]) -> Color {
    match mode {
        ColorFieldMode::Rgba => Color::srgba(
            values[0].clamp(0.0, 1.0),
            values[1].clamp(0.0, 1.0),
            values[2].clamp(0.0, 1.0),
            values[3].clamp(0.0, 1.0),
        ),
        ColorFieldMode::RgbaU8 => Color::srgba(
            values[0].clamp(0.0, 255.0) / 255.0,
            values[1].clamp(0.0, 255.0) / 255.0,
            values[2].clamp(0.0, 255.0) / 255.0,
            values[3].clamp(0.0, 1.0),
        ),
        ColorFieldMode::Hsla => Color::hsla(
            values[0].rem_euclid(360.0),
            values[1].clamp(0.0, 100.0) / 100.0,
            values[2].clamp(0.0, 100.0) / 100.0,
            values[3].clamp(0.0, 1.0),
        ),
    }
}

fn topmost_ancestor(mut entity: Entity, parents: &Query<&ChildOf>) -> Entity {
    while let Ok(parent) = parents.get(entity) {
        entity = parent.parent();
    }
    entity
}
