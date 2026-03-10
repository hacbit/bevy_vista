use bevy::prelude::*;

use crate::theme::Theme;

use super::*;

pub struct NumericFieldsPlugin;

impl Plugin for NumericFieldsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            I8FieldPlugin,
            I16FieldPlugin,
            I32FieldPlugin,
            I64FieldPlugin,
            IsizeFieldPlugin,
            U8FieldPlugin,
            U16FieldPlugin,
            U32FieldPlugin,
            U64FieldPlugin,
            UsizeFieldPlugin,
            F32FieldPlugin,
            F64FieldPlugin,
        ));
    }
}

macro_rules! define_numeric_field {
    (
        $module:ident,
        $plugin:ident,
        $field:ident,
        $builder:ident,
        $change:ident,
        $marker:ident,
        $path:literal,
        $ty:ty,
        $default_value:expr,
        $default_step:expr,
        $placeholder:literal,
        $parser:ident
    ) => {
        mod $module {
            use super::*;

            pub struct $plugin;

            impl Plugin for $plugin {
                fn build(&self, app: &mut App) {
                    app.add_message::<$change>().add_systems(
                        PostUpdate,
                        (sync_from_text_changes, sync_from_text_submit, sync_visuals).chain(),
                    );
                }
            }

            #[derive(Component, Reflect, Clone, Widget)]
            #[widget($path, children = "exact(0)")]
            #[builder($builder)]
            pub struct $field {
                pub value: $ty,
                pub min: Option<$ty>,
                pub max: Option<$ty>,
                pub step: $ty,
                pub disabled: bool,
            }

            impl Default for $field {
                fn default() -> Self {
                    Self {
                        value: $default_value,
                        min: None,
                        max: None,
                        step: $default_step,
                        disabled: false,
                    }
                }
            }

            #[derive(Component)]
            struct $marker;

            #[derive(Message, EntityEvent)]
            pub struct $change {
                pub entity: Entity,
                pub value: $ty,
            }

            #[derive(Clone)]
            pub struct $builder {
                field: $field,
                width: Val,
                height: Val,
            }

            impl Default for $builder {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl $builder {
                pub fn new() -> Self {
                    Self {
                        field: $field::default(),
                        width: px(120.0),
                        height: px(28.0),
                    }
                }

                pub fn value(mut self, value: $ty) -> Self {
                    self.field.value = value;
                    self
                }

                pub fn min(mut self, min: Option<$ty>) -> Self {
                    self.field.min = min;
                    self
                }

                pub fn max(mut self, max: Option<$ty>) -> Self {
                    self.field.max = max;
                    self
                }

                pub fn step(mut self, step: $ty) -> Self {
                    self.field.step = step;
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
                        stringify!($field),
                        self.field.value.to_string(),
                        $placeholder,
                        self.width,
                        self.height,
                        self.field.disabled,
                        theme,
                        $marker,
                    );
                    commands.entity(root).add_child(input);
                    root
                }
            }

            impl DefaultWidgetBuilder for $builder {
                fn spawn_default(
                    commands: &mut Commands,
                    theme: Option<&crate::theme::Theme>,
                ) -> WidgetSpawnResult {
                    $builder::new().build(commands, theme).into()
                }
            }

            fn sync_from_text_changes(
                mut changes: MessageReader<TextInputChange>,
                parents: Query<&ChildOf>,
                inputs: Query<(), With<$marker>>,
                mut fields: Query<&mut $field>,
                mut out: MessageWriter<$change>,
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
                    let Some(value) = $parser(&change.value, field.min, field.max) else {
                        continue;
                    };
                    if field.value != value {
                        field.value = value;
                        out.write($change {
                            entity: field_entity,
                            value,
                        });
                    }
                }
            }

            fn sync_from_text_submit(
                mut submits: MessageReader<TextInputSubmit>,
                parents: Query<&ChildOf>,
                inputs: Query<(), With<$marker>>,
                fields: Query<&$field>,
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
                    text_field.value = field.value.to_string();
                    text_field.cursor_pos = text_field.value.chars().count();
                    text_field.selection = None;
                }
            }

            fn sync_visuals(
                fields: Query<(&$field, &Children), Changed<$field>>,
                children_query: Query<&Children>,
                input_markers: Query<(), With<$marker>>,
                mut text_fields: Query<&mut TextField>,
            ) {
                for (field, children) in fields.iter() {
                    let Some(input) =
                        find_descendant_with(children, &children_query, &input_markers)
                    else {
                        continue;
                    };
                    let Ok(mut text_field) = text_fields.get_mut(input) else {
                        continue;
                    };
                    let next = field.value.to_string();
                    if text_field.value != next {
                        text_field.value = next;
                        text_field.cursor_pos = text_field.value.chars().count();
                        text_field.selection = None;
                    }
                    text_field.disabled = field.disabled;
                }
            }
        }

        pub use $module::{$builder, $change, $field, $plugin};
    };
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

fn clamp_numeric<T: PartialOrd + Copy>(mut value: T, min: Option<T>, max: Option<T>) -> T {
    if let Some(min) = min
        && value < min
    {
        value = min;
    }
    if let Some(max) = max
        && value > max
    {
        value = max;
    }
    value
}

fn parse_signed_numeric<T: std::str::FromStr + PartialOrd + Copy>(
    value: &str,
    min: Option<T>,
    max: Option<T>,
) -> Option<T> {
    let trimmed = value.trim();
    if trimmed.is_empty() || matches!(trimmed, "-" | "+") {
        return None;
    }
    trimmed
        .parse::<T>()
        .ok()
        .map(|parsed| clamp_numeric(parsed, min, max))
}

fn parse_unsigned_numeric<T: std::str::FromStr + PartialOrd + Copy>(
    value: &str,
    min: Option<T>,
    max: Option<T>,
) -> Option<T> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed
        .parse::<T>()
        .ok()
        .map(|parsed| clamp_numeric(parsed, min, max))
}

fn parse_float_numeric<T: std::str::FromStr + PartialOrd + Copy>(
    value: &str,
    min: Option<T>,
    max: Option<T>,
) -> Option<T> {
    let trimmed = value.trim();
    if trimmed.is_empty() || matches!(trimmed, "-" | "+" | "." | "-." | "+.") {
        return None;
    }
    trimmed
        .parse::<T>()
        .ok()
        .map(|parsed| clamp_numeric(parsed, min, max))
}

define_numeric_field!(
    i8_field,
    I8FieldPlugin,
    I8Field,
    I8FieldBuilder,
    I8FieldChange,
    I8FieldTextInput,
    "input/i8_field",
    i8,
    0,
    1,
    "0",
    parse_signed_numeric
);
define_numeric_field!(
    i16_field,
    I16FieldPlugin,
    I16Field,
    I16FieldBuilder,
    I16FieldChange,
    I16FieldTextInput,
    "input/i16_field",
    i16,
    0,
    1,
    "0",
    parse_signed_numeric
);
define_numeric_field!(
    i32_field,
    I32FieldPlugin,
    I32Field,
    I32FieldBuilder,
    I32FieldChange,
    I32FieldTextInput,
    "input/i32_field",
    i32,
    0,
    1,
    "0",
    parse_signed_numeric
);
define_numeric_field!(
    i64_field,
    I64FieldPlugin,
    I64Field,
    I64FieldBuilder,
    I64FieldChange,
    I64FieldTextInput,
    "input/i64_field",
    i64,
    0,
    1,
    "0",
    parse_signed_numeric
);
define_numeric_field!(
    isize_field,
    IsizeFieldPlugin,
    IsizeField,
    IsizeFieldBuilder,
    IsizeFieldChange,
    IsizeFieldTextInput,
    "input/isize_field",
    isize,
    0,
    1,
    "0",
    parse_signed_numeric
);
define_numeric_field!(
    u8_field,
    U8FieldPlugin,
    U8Field,
    U8FieldBuilder,
    U8FieldChange,
    U8FieldTextInput,
    "input/u8_field",
    u8,
    0,
    1,
    "0",
    parse_unsigned_numeric
);
define_numeric_field!(
    u16_field,
    U16FieldPlugin,
    U16Field,
    U16FieldBuilder,
    U16FieldChange,
    U16FieldTextInput,
    "input/u16_field",
    u16,
    0,
    1,
    "0",
    parse_unsigned_numeric
);
define_numeric_field!(
    u32_field,
    U32FieldPlugin,
    U32Field,
    U32FieldBuilder,
    U32FieldChange,
    U32FieldTextInput,
    "input/u32_field",
    u32,
    0,
    1,
    "0",
    parse_unsigned_numeric
);
define_numeric_field!(
    u64_field,
    U64FieldPlugin,
    U64Field,
    U64FieldBuilder,
    U64FieldChange,
    U64FieldTextInput,
    "input/u64_field",
    u64,
    0,
    1,
    "0",
    parse_unsigned_numeric
);
define_numeric_field!(
    usize_field,
    UsizeFieldPlugin,
    UsizeField,
    UsizeFieldBuilder,
    UsizeFieldChange,
    UsizeFieldTextInput,
    "input/usize_field",
    usize,
    0,
    1,
    "0",
    parse_unsigned_numeric
);
define_numeric_field!(
    f32_field,
    F32FieldPlugin,
    F32Field,
    F32FieldBuilder,
    F32FieldChange,
    F32FieldTextInput,
    "input/f32_field",
    f32,
    0.0,
    1.0,
    "0.0",
    parse_float_numeric
);
define_numeric_field!(
    f64_field,
    F64FieldPlugin,
    F64Field,
    F64FieldBuilder,
    F64FieldChange,
    F64FieldTextInput,
    "input/f64_field",
    f64,
    0.0,
    1.0,
    "0.0",
    parse_float_numeric
);
