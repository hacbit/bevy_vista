use bevy::prelude::*;

use crate::core::theme::Theme;

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
    pub const fn name(self) -> &'static str {
        match self {
            Self::I8 => "I8",
            Self::I16 => "I16",
            Self::I32 => "I32",
            Self::I64 => "I64",
            Self::Isize => "Isize",
            Self::U8 => "U8",
            Self::U16 => "U16",
            Self::U32 => "U32",
            Self::U64 => "U64",
            Self::Usize => "Usize",
            Self::F32 => "F32",
            Self::F64 => "F64",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "I8" => Some(Self::I8),
            "I16" => Some(Self::I16),
            "I32" => Some(Self::I32),
            "I64" => Some(Self::I64),
            "Isize" => Some(Self::Isize),
            "U8" => Some(Self::U8),
            "U16" => Some(Self::U16),
            "U32" => Some(Self::U32),
            "U64" => Some(Self::U64),
            "Usize" => Some(Self::Usize),
            "F32" => Some(Self::F32),
            "F64" => Some(Self::F64),
            _ => None,
        }
    }

    pub const fn placeholder(self) -> &'static str {
        match self {
            Self::F32 | Self::F64 => "0.0",
            _ => "0",
        }
    }
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq)]
pub enum Number {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Isize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Usize(usize),
    F32(f32),
    F64(f64),
}

impl Default for Number {
    fn default() -> Self {
        Self::F32(0.0)
    }
}

impl Number {
    pub const fn kind(self) -> NumberKind {
        match self {
            Self::I8(_) => NumberKind::I8,
            Self::I16(_) => NumberKind::I16,
            Self::I32(_) => NumberKind::I32,
            Self::I64(_) => NumberKind::I64,
            Self::Isize(_) => NumberKind::Isize,
            Self::U8(_) => NumberKind::U8,
            Self::U16(_) => NumberKind::U16,
            Self::U32(_) => NumberKind::U32,
            Self::U64(_) => NumberKind::U64,
            Self::Usize(_) => NumberKind::Usize,
            Self::F32(_) => NumberKind::F32,
            Self::F64(_) => NumberKind::F64,
        }
    }

    pub fn cast<T: NumberCast>(self) -> Option<T> {
        T::from_number(self)
    }

    fn as_i128(self) -> Option<i128> {
        match self {
            Self::I8(value) => Some(value as i128),
            Self::I16(value) => Some(value as i128),
            Self::I32(value) => Some(value as i128),
            Self::I64(value) => Some(value as i128),
            Self::Isize(value) => Some(value as i128),
            Self::U8(value) => Some(value as i128),
            Self::U16(value) => Some(value as i128),
            Self::U32(value) => Some(value as i128),
            Self::U64(value) => i128::try_from(value).ok(),
            Self::Usize(value) => i128::try_from(value).ok(),
            Self::F32(value) => float_to_i128(value as f64),
            Self::F64(value) => float_to_i128(value),
        }
    }

    fn as_u128(self) -> Option<u128> {
        match self {
            Self::I8(value) => u128::try_from(value).ok(),
            Self::I16(value) => u128::try_from(value).ok(),
            Self::I32(value) => u128::try_from(value).ok(),
            Self::I64(value) => u128::try_from(value).ok(),
            Self::Isize(value) => u128::try_from(value).ok(),
            Self::U8(value) => Some(value as u128),
            Self::U16(value) => Some(value as u128),
            Self::U32(value) => Some(value as u128),
            Self::U64(value) => Some(value as u128),
            Self::Usize(value) => Some(value as u128),
            Self::F32(value) => float_to_u128(value as f64),
            Self::F64(value) => float_to_u128(value),
        }
    }

    fn as_f32(self) -> f32 {
        match self {
            Self::I8(value) => value as f32,
            Self::I16(value) => value as f32,
            Self::I32(value) => value as f32,
            Self::I64(value) => value as f32,
            Self::Isize(value) => value as f32,
            Self::U8(value) => value as f32,
            Self::U16(value) => value as f32,
            Self::U32(value) => value as f32,
            Self::U64(value) => value as f32,
            Self::Usize(value) => value as f32,
            Self::F32(value) => value,
            Self::F64(value) => value as f32,
        }
    }

    fn as_f64_lossy(self) -> f64 {
        match self {
            Self::I8(value) => value as f64,
            Self::I16(value) => value as f64,
            Self::I32(value) => value as f64,
            Self::I64(value) => value as f64,
            Self::Isize(value) => value as f64,
            Self::U8(value) => value as f64,
            Self::U16(value) => value as f64,
            Self::U32(value) => value as f64,
            Self::U64(value) => value as f64,
            Self::Usize(value) => value as f64,
            Self::F32(value) => value as f64,
            Self::F64(value) => value,
        }
    }

    fn clamp_bounds(self, min: Option<Number>, max: Option<Number>) -> Self {
        match self {
            Self::I8(value) => Self::I8(clamp_bound_i(value, min, max)),
            Self::I16(value) => Self::I16(clamp_bound_i(value, min, max)),
            Self::I32(value) => Self::I32(clamp_bound_i(value, min, max)),
            Self::I64(value) => Self::I64(clamp_bound_i(value, min, max)),
            Self::Isize(value) => Self::Isize(clamp_bound_i(value, min, max)),
            Self::U8(value) => Self::U8(clamp_bound_u(value, min, max)),
            Self::U16(value) => Self::U16(clamp_bound_u(value, min, max)),
            Self::U32(value) => Self::U32(clamp_bound_u(value, min, max)),
            Self::U64(value) => Self::U64(clamp_bound_u(value, min, max)),
            Self::Usize(value) => Self::Usize(clamp_bound_u(value, min, max)),
            Self::F32(value) => Self::F32(clamp_bound_f32(value, min, max)),
            Self::F64(value) => Self::F64(clamp_bound_f64(value, min, max)),
        }
    }

    pub fn cast_to(self, kind: NumberKind, min: Option<Number>, max: Option<Number>) -> Self {
        let value = match kind {
            NumberKind::I8 => Self::I8(clamp_to_signed::<i8>(self)),
            NumberKind::I16 => Self::I16(clamp_to_signed::<i16>(self)),
            NumberKind::I32 => Self::I32(clamp_to_signed::<i32>(self)),
            NumberKind::I64 => Self::I64(clamp_to_signed::<i64>(self)),
            NumberKind::Isize => Self::Isize(clamp_to_signed::<isize>(self)),
            NumberKind::U8 => Self::U8(clamp_to_unsigned::<u8>(self)),
            NumberKind::U16 => Self::U16(clamp_to_unsigned::<u16>(self)),
            NumberKind::U32 => Self::U32(clamp_to_unsigned::<u32>(self)),
            NumberKind::U64 => Self::U64(clamp_to_unsigned::<u64>(self)),
            NumberKind::Usize => Self::Usize(clamp_to_unsigned::<usize>(self)),
            NumberKind::F32 => Self::F32(self.as_f32()),
            NumberKind::F64 => Self::F64(self.as_f64_lossy()),
        };
        value.clamp_bounds(min, max)
    }

    pub fn parse_as(
        kind: NumberKind,
        value: &str,
        min: Option<Number>,
        max: Option<Number>,
    ) -> Option<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }

        let parsed = match kind {
            NumberKind::I8 => Self::I8(trimmed.parse::<i8>().ok()?),
            NumberKind::I16 => Self::I16(trimmed.parse::<i16>().ok()?),
            NumberKind::I32 => Self::I32(trimmed.parse::<i32>().ok()?),
            NumberKind::I64 => Self::I64(trimmed.parse::<i64>().ok()?),
            NumberKind::Isize => Self::Isize(trimmed.parse::<isize>().ok()?),
            NumberKind::U8 => Self::U8(trimmed.parse::<u8>().ok()?),
            NumberKind::U16 => Self::U16(trimmed.parse::<u16>().ok()?),
            NumberKind::U32 => Self::U32(trimmed.parse::<u32>().ok()?),
            NumberKind::U64 => Self::U64(trimmed.parse::<u64>().ok()?),
            NumberKind::Usize => Self::Usize(trimmed.parse::<usize>().ok()?),
            NumberKind::F32 => {
                if matches!(trimmed, "-" | "+" | "." | "-." | "+.") {
                    return None;
                }
                Self::F32(trimmed.parse::<f32>().ok()?)
            }
            NumberKind::F64 => {
                if matches!(trimmed, "-" | "+" | "." | "-." | "+.") {
                    return None;
                }
                Self::F64(trimmed.parse::<f64>().ok()?)
            }
        };
        Some(parsed.clamp_bounds(min, max))
    }

    pub fn format(self) -> String {
        match self {
            Self::I8(value) => value.to_string(),
            Self::I16(value) => value.to_string(),
            Self::I32(value) => value.to_string(),
            Self::I64(value) => value.to_string(),
            Self::Isize(value) => value.to_string(),
            Self::U8(value) => value.to_string(),
            Self::U16(value) => value.to_string(),
            Self::U32(value) => value.to_string(),
            Self::U64(value) => value.to_string(),
            Self::Usize(value) => value.to_string(),
            Self::F32(value) => format_float(value as f64),
            Self::F64(value) => format_float(value),
        }
    }

    pub fn serialize(self) -> String {
        format!("{}:{}", self.kind().name(), self.format())
    }

    pub fn deserialize(raw: &str, min: Option<Number>, max: Option<Number>) -> Option<Self> {
        let (kind, value) = raw.split_once(':')?;
        let kind = NumberKind::from_name(kind)?;
        Self::parse_as(kind, value, min, max)
    }
}

pub trait NumberCast: Sized {
    fn from_number(value: Number) -> Option<Self>;
}

macro_rules! impl_number_cast_signed {
    ($($ty:ty),* $(,)?) => {
        $(
            impl NumberCast for $ty {
                fn from_number(value: Number) -> Option<Self> {
                    Some(clamp_to_signed::<$ty>(value))
                }
            }
        )*
    };
}

macro_rules! impl_number_cast_unsigned {
    ($($ty:ty),* $(,)?) => {
        $(
            impl NumberCast for $ty {
                fn from_number(value: Number) -> Option<Self> {
                    Some(clamp_to_unsigned::<$ty>(value))
                }
            }
        )*
    };
}

impl_number_cast_signed!(i8, i16, i32, i64, isize);
impl_number_cast_unsigned!(u8, u16, u32, u64, usize);

impl NumberCast for f32 {
    fn from_number(value: Number) -> Option<Self> {
        Some(value.as_f32())
    }
}

impl NumberCast for f64 {
    fn from_number(value: Number) -> Option<Self> {
        Some(value.as_f64_lossy())
    }
}

macro_rules! impl_number_from {
    ($($ty:ty => $variant:ident),* $(,)?) => {
        $(
            impl From<$ty> for Number {
                fn from(value: $ty) -> Self {
                    Self::$variant(value)
                }
            }
        )*
    };
}

impl_number_from!(
    i8 => I8,
    i16 => I16,
    i32 => I32,
    i64 => I64,
    isize => Isize,
    u8 => U8,
    u16 => U16,
    u32 => U32,
    u64 => U64,
    usize => Usize,
    f32 => F32,
    f64 => F64,
);

trait BoundedSigned: Copy + Ord {
    const MIN_I128: i128;
    const MAX_I128: i128;
    fn to_i128(self) -> i128;
    fn from_i128_clamped(value: i128) -> Self;
}

trait BoundedUnsigned: Copy + Ord {
    const MAX_U128: u128;
    fn to_u128(self) -> u128;
    fn from_u128_clamped(value: u128) -> Self;
}

macro_rules! impl_bounded_signed {
    ($($ty:ty),* $(,)?) => {
        $(
            impl BoundedSigned for $ty {
                const MIN_I128: i128 = <$ty>::MIN as i128;
                const MAX_I128: i128 = <$ty>::MAX as i128;

                fn to_i128(self) -> i128 {
                    self as i128
                }

                fn from_i128_clamped(value: i128) -> Self {
                    value.clamp(Self::MIN_I128, Self::MAX_I128) as $ty
                }
            }
        )*
    };
}

macro_rules! impl_bounded_unsigned {
    ($($ty:ty),* $(,)?) => {
        $(
            impl BoundedUnsigned for $ty {
                const MAX_U128: u128 = <$ty>::MAX as u128;

                fn to_u128(self) -> u128 {
                    self as u128
                }

                fn from_u128_clamped(value: u128) -> Self {
                    value.min(Self::MAX_U128) as $ty
                }
            }
        )*
    };
}

impl_bounded_signed!(i8, i16, i32, i64, isize);
impl_bounded_unsigned!(u8, u16, u32, u64, usize);

fn clamp_to_signed<T: BoundedSigned>(value: Number) -> T {
    value
        .as_i128()
        .map(T::from_i128_clamped)
        .unwrap_or_else(|| T::from_i128_clamped(0))
}

fn clamp_to_unsigned<T: BoundedUnsigned>(value: Number) -> T {
    value
        .as_u128()
        .map(T::from_u128_clamped)
        .unwrap_or_else(|| T::from_u128_clamped(0))
}

fn float_to_i128(value: f64) -> Option<i128> {
    if !value.is_finite() {
        return None;
    }
    Some(value.round().clamp(i128::MIN as f64, i128::MAX as f64) as i128)
}

fn float_to_u128(value: f64) -> Option<u128> {
    if !value.is_finite() {
        return None;
    }
    Some(value.round().clamp(0.0, u128::MAX as f64) as u128)
}

fn clamp_bound_i<T>(value: T, min: Option<Number>, max: Option<Number>) -> T
where
    T: BoundedSigned,
{
    let mut value_i128 = value.to_i128();
    if let Some(min) = min.and_then(Number::as_i128) {
        value_i128 = value_i128.max(min);
    }
    if let Some(max) = max.and_then(Number::as_i128) {
        value_i128 = value_i128.min(max);
    }
    T::from_i128_clamped(value_i128)
}

fn clamp_bound_u<T>(value: T, min: Option<Number>, max: Option<Number>) -> T
where
    T: BoundedUnsigned,
{
    let mut value_u128 = value.to_u128();
    if let Some(min) = min.and_then(Number::as_u128) {
        value_u128 = value_u128.max(min);
    }
    if let Some(max) = max.and_then(Number::as_u128) {
        value_u128 = value_u128.min(max);
    }
    T::from_u128_clamped(value_u128)
}

fn clamp_bound_f32(value: f32, min: Option<Number>, max: Option<Number>) -> f32 {
    let mut value = value;
    if let Some(min) = min.and_then(NumberCast::from_number) {
        value = value.max(min);
    }
    if let Some(max) = max.and_then(NumberCast::from_number) {
        value = value.min(max);
    }
    value
}

fn clamp_bound_f64(value: f64, min: Option<Number>, max: Option<Number>) -> f64 {
    let mut value = value;
    if let Some(min) = min.and_then(NumberCast::from_number) {
        value = value.max(min);
    }
    if let Some(max) = max.and_then(NumberCast::from_number) {
        value = value.min(max);
    }
    value
}

fn format_float(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        value.to_string()
    }
}

#[derive(Component, Reflect, Clone, Widget, ShowInInspector)]
#[widget("input/number_field", children = "exact(0)")]
#[builder(NumberFieldBuilder)]
pub struct NumberField {
    pub value: Number,
    pub min: Option<Number>,
    pub max: Option<Number>,
    pub step: f64,
    pub disabled: bool,
}

impl Default for NumberField {
    fn default() -> Self {
        Self {
            value: Number::F32(0.0),
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
    pub value: Number,
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
        self.field.value = self
            .field
            .value
            .cast_to(kind, self.field.min, self.field.max);
        self.field.step = default_step(kind);
        self
    }

    pub fn value(mut self, value: impl Into<Number>) -> Self {
        self.field.value =
            value
                .into()
                .cast_to(self.field.value.kind(), self.field.min, self.field.max);
        self
    }

    pub fn min<T>(mut self, min: Option<T>) -> Self
    where
        T: Into<Number>,
    {
        self.field.min = min.map(Into::into);
        self.field.value =
            self.field
                .value
                .cast_to(self.field.value.kind(), self.field.min, self.field.max);
        self
    }

    pub fn max<T>(mut self, max: Option<T>) -> Self
    where
        T: Into<Number>,
    {
        self.field.max = max.map(Into::into);
        self.field.value =
            self.field
                .value
                .cast_to(self.field.value.kind(), self.field.min, self.field.max);
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
            self.field.value.format(),
            self.field.value.kind().placeholder(),
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
    fn spawn_default(
        commands: &mut Commands,
        theme: Option<&crate::core::theme::Theme>,
    ) -> WidgetSpawnResult {
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
        let Some(field_entity) = parents
            .get(change.entity)
            .ok()
            .map(|parent| parent.parent())
        else {
            continue;
        };
        let Ok(mut field) = fields.get_mut(field_entity) else {
            continue;
        };
        let Some(value) = Number::parse_as(field.value.kind(), &change.value, field.min, field.max)
        else {
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
        let Some(field_entity) = parents
            .get(submit.entity)
            .ok()
            .map(|parent| parent.parent())
        else {
            continue;
        };
        let Ok(field) = fields.get(field_entity) else {
            continue;
        };
        let Ok(mut text_field) = text_fields.get_mut(submit.entity) else {
            continue;
        };
        text_field.value = field.value.format();
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
        let next = field.value.format();
        if text_field.value != next {
            text_field.value = next;
            text_field.cursor_pos = text_field.value.chars().count();
            text_field.selection = None;
        }
        text_field.disabled = field.disabled;
        text_field.placeholder = field.value.kind().placeholder().to_owned();
    }
}

fn normalize_number_fields(mut fields: Query<&mut NumberField, Changed<NumberField>>) {
    for mut field in fields.iter_mut() {
        let normalized = field
            .value
            .cast_to(field.value.kind(), field.min, field.max);
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
    match kind {
        NumberKind::F32 | NumberKind::F64 => 1.0,
        _ => 1.0,
    }
}
