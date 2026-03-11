use arboard::Clipboard;
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    text::TextLayoutInfo,
    ui::RelativeCursorPosition,
    window::PrimaryWindow,
};

use crate::theme::resolve_theme_or_global;

use super::*;

pub struct TextFieldPlugin;

impl Plugin for TextFieldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FocusedTextField>()
            .init_resource::<TextFieldCursorBlink>()
            .init_resource::<TextClipboard>()
            .add_message::<TextInputChange>()
            .add_message::<TextInputSubmit>()
            .add_systems(
                PostUpdate,
                (
                    text_field_focus,
                    text_field_click_to_move,
                    text_field_drag_select,
                    text_field_ime_state,
                    text_field_input,
                    text_field_ime_input,
                    text_field_layout_behavior,
                    text_field_cursor_blink,
                    text_field_sync_visuals,
                )
                    .chain(),
            );
    }
}

#[derive(Component, Default, Reflect, Clone, Widget, ShowInInspector)]
#[widget("input/text_field", children = "exact(0)")]
#[builder(TextFieldBuilder)]
pub struct TextField {
    #[property(hidden)]
    pub value: String,
    #[property(hidden)]
    pub placeholder: String,
    #[property(hidden)]
    pub max_length: Option<usize>,
    #[property(hidden)]
    pub validator: Option<TextInputValidator>,
    #[property(hidden)]
    pub formatter: Option<TextInputFormatter>,
    #[property(hidden)]
    pub cursor_pos: usize,
    #[property(hidden)]
    pub selection: Option<(usize, usize)>,
    #[property(hidden)]
    pub drag_anchor: Option<usize>,
    #[property(label = "Disabled")]
    pub disabled: bool,
    #[property(label = "Input Type")]
    pub input_type: TextInputType,
    #[property(label = "Layout Mode")]
    pub layout_mode: TextFieldLayoutMode,
    #[property(label = "Min Width", min = 0.0)]
    pub min_width: f32,
    #[property(label = "Min Height", min = 0.0)]
    pub min_height: f32,
}

#[derive(Component)]
struct TextFieldCursor;

#[derive(Component)]
struct TextFieldInputText;

#[derive(Component)]
struct TextFieldSelectionHighlight;

#[derive(Reflect, Clone)]
pub enum TextInputValidator {
    Any,
    Numeric,
    Alpha,
    AlphaNum,
    Email,
}

#[derive(Reflect, Clone)]
pub enum TextInputFormatter {
    Uppercase,
    Lowercase,
    Capitalize,
    Trim,
}

#[derive(Reflect, Default, Clone)]
pub enum TextInputType {
    #[default]
    SingleLine,
    MultiLine,
    Password,
    Search,
    Email,
    Number,
}

#[derive(Reflect, Clone, Copy, Default)]
pub enum TextFieldLayoutMode {
    #[default]
    FixedTruncate,
    AutoWidth,
    AutoWrap,
    MultiLine,
}

#[derive(Message, EntityEvent)]
pub struct TextInputChange {
    pub entity: Entity,
    pub value: String,
}

#[derive(Message, EntityEvent)]
pub struct TextInputSubmit {
    pub entity: Entity,
    pub value: String,
}

#[derive(Resource, Default)]
struct FocusedTextField(pub Option<Entity>);

#[derive(Resource)]
struct TextFieldCursorBlink {
    timer: Timer,
    visible: bool,
}

impl Default for TextFieldCursorBlink {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
            visible: true,
        }
    }
}

#[derive(Resource)]
struct TextClipboard {
    os_clipboard: Option<Clipboard>,
    fallback: String,
}

impl FromWorld for TextClipboard {
    fn from_world(_: &mut World) -> Self {
        Self {
            os_clipboard: Clipboard::new().ok(),
            fallback: String::new(),
        }
    }
}

pub struct TextFieldBuilder {
    text: String,
    placeholder: String,
    max_length: Option<usize>,
    width: Val,
    height: Val,
    disabled: bool,
    validator: Option<TextInputValidator>,
    formatter: Option<TextInputFormatter>,
    layout_mode: TextFieldLayoutMode,
}

impl Default for TextFieldBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TextFieldBuilder {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            placeholder: "Enter text...".to_owned(),
            max_length: Some(256),
            width: Val::Px(200.0),
            height: Val::Px(30.0),
            disabled: false,
            validator: None,
            formatter: None,
            layout_mode: TextFieldLayoutMode::FixedTruncate,
        }
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = text.to_string();
        self
    }

    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.to_string();
        self
    }

    pub fn max_length(mut self, max_length: Option<usize>) -> Self {
        self.max_length = max_length;
        self
    }

    pub fn width(mut self, width: Val) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Val) -> Self {
        self.height = height;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn validator(mut self, validator: TextInputValidator) -> Self {
        self.validator = Some(validator);
        self
    }

    pub fn formatter(mut self, formatter: TextInputFormatter) -> Self {
        self.formatter = Some(formatter);
        self
    }

    pub fn layout_mode(mut self, mode: TextFieldLayoutMode) -> Self {
        self.layout_mode = mode;
        self
    }

    pub fn build(self, commands: &mut Commands, theme: Option<&Theme>) -> Entity {
        let (surface, outline, on_surface, on_surface_muted, primary, radius, text_font) =
            match theme {
                Some(theme) => (
                    theme.palette.surface,
                    theme.palette.outline,
                    theme.palette.on_surface,
                    theme.palette.on_surface_muted,
                    theme.palette.primary,
                    theme.radius.md,
                    theme.typography.body_medium.font.clone(),
                ),
                None => (
                    Color::srgb(0.12, 0.12, 0.12),
                    Color::srgb(0.3, 0.3, 0.3),
                    Color::srgb(0.9, 0.9, 0.9),
                    Color::srgb(0.55, 0.55, 0.55),
                    Color::srgb(0.2, 0.6, 0.95),
                    8.0,
                    TextFont::from_font_size(13.0),
                ),
            };
        let is_multiline = matches!(self.layout_mode, TextFieldLayoutMode::MultiLine);

        let background = commands
            .spawn((
                Node {
                    width: self.width,
                    height: self.height,
                    padding: UiRect::all(px(4.)),
                    border: UiRect::all(px(1.)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(surface),
                BorderColor::all(outline),
                BorderRadius::all(px(radius)),
                TextField {
                    value: self.text.clone(),
                    placeholder: self.placeholder.clone(),
                    max_length: self.max_length,
                    validator: self.validator,
                    formatter: self.formatter,
                    input_type: if is_multiline {
                        TextInputType::MultiLine
                    } else {
                        TextInputType::SingleLine
                    },
                    disabled: self.disabled,
                    layout_mode: self.layout_mode,
                    min_width: val_to_px(self.width, 200.0),
                    min_height: val_to_px(self.height, 30.0),
                    ..default()
                },
                Interaction::default(),
            ))
            .id();

        let input_text = commands
            .spawn((
                Node {
                    width: percent(100.0),
                    height: percent(100.0),
                    ..default()
                },
                Text(if self.text.is_empty() {
                    self.placeholder.clone()
                } else {
                    self.text.clone()
                }),
                match self.layout_mode {
                    TextFieldLayoutMode::AutoWrap | TextFieldLayoutMode::MultiLine => {
                        TextLayout::new_with_linebreak(bevy::text::LineBreak::WordOrCharacter)
                    }
                    _ => TextLayout::new_with_no_wrap(),
                },
                text_font,
                TextColor(if self.text.is_empty() {
                    on_surface_muted
                } else {
                    on_surface
                }),
                RelativeCursorPosition::default(),
                TextFieldInputText,
                Name::new("Input Text"),
            ))
            .id();

        let cursor = commands
            .spawn((
                Node {
                    width: px(2.0),
                    height: px(20.0),
                    position_type: PositionType::Absolute,
                    left: px(0.),
                    top: px(2.0),
                    ..default()
                },
                BackgroundColor(on_surface),
                Visibility::Hidden,
                TextFieldCursor,
                Name::new("Text Cursor"),
            ))
            .id();

        let selection = commands
            .spawn((
                Node {
                    width: px(0.0),
                    height: percent(100.0),
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    top: px(0.0),
                    ..default()
                },
                BackgroundColor(primary.with_alpha(0.45)),
                Visibility::Hidden,
                TextFieldSelectionHighlight,
                Name::new("Text Selection"),
            ))
            .id();

        commands.entity(background).add_children(&[input_text]);
        commands
            .entity(input_text)
            .add_children(&[selection, cursor]);

        background
    }
}

impl DefaultWidgetBuilder for TextFieldBuilder {
    fn spawn_default(
        commands: &mut Commands,
        theme: Option<&crate::theme::Theme>,
    ) -> WidgetSpawnResult {
        TextFieldBuilder::new().build(commands, theme).into()
    }
}

fn text_field_focus(
    mouse_btn: Res<ButtonInput<MouseButton>>,
    mut focused: ResMut<FocusedTextField>,
    mut fields: Query<(Entity, &Interaction, &mut TextField), With<TextField>>,
) {
    let mut is_focused = false;

    for (entity, interaction, mut field) in fields.iter_mut() {
        if matches!(interaction, Interaction::Pressed)
            || (matches!(interaction, Interaction::Hovered)
                && (mouse_btn.just_pressed(MouseButton::Left)
                    || mouse_btn.just_released(MouseButton::Left)))
        {
            if focused.0 != Some(entity) {
                field.drag_anchor = None;
            }
            focused.0 = Some(entity);
            is_focused = true;
            break;
        }
    }

    if mouse_btn.just_pressed(MouseButton::Left) && !is_focused {
        focused.0 = None;
        for (_, _, mut field) in fields.iter_mut() {
            field.drag_anchor = None;
        }
    }
}

fn text_field_click_to_move(
    mouse_btn: Res<ButtonInput<MouseButton>>,
    focused: Res<FocusedTextField>,
    text_entities: Query<(), With<TextFieldInputText>>,
    text_nodes: Query<
        (&RelativeCursorPosition, &ComputedNode, &TextLayoutInfo),
        With<TextFieldInputText>,
    >,
    children_query: Query<&Children>,
    mut fields: Query<(Entity, &Children, &Interaction, &mut TextField)>,
) {
    if !mouse_btn.just_pressed(MouseButton::Left) {
        return;
    }

    for (field_entity, children, interaction, mut field) in fields.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if focused.0 != Some(field_entity) || field.disabled {
            continue;
        }

        let text_entity = find_descendant_with(children, &children_query, &text_entities);
        let Some(text_entity) = text_entity else {
            continue;
        };
        let Ok((cursor_pos, node, layout)) = text_nodes.get(text_entity) else {
            continue;
        };
        if !cursor_pos.cursor_over {
            continue;
        }
        let Some(normalized) = cursor_pos.normalized else {
            continue;
        };
        // left-based coordinates: [0, width]
        let local_x = (normalized.x + 0.5) * node.size.x;
        let local_y = (normalized.y + 0.5) * node.size.y;

        let new_pos = cursor_pos_from_xy_nearest(&field.value, local_x, local_y, layout);
        field.cursor_pos = new_pos;
        field.selection = None;
        field.drag_anchor = Some(new_pos);
    }
}

fn text_field_drag_select(
    mouse_btn: Res<ButtonInput<MouseButton>>,
    focused: Res<FocusedTextField>,
    text_entities: Query<(), With<TextFieldInputText>>,
    text_nodes: Query<
        (&RelativeCursorPosition, &ComputedNode, &TextLayoutInfo),
        With<TextFieldInputText>,
    >,
    children_query: Query<&Children>,
    mut fields: Query<(Entity, &Children, &mut TextField)>,
) {
    let Some(focused_entity) = focused.0 else {
        return;
    };

    if mouse_btn.just_released(MouseButton::Left) {
        if let Ok((_, _, mut field)) = fields.get_mut(focused_entity) {
            field.drag_anchor = None;
        }
        return;
    }

    if !mouse_btn.pressed(MouseButton::Left) {
        return;
    }

    let Ok((_, children, mut field)) = fields.get_mut(focused_entity) else {
        return;
    };
    if field.disabled {
        return;
    }

    let Some(anchor) = field.drag_anchor else {
        field.drag_anchor = Some(field.cursor_pos);
        return;
    };

    let text_entity = find_descendant_with(children, &children_query, &text_entities);
    let Some(text_entity) = text_entity else {
        return;
    };
    let Ok((cursor_pos, node, layout)) = text_nodes.get(text_entity) else {
        return;
    };
    let Some(normalized) = cursor_pos.normalized else {
        return;
    };
    let local_x = ((normalized.x + 0.5) * node.size.x).clamp(0.0, node.size.x);
    let local_y = ((normalized.y + 0.5) * node.size.y).clamp(0.0, node.size.y);
    let new_pos = cursor_pos_from_xy_nearest(&field.value, local_x, local_y, layout);
    field.cursor_pos = new_pos;
    field.selection = if new_pos == anchor {
        None
    } else {
        Some((anchor, new_pos))
    };
}

fn text_field_ime_state(
    focused: Res<FocusedTextField>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    if focused.0.is_some() {
        if let Some(pos) = window.cursor_position() {
            window.ime_position = pos;
        }
        window.ime_enabled = true;
    } else {
        window.ime_enabled = false;
    }
}

fn text_field_ime_input(
    focused: Res<FocusedTextField>,
    mut ime_reader: MessageReader<Ime>,
    mut fields: Query<(Entity, &mut TextField)>,
    mut change_events: MessageWriter<TextInputChange>,
) {
    let Some(focused) = focused.0 else {
        ime_reader.clear();
        return;
    };

    let Ok((entity, mut field)) = fields.get_mut(focused) else {
        ime_reader.clear();
        return;
    };

    if field.disabled {
        ime_reader.clear();
        return;
    }

    let mut changed = false;
    let mut text_len = field.value.chars().count();

    for ime in ime_reader.read() {
        if let Ime::Commit { value, .. } = ime {
            for ch in value.chars().filter(is_printable_char) {
                if let Some(max) = field.max_length {
                    if field.value.chars().count() >= max {
                        break;
                    }
                }

                if let Some((start, end)) = selection_range(field.selection, text_len) {
                    remove_char_range(&mut field.value, start, end);
                    field.cursor_pos = start;
                    field.selection = None;
                    text_len = field.value.chars().count();
                }

                let char_index = field.cursor_pos;
                insert_char_at(&mut field.value, char_index, ch);
                field.cursor_pos += 1;
                text_len += 1;
                changed = true;
            }
        }
    }

    if changed {
        change_events.write(TextInputChange {
            entity,
            value: field.value.clone(),
        });
    }
}

fn text_field_layout_behavior(
    mut fields: Query<(&mut TextField, &Children, &mut Node)>,
    children_query: Query<&Children>,
    text_entities: Query<(), With<TextFieldInputText>>,
    text_infos: Query<&TextLayoutInfo, With<TextFieldInputText>>,
    mut text_layouts: Query<&mut TextLayout, With<TextFieldInputText>>,
) {
    for (mut field, children, mut field_node) in fields.iter_mut() {
        let text_entity = find_descendant_with(children, &children_query, &text_entities);
        let Some(text_entity) = text_entity else {
            continue;
        };
        let Ok(layout_info) = text_infos.get(text_entity) else {
            continue;
        };
        let Ok(mut text_layout) = text_layouts.get_mut(text_entity) else {
            continue;
        };

        match field.layout_mode {
            TextFieldLayoutMode::FixedTruncate => {
                text_layout.linebreak = bevy::text::LineBreak::NoWrap;
                if let Val::Px(width_px) = field_node.width {
                    let available = (width_px - 8.0).max(1.0);
                    if !field.value.is_empty() && layout_info.size.x > available {
                        let len = field.value.chars().count();
                        if len > 0 {
                            let keep =
                                ((len as f32) * (available / layout_info.size.x)).floor() as usize;
                            let keep = keep.min(len);
                            field.value = field.value.chars().take(keep).collect();
                            field.cursor_pos = field.cursor_pos.min(keep);
                            field.selection = None;
                        }
                    }
                }
            }
            TextFieldLayoutMode::AutoWidth => {
                text_layout.linebreak = bevy::text::LineBreak::NoWrap;
                let desired = (layout_info.size.x + 12.0).max(field.min_width);
                field_node.width = px(desired);
            }
            TextFieldLayoutMode::AutoWrap => {
                text_layout.linebreak = bevy::text::LineBreak::WordOrCharacter;
                let desired_h = (layout_info.size.y + 8.0).max(field.min_height);
                field_node.height = px(desired_h);
            }
            TextFieldLayoutMode::MultiLine => {
                text_layout.linebreak = bevy::text::LineBreak::WordOrCharacter;
                let desired_h = (layout_info.size.y + 8.0).max(field.min_height);
                field_node.height = px(desired_h);
            }
        }
    }
}

fn text_field_input(
    focused: Res<FocusedTextField>,
    mut inputs: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut fields: Query<(Entity, &mut TextField)>,
    mut clipboard: ResMut<TextClipboard>,
    mut change_events: MessageWriter<TextInputChange>,
    mut submit_events: MessageWriter<TextInputSubmit>,
) {
    let Some(focused) = focused.0 else {
        return;
    };

    let Ok((entity, mut field)) = fields.get_mut(focused) else {
        return;
    };

    if field.disabled {
        return;
    }

    let mut text_len = field.value.chars().count();
    field.cursor_pos = field.cursor_pos.min(text_len);

    let mut changed = false;
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    // text entry
    for input in inputs.read() {
        if input.state != ButtonState::Pressed {
            continue;
        }

        if ctrl {
            match input.key_code {
                KeyCode::KeyA => {
                    let len = field.value.chars().count();
                    if len > 0 {
                        field.selection = Some((0, len));
                        field.cursor_pos = len;
                    }
                    continue;
                }
                KeyCode::KeyC => {
                    if let Some((start, end)) =
                        selection_range(field.selection, field.value.chars().count())
                    {
                        let selected = text_slice_chars(&field.value, start, end).to_string();
                        clipboard.fallback = selected.clone();
                        if let Some(os) = clipboard.os_clipboard.as_mut() {
                            let _ = os.set_text(selected);
                        }
                    }
                    continue;
                }
                KeyCode::KeyX => {
                    if let Some((start, end)) =
                        selection_range(field.selection, field.value.chars().count())
                    {
                        let selected = text_slice_chars(&field.value, start, end).to_string();
                        clipboard.fallback = selected.clone();
                        if let Some(os) = clipboard.os_clipboard.as_mut() {
                            let _ = os.set_text(selected);
                        }
                        remove_char_range(&mut field.value, start, end);
                        field.cursor_pos = start;
                        field.selection = None;
                        text_len = field.value.chars().count();
                        changed = true;
                    }
                    continue;
                }
                KeyCode::KeyV => {
                    let pasted = if let Some(os) = clipboard.os_clipboard.as_mut() {
                        os.get_text()
                            .ok()
                            .unwrap_or_else(|| clipboard.fallback.clone())
                    } else {
                        clipboard.fallback.clone()
                    };

                    if !pasted.is_empty() {
                        if let Some((start, end)) =
                            selection_range(field.selection, field.value.chars().count())
                        {
                            remove_char_range(&mut field.value, start, end);
                            field.cursor_pos = start;
                            field.selection = None;
                            text_len = field.value.chars().count();
                        }
                        for ch in pasted.chars().filter(is_printable_char) {
                            if let Some(max) = field.max_length {
                                if field.value.chars().count() >= max {
                                    break;
                                }
                            }
                            let char_index = field.cursor_pos;
                            insert_char_at(&mut field.value, char_index, ch);
                            field.cursor_pos += 1;
                            text_len += 1;
                            changed = true;
                        }
                    }
                    continue;
                }
                _ => {
                    // With Ctrl pressed, do not treat key events as text input.
                    continue;
                }
            }
        }

        if input.key_code == KeyCode::ArrowLeft {
            if let Some((start, _)) = selection_range(field.selection, text_len) {
                field.cursor_pos = start;
            } else if field.cursor_pos > 0 {
                field.cursor_pos -= 1;
            }
            field.selection = None;
            continue;
        }

        if input.key_code == KeyCode::ArrowRight {
            if let Some((_, end)) = selection_range(field.selection, text_len) {
                field.cursor_pos = end;
            } else if field.cursor_pos < text_len {
                field.cursor_pos += 1;
            }
            field.selection = None;
            continue;
        }

        if input.key_code == KeyCode::Backspace {
            if let Some((start, end)) = selection_range(field.selection, text_len) {
                remove_char_range(&mut field.value, start, end);
                field.cursor_pos = start;
                field.selection = None;
                text_len = field.value.chars().count();
                changed = true;
                continue;
            }
            if field.cursor_pos > 0 {
                let remove_at = field.cursor_pos - 1;
                let end = field.cursor_pos;
                remove_char_range(&mut field.value, remove_at, end);
                field.cursor_pos = remove_at;
                text_len = field.value.chars().count();
                changed = true;
            }
            continue;
        }

        let text: Option<&str> = input.text.as_deref().or_else(|| match &input.logical_key {
            Key::Character(s) => Some(s.as_str()),
            _ => None,
        });
        let Some(text) = text else {
            continue;
        };

        for ch in text.chars() {
            if ch.is_control() {
                continue;
            }
            if let Some(max) = field.max_length {
                if field.value.chars().count() >= max {
                    break;
                }
            }

            if let Some((start, end)) = selection_range(field.selection, text_len) {
                remove_char_range(&mut field.value, start, end);
                field.cursor_pos = start;
                field.selection = None;
                text_len = field.value.chars().count();
            }

            let char_index = field.cursor_pos;
            insert_char_at(&mut field.value, char_index, ch);
            field.cursor_pos += 1;
            text_len += 1;
            changed = true;
        }
    }

    if changed {
        change_events.write(TextInputChange {
            entity,
            value: field.value.clone(),
        });
    }

    // submit / newline
    if keys.just_pressed(KeyCode::Enter) {
        if matches!(field.input_type, TextInputType::MultiLine) {
            if field
                .max_length
                .is_none_or(|max| field.value.chars().count() < max)
            {
                if let Some((start, end)) = selection_range(field.selection, text_len) {
                    remove_char_range(&mut field.value, start, end);
                    field.cursor_pos = start;
                    field.selection = None;
                }
                let insert_at = field.cursor_pos;
                insert_char_at(&mut field.value, insert_at, '\n');
                field.cursor_pos = insert_at + 1;
                change_events.write(TextInputChange {
                    entity,
                    value: field.value.clone(),
                });
            }
        } else {
            submit_events.write(TextInputSubmit {
                entity,
                value: field.value.clone(),
            });
        }
    }
}

fn text_field_cursor_blink(time: Res<Time>, mut blink: ResMut<TextFieldCursorBlink>) {
    blink.timer.tick(time.delta());
    if blink.timer.just_finished() {
        blink.visible = !blink.visible;
    }
}

fn text_field_sync_visuals(
    focused: Res<FocusedTextField>,
    blink: Res<TextFieldCursorBlink>,
    global_theme: Option<Res<Theme>>,
    fields: Query<(Entity, &TextField, &Children)>,
    parents: Query<&ChildOf>,
    scopes: Query<&ThemeScope>,
    boundaries: Query<(), With<ThemeBoundary>>,
    children_query: Query<&Children>,
    text_entities: Query<(), With<TextFieldInputText>>,
    cursor_entities: Query<(), With<TextFieldCursor>>,
    selection_entities: Query<(), With<TextFieldSelectionHighlight>>,
    text_layouts: Query<&TextLayoutInfo, With<TextFieldInputText>>,
    text_nodes: Query<&ComputedNode, With<TextFieldInputText>>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<TextFieldInputText>>,
    mut overlays: ParamSet<(
        Query<(&mut BackgroundColor, &mut BorderColor), With<TextField>>,
        Query<(&mut Node, &mut Visibility), With<TextFieldCursor>>,
        Query<
            (&mut Node, &mut Visibility, &mut BackgroundColor),
            With<TextFieldSelectionHighlight>,
        >,
    )>,
) {
    for (field_entity, field, children) in fields.iter() {
        let (surface, outline, on_surface, on_surface_muted, primary, disabled_bg) =
            match resolve_theme_or_global(
                field_entity,
                &parents,
                &scopes,
                &boundaries,
                global_theme.as_deref(),
            ) {
                Some(theme) => (
                    theme.palette.surface,
                    theme.palette.outline,
                    theme.palette.on_surface,
                    theme.palette.on_surface_muted,
                    theme.palette.primary,
                    theme.palette.disabled_container,
                ),
                None => (
                    Color::srgb(0.12, 0.12, 0.12),
                    Color::srgb(0.3, 0.3, 0.3),
                    Color::srgb(0.9, 0.9, 0.9),
                    Color::srgb(0.55, 0.55, 0.55),
                    Color::srgb(0.2, 0.6, 0.95),
                    Color::srgb(0.09, 0.09, 0.09),
                ),
            };

        if let Ok((mut background, mut border)) = overlays.p0().get_mut(field_entity) {
            background.0 = if field.disabled { disabled_bg } else { surface };
            *border = BorderColor::all(if field.disabled {
                outline.with_alpha(0.55)
            } else if focused.0 == Some(field_entity) {
                primary.with_alpha(0.85)
            } else {
                outline
            });
        }

        let text_entity = find_descendant_with(children, &children_query, &text_entities);
        let cursor_entity = find_descendant_with(children, &children_query, &cursor_entities);
        let selection_entity = find_descendant_with(children, &children_query, &selection_entities);

        let Some(text_entity) = text_entity else {
            continue;
        };
        let Some(cursor_entity) = cursor_entity else {
            continue;
        };

        if let Ok((mut text, mut text_color)) = text_query.get_mut(text_entity) {
            let value_color = if field.disabled {
                on_surface_muted.with_alpha(0.75)
            } else {
                on_surface
            };
            let placeholder_color = if field.disabled {
                on_surface_muted.with_alpha(0.55)
            } else {
                on_surface_muted
            };
            if field.value.is_empty() {
                if text.0 != field.placeholder {
                    text.0 = field.placeholder.clone();
                }
                if text_color.0 != placeholder_color {
                    *text_color = TextColor(placeholder_color);
                }
            } else {
                if text.0 != field.value {
                    text.0 = field.value.clone();
                }
                if text_color.0 != value_color {
                    *text_color = TextColor(value_color);
                }
            }
        }

        let layout = text_layouts.get(text_entity).ok();
        let inv_scale = text_nodes
            .get(text_entity)
            .map(|n| n.inverse_scale_factor())
            .unwrap_or(1.0);
        if let Ok((mut cursor_node, mut cursor_vis)) = overlays.p1().get_mut(cursor_entity) {
            let is_focused = focused.0 == Some(field_entity) && !field.disabled;
            *cursor_vis = if is_focused && blink.visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };

            let (cursor_left, cursor_top, cursor_height) = match layout {
                Some(layout) if !field.value.is_empty() => {
                    if matches!(
                        field.layout_mode,
                        TextFieldLayoutMode::AutoWrap | TextFieldLayoutMode::MultiLine
                    ) {
                        cursor_metrics_from_pos(&field.value, field.cursor_pos, layout)
                    } else {
                        (
                            cursor_x_from_pos(&field.value, field.cursor_pos, layout),
                            2.0,
                            20.0,
                        )
                    }
                }
                _ => (0.0, 2.0, 20.0),
            };
            cursor_node.left = px(cursor_left * inv_scale);
            cursor_node.top = px((cursor_top * inv_scale).max(0.0));
            cursor_node.height = px((cursor_height * inv_scale).max(12.0));
        }

        if let (Some(selection_entity), Some(layout)) = (selection_entity, layout) {
            if let Ok((mut sel_node, mut sel_vis, mut sel_bg)) =
                overlays.p2().get_mut(selection_entity)
            {
                let is_focused = focused.0 == Some(field_entity) && !field.disabled;
                if is_focused {
                    if let Some((start, end)) =
                        selection_range(field.selection, field.value.chars().count())
                    {
                        let x0 = cursor_x_from_pos(&field.value, start, layout);
                        let x1 = cursor_x_from_pos(&field.value, end, layout);
                        let left = x0.min(x1) * inv_scale;
                        let width = (x1 - x0).abs() * inv_scale;
                        sel_node.left = px(left);
                        sel_node.width = px(width.max(1.0));
                        sel_bg.0 = primary.with_alpha(0.45);
                        *sel_vis = Visibility::Visible;
                    } else {
                        *sel_vis = Visibility::Hidden;
                    }
                } else {
                    *sel_vis = Visibility::Hidden;
                }
            }
        }
    }
}

fn find_descendant_with<T: Component>(
    children: &Children,
    children_query: &Query<&Children>,
    query: &Query<(), With<T>>,
) -> Option<Entity> {
    let mut stack: Vec<Entity> = children.iter().collect();
    while let Some(entity) = stack.pop() {
        if query.get(entity).is_ok() {
            return Some(entity);
        }
        if let Ok(kids) = children_query.get(entity) {
            stack.extend(kids.iter());
        }
    }
    None
}

fn selection_range(selection: Option<(usize, usize)>, len: usize) -> Option<(usize, usize)> {
    let (a, b) = selection?;
    let start = a.min(b).min(len);
    let end = a.max(b).min(len);
    if start == end {
        None
    } else {
        Some((start, end))
    }
}

fn char_index_to_byte(s: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    s.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or_else(|| s.len())
}

fn byte_index_to_char(s: &str, byte_index: usize) -> usize {
    let mut count = 0;
    for (idx, _) in s.char_indices() {
        if idx >= byte_index {
            break;
        }
        count += 1;
    }
    count
}

fn remove_char_range(s: &mut String, start: usize, end: usize) {
    if start >= end {
        return;
    }
    let start_b = char_index_to_byte(s, start);
    let end_b = char_index_to_byte(s, end);
    s.replace_range(start_b..end_b, "");
}

fn insert_char_at(s: &mut String, char_index: usize, ch: char) {
    let byte_index = char_index_to_byte(s, char_index);
    s.insert(byte_index, ch);
}

fn text_slice_chars(s: &str, start: usize, end: usize) -> &str {
    if start >= end {
        return "";
    }
    let start_b = char_index_to_byte(s, start);
    let end_b = char_index_to_byte(s, end);
    &s[start_b..end_b]
}

fn val_to_px(val: Val, default_px: f32) -> f32 {
    match val {
        Val::Px(v) => v,
        _ => default_px,
    }
}

fn cursor_x_from_pos(text: &str, pos: usize, layout: &TextLayoutInfo) -> f32 {
    let spans = build_glyph_spans(text, layout);
    if spans.is_empty() {
        return 0.0;
    }

    if pos <= spans[0].start {
        return spans[0].left;
    }

    let mut prev: Option<GlyphSpan> = None;
    for span in &spans {
        if pos < span.start {
            if let Some(prev) = prev {
                if pos == prev.end {
                    return prev.right;
                }
            }
            return span.left;
        }

        if pos >= span.start && pos < span.end {
            let chars = (span.end - span.start).max(1) as f32;
            let t = (pos - span.start) as f32 / chars;
            return span.left + (span.right - span.left) * t;
        }

        prev = Some(*span);
    }

    spans.last().map(|span| span.right).unwrap_or(0.0)
}

fn cursor_pos_from_xy_nearest(text: &str, x: f32, y: f32, layout: &TextLayoutInfo) -> usize {
    let len = text.chars().count();
    if len == 0 {
        return 0;
    }

    let spans = build_glyph_spans(text, layout);
    if spans.is_empty() {
        return 0;
    }

    let target_line = nearest_line_from_y(y, &spans);
    let mut best_pos = 0usize;
    let mut best_dist = f32::INFINITY;
    for pos in 0..=len {
        if line_for_pos(text, pos, &spans) != target_line {
            continue;
        }
        let px = cursor_x_from_pos_on_line(pos, target_line, &spans);
        let dist = (px - x).abs();
        if dist < best_dist {
            best_dist = dist;
            best_pos = pos;
        }
    }

    if best_dist.is_finite() { best_pos } else { 0 }
}

#[derive(Clone, Copy)]
struct GlyphSpan {
    start: usize,
    end: usize,
    line: usize,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

fn build_glyph_spans(text: &str, layout: &TextLayoutInfo) -> Vec<GlyphSpan> {
    if text.is_empty() || layout.glyphs.is_empty() {
        return Vec::new();
    }

    let mut glyphs = layout.glyphs.clone();
    glyphs.sort_by_key(|g| g.byte_index);

    let mut spans = Vec::with_capacity(glyphs.len());
    for glyph in glyphs {
        let start = byte_index_to_char(text, glyph.byte_index);
        let end = byte_index_to_char(text, glyph.byte_index + glyph.byte_length);
        if end <= start {
            continue;
        }

        let left = glyph.position.x - glyph.size.x * 0.5;
        let right = glyph.position.x + glyph.size.x * 0.5;
        let top = glyph.position.y - glyph.size.y * 0.5;
        let bottom = glyph.position.y + glyph.size.y * 0.5;
        spans.push(GlyphSpan {
            start,
            end,
            line: glyph.line_index as usize,
            left,
            right,
            top,
            bottom,
        });
    }

    spans
}

fn cursor_x_from_pos_on_line(pos: usize, line: usize, spans: &[GlyphSpan]) -> f32 {
    let Some(first) = spans.iter().find(|s| s.line == line) else {
        return 0.0;
    };

    if pos <= first.start {
        return first.left;
    }

    let mut prev: Option<GlyphSpan> = None;
    for span in spans.iter().filter(|s| s.line == line) {
        if pos < span.start {
            if let Some(prev) = prev {
                if pos == prev.end {
                    return prev.right;
                }
            }
            return span.left;
        }
        if pos >= span.start && pos < span.end {
            let chars = (span.end - span.start).max(1) as f32;
            let t = (pos - span.start) as f32 / chars;
            return span.left + (span.right - span.left) * t;
        }
        prev = Some(*span);
    }

    prev.map(|s| s.right).unwrap_or(first.left)
}

fn line_for_pos(text: &str, pos: usize, spans: &[GlyphSpan]) -> usize {
    if spans.is_empty() {
        return 0;
    }

    let mut prev: Option<GlyphSpan> = None;
    for span in spans {
        if pos < span.start {
            if let Some(prev) = prev {
                if pos == prev.end {
                    return prev.line;
                }
            }
            return span.line;
        }
        if pos >= span.start && pos < span.end {
            return span.line;
        }
        prev = Some(*span);
    }

    if pos == text.chars().count() && text.ends_with('\n') {
        spans.last().map(|s| s.line + 1).unwrap_or(0)
    } else {
        spans.last().map(|s| s.line).unwrap_or(0)
    }
}

fn nearest_line_from_y(y: f32, spans: &[GlyphSpan]) -> usize {
    if spans.is_empty() {
        return 0;
    }

    let mut lines: Vec<usize> = spans.iter().map(|s| s.line).collect();
    lines.sort_unstable();
    lines.dedup();

    let mut best_line = lines[0];
    let mut best_dist = f32::INFINITY;

    for line in lines {
        let (top, bottom, _) = line_bounds(line, spans);
        let dist = if y < top {
            top - y
        } else if y > bottom {
            y - bottom
        } else {
            0.0
        };
        if dist < best_dist {
            best_dist = dist;
            best_line = line;
        }
    }

    best_line
}

fn cursor_metrics_from_pos(text: &str, pos: usize, layout: &TextLayoutInfo) -> (f32, f32, f32) {
    let spans = build_glyph_spans(text, layout);
    if spans.is_empty() {
        return (0.0, 0.0, 20.0);
    }

    let line = line_for_pos(text, pos, &spans);
    let x = cursor_x_from_pos_on_line(pos, line, &spans);

    let is_trailing_newline = pos == text.chars().count() && text.ends_with('\n');
    let (top, bottom, line_left) = if is_trailing_newline && line > 0 {
        line_bounds(line - 1, &spans)
    } else {
        line_bounds(line, &spans)
    };
    let (first_top, _, _) = line_bounds(spans[0].line, &spans);
    let line_height = (bottom - top).max(12.0);
    let top_relative = 2.0 + (top - first_top);

    if is_trailing_newline {
        return (line_left, top_relative + line_height, line_height);
    }

    (x, top_relative, line_height)
}

fn line_bounds(line: usize, spans: &[GlyphSpan]) -> (f32, f32, f32) {
    let mut min_top = f32::INFINITY;
    let mut max_bottom = f32::NEG_INFINITY;
    let mut min_left = f32::INFINITY;
    for span in spans.iter().filter(|s| s.line == line) {
        min_top = min_top.min(span.top);
        max_bottom = max_bottom.max(span.bottom);
        min_left = min_left.min(span.left);
    }

    if !min_top.is_finite() || !max_bottom.is_finite() {
        let fallback_top = spans.first().map(|s| s.top).unwrap_or(0.0);
        let fallback_bottom = spans.first().map(|s| s.bottom).unwrap_or(20.0);
        let fallback_left = spans.first().map(|s| s.left).unwrap_or(0.0);
        return (fallback_top, fallback_bottom, fallback_left);
    }

    (min_top, max_bottom, min_left)
}

// this logic is taken from egui-winit
fn is_printable_char(chr: &char) -> bool {
    let is_in_private_use_area = ('\u{e000}'..='\u{f8ff}').contains(chr)
        || ('\u{f0000}'..='\u{ffffd}').contains(chr)
        || ('\u{100000}'..='\u{10fffd}').contains(chr);

    !is_in_private_use_area && !chr.is_ascii_control()
}
