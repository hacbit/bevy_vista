use bevy::prelude::*;

use crate::theme::Theme;

use super::*;

pub struct NumericFieldsPlugin;

impl Plugin for NumericFieldsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NumberFieldPlugin);
    }
}

pub struct NumberFieldPlugin;

impl Plugin for NumberFieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<NumberFieldChange>().add_systems(
            PostUpdate,
            (
                normalize_number_fields,
                sync_number_from_text_changes,
                sync_number_from_text_submit,
                sync_number_visuals,
            )
                .chain(),
        );
    }
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumberKind {
    I8,
    I16,
    I32,
    I64,
    Isize,
    U8,
    U16,
    U32,
    U64,
    Usize,
    F32,
    F64,
}

impl NumberKind {
    pub const fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    pub const fn is_signed(self) -> bool {
        matches!(
            self,
            Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::Isize | Self::F32 | Self::F64
        )
    }

    pub const fn placeholder(self) -> &'static str {
        if self.is_float() { "0.0" } else { "0" }
    }

    fn kind_range(self) -> (f64, f64) {
        match self {
            Self::I8 => (i8::MIN as f64, i8::MAX as f64),
            Self::I16 => (i16::MIN as f64, i16::MAX as f64),
            Self::I32 => (i32::MIN as f64, i32::MAX as f64),
            Self::I64 => (i64::MIN as f64, i64::MAX as f64),
            Self::Isize => (isize::MIN as f64, isize::MAX as f64),
            Self::U8 => (u8::MIN as f64, u8::MAX as f64),
            Self::U16 => (u16::MIN as f64, u16::MAX as f64),
            Self::U32 => (u32::MIN as f64, u32::MAX as f64),
            Self::U64 => (0.0, u64::MAX as f64),
            Self::Usize => (0.0, usize::MAX as f64),
            Self::F32 => (f32::MIN as f64, f32::MAX as f64),
            Self::F64 => (f64::MIN, f64::MAX),
        }
    }

    pub fn normalize(self, value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
        let (kind_min, kind_max) = self.kind_range();
        let mut value = value.clamp(kind_min, kind_max);
        if let Some(min) = min {
            value = value.max(min);
        }
        if let Some(max) = max {
            value = value.min(max);
        }
        if self.is_float() {
            value
        } else {
            value.round()
        }
    }

    pub fn parse_input(self, value: &str, min: Option<f64>, max: Option<f64>) -> Option<f64> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }

        let parsed = if self.is_float() {
            if matches!(trimmed, "-" | "+" | "." | "-." | "+.") {
                return None;
            }
            trimmed.parse::<f64>().ok()?
        } else if self.is_signed() {
            if matches!(trimmed, "-" | "+") {
                return None;
            }
            trimmed.parse::<i128>().ok()? as f64
        } else {
            trimmed.parse::<u128>().ok()? as f64
        };

        Some(self.normalize(parsed, min, max))
    }

    pub fn format_value(self, value: f64) -> String {
        let value = self.normalize(value, None, None);
        match self {
            Self::I8 => (value as i8).to_string(),
            Self::I16 => (value as i16).to_string(),
            Self::I32 => (value as i32).to_string(),
            Self::I64 => (value as i64).to_string(),
            Self::Isize => (value as isize).to_string(),
            Self::U8 => (value as u8).to_string(),
            Self::U16 => (value as u16).to_string(),
            Self::U32 => (value as u32).to_string(),
            Self::U64 => (value.max(0.0) as u64).to_string(),
            Self::Usize => (value.max(0.0) as usize).to_string(),
            Self::F32 => {
                let value = value as f32;
                if value.fract() == 0.0 {
                    format!("{value:.1}")
                } else {
                    value.to_string()
                }
            }
            Self::F64 => {
                if value.fract() == 0.0 {
                    format!("{value:.1}")
                } else {
                    value.to_string()
                }
            }
        }
    }
}

#[derive(Component, Reflect, Clone, Widget, ShowInInspector)]
#[widget("input/number_field", children = "exact(0)")]
#[builder(NumberFieldBuilder)]
pub struct NumberField {
    pub kind: NumberKind,
    pub value: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: f64,
    pub disabled: bool,
}

impl Default for NumberField {
    fn default() -> Self {
        Self {
            kind: NumberKind::F32,
            value: 0.0,
            min: None,
            max: None,
            step: 1.0,
            disabled: false,
        }
    }
}

#[derive(Component)]
struct NumberFieldTextInput;

#[derive(Message, EntityEvent)]
pub struct NumberFieldChange {
    pub entity: Entity,
    pub value: f64,
}

#[derive(Clone)]
pub struct NumberFieldBuilder {
    field: NumberField,
    width: Val,
    height: Val,
}

impl Default for NumberFieldBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NumberFieldBuilder {
    pub fn new() -> Self {
        Self {
            field: NumberField::default(),
            width: px(120.0),
            height: px(28.0),
        }
    }

    pub fn kind(mut self, kind: NumberKind) -> Self {
        self.field.kind = kind;
        self.field.value = kind.normalize(self.field.value, self.field.min, self.field.max);
        self.field.step = default_step(kind);
        self
    }

    pub fn value(mut self, value: impl Into<f64>) -> Self {
        self.field.value = self
            .field
            .kind
            .normalize(value.into(), self.field.min, self.field.max);
        self
    }

    pub fn min(mut self, min: Option<impl Into<f64>>) -> Self {
        self.field.min = min.map(Into::into);
        self.field.value = self
            .field
            .kind
            .normalize(self.field.value, self.field.min, self.field.max);
        self
    }

    pub fn max(mut self, max: Option<impl Into<f64>>) -> Self {
        self.field.max = max.map(Into::into);
        self.field.value = self
            .field
            .kind
            .normalize(self.field.value, self.field.min, self.field.max);
        self
    }

    pub fn step(mut self, step: impl Into<f64>) -> Self {
        self.field.step = step.into();
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

    pub fn height(mut self, height: Val) -> Self {
        self.height = height;
        self
    }

    pub fn build(self, commands: &mut Commands, theme: Option<&Theme>) -> Entity {
        let root = commands
            .spawn((
                Node {
                    width: self.width,
                    height: self.height,
                    ..default()
                },
                self.field.clone(),
            ))
            .id();

        let input = build_numeric_text_field(
            commands,
            "NumberField",
            self.field.kind.format_value(self.field.value),
            self.field.kind.placeholder(),
            self.width,
            self.height,
            self.field.disabled,
            theme,
            NumberFieldTextInput,
        );
        commands.entity(root).add_child(input);
        root
    }
}

impl DefaultWidgetBuilder for NumberFieldBuilder {
    fn spawn_default(commands: &mut Commands, theme: Option<&crate::theme::Theme>) -> WidgetSpawnResult {
        NumberFieldBuilder::new().build(commands, theme).into()
    }
}

fn build_numeric_text_field<T: Component>(
    commands: &mut Commands,
    name: &'static str,
    value: String,
    placeholder: &'static str,
    width: Val,
    height: Val,
    disabled: bool,
    theme: Option<&Theme>,
    marker: T,
) -> Entity {
    let input = TextFieldBuilder::new()
        .text(&value)
        .placeholder(placeholder)
        .width(width)
        .height(height)
        .disabled(disabled)
        .validator(TextInputValidator::Numeric)
        .build(commands, theme);
    if let Ok(mut entity) = commands.get_entity(input) {
        entity.insert((Name::new(format!("{name} Input")), marker));
    }
    input
}

fn sync_number_from_text_changes(
    mut changes: MessageReader<TextInputChange>,
    parents: Query<&ChildOf>,
    inputs: Query<(), With<NumberFieldTextInput>>,
    mut fields: Query<&mut NumberField>,
    mut out: MessageWriter<NumberFieldChange>,
) {
    for change in changes.read() {
        if inputs.get(change.entity).is_err() {
            continue;
        }
        let Some(field_entity) = parents.get(change.entity).ok().map(|parent| parent.parent()) else {
            continue;
        };
        let Ok(mut field) = fields.get_mut(field_entity) else {
            continue;
        };
        let Some(value) = field.kind.parse_input(&change.value, field.min, field.max) else {
            continue;
        };
        if field.value != value {
            field.value = value;
            out.write(NumberFieldChange {
                entity: field_entity,
                value,
            });
        }
    }
}

fn sync_number_from_text_submit(
    mut submits: MessageReader<TextInputSubmit>,
    parents: Query<&ChildOf>,
    inputs: Query<(), With<NumberFieldTextInput>>,
    fields: Query<&NumberField>,
    mut text_fields: Query<&mut TextField>,
) {
    for submit in submits.read() {
        if inputs.get(submit.entity).is_err() {
            continue;
        }
        let Some(field_entity) = parents.get(submit.entity).ok().map(|parent| parent.parent()) else {
            continue;
        };
        let Ok(field) = fields.get(field_entity) else {
            continue;
        };
        let Ok(mut text_field) = text_fields.get_mut(submit.entity) else {
            continue;
        };
        text_field.value = field.kind.format_value(field.value);
        text_field.cursor_pos = text_field.value.chars().count();
        text_field.selection = None;
    }
}

fn sync_number_visuals(
    fields: Query<(&NumberField, &Children), Changed<NumberField>>,
    children_query: Query<&Children>,
    input_markers: Query<(), With<NumberFieldTextInput>>,
    mut text_fields: Query<&mut TextField>,
) {
    for (field, children) in fields.iter() {
        let Some(input) = find_descendant_with(children, &children_query, &input_markers) else {
            continue;
        };
        let Ok(mut text_field) = text_fields.get_mut(input) else {
            continue;
        };
        let next = field.kind.format_value(field.value);
        if text_field.value != next {
            text_field.value = next;
            text_field.cursor_pos = text_field.value.chars().count();
            text_field.selection = None;
        }
        text_field.disabled = field.disabled;
        text_field.placeholder = field.kind.placeholder().to_owned();
    }
}

fn normalize_number_fields(mut fields: Query<&mut NumberField, Changed<NumberField>>) {
    for mut field in fields.iter_mut() {
        let normalized = field.kind.normalize(field.value, field.min, field.max);
        if field.value != normalized {
            field.value = normalized;
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

fn default_step(kind: NumberKind) -> f64 {
    if kind.is_float() { 1.0 } else { 1.0 }
}

pub type F32Field = NumberField;
pub type F32FieldBuilder = NumberFieldBuilder;
pub type F32FieldChange = NumberFieldChange;
pub type F32FieldPlugin = NumberFieldPlugin;
