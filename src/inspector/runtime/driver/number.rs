use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use super::*;

pub(super) fn driver() -> Arc<dyn InspectorDriver> {
    Arc::new(NumberInspectorDriver)
}

struct NumberInspectorDriver;

#[derive(Component)]
struct NumberControlRoot {
    numeric_min: Option<f32>,
    value_input: Entity,
    kind_input: Entity,
}

#[derive(Component)]
struct NumberValuePart;

#[derive(Component)]
struct NumberKindPart;

impl InspectorDriver for NumberInspectorDriver {
    fn id(&self) -> InspectorDriverId {
        INSPECTOR_DRIVER_NUMBER
    }

    fn build(
        &self,
        commands: &mut Commands,
        field: &InspectorFieldDescriptor,
        theme: Option<&Theme>,
    ) -> Entity {
        let value_input = NumberFieldBuilder::new()
            .kind(NumberKind::F32)
            .width(px(96.0))
            .height(px(28.0))
            .disabled(true)
            .build(commands, theme);
        let kind_input = DropdownBuilder::new()
            .width(px(84.0))
            .options(number_kind_options())
            .disabled(true)
            .build(commands, theme);
        let owner = commands
            .spawn((
                Name::new(format!("Inspector {} Number Control", field.label)),
                Node {
                    min_width: px(0.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexEnd,
                    column_gap: px(6.0),
                    ..default()
                },
                NumberControlRoot {
                    numeric_min: field.numeric_min,
                    value_input,
                    kind_input,
                },
            ))
            .add_children(&[value_input, kind_input])
            .id();
        commands
            .entity(value_input)
            .insert((InspectorControlOwner { owner }, NumberValuePart));
        commands
            .entity(kind_input)
            .insert((InspectorControlOwner { owner }, NumberKindPart));
        owner
    }

    fn serialize(&self, field: &dyn PartialReflect) -> Option<String> {
        Some(read_number_field(field)?.serialize())
    }

    fn apply_serialized(&self, field: &mut dyn PartialReflect, raw: &str) -> bool {
        parse_number_for_field(field, raw, None)
            .is_some_and(|value| write_number_field(field, value, None))
    }

    fn install_runtime(&self, builder: &mut InspectorDriverRuntimeBuilder) {
        builder.on_apply(apply_inspector_number_value_changes);
        builder.on_apply(apply_inspector_number_kind_changes);
        builder.on_sync(sync_inspector_number_controls);
    }
}

fn apply_inspector_number_value_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<NumberFieldChange>,
    value_inputs: Query<&InspectorControlOwner, With<NumberValuePart>>,
    number_controls: Query<&NumberControlRoot>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }
    for change in changes.read() {
        let Ok(input) = value_inputs.get(change.entity) else {
            continue;
        };
        let Ok(control) = number_controls.get(input.owner) else {
            continue;
        };
        let _ = ctx.write_for(change.entity, |field| {
            write_number_field(field, change.value, control.numeric_min)
        });
    }
}

fn apply_inspector_number_kind_changes(
    mut ctx: InspectorDriverApplyContext,
    mut changes: MessageReader<DropdownChange>,
    kind_inputs: Query<&InspectorControlOwner, With<NumberKindPart>>,
    number_controls: Query<&NumberControlRoot>,
) {
    if !ctx.can_edit() {
        changes.clear();
        return;
    }
    for change in changes.read() {
        let Ok(input) = kind_inputs.get(change.entity) else {
            continue;
        };
        let Ok(control) = number_controls.get(input.owner) else {
            continue;
        };
        let Some(kind) = number_kind_from_index(change.selected) else {
            continue;
        };
        let _ = ctx.write_for(change.entity, |field| {
            write_number_kind_field(field, kind, control.numeric_min)
        });
    }
}

fn sync_inspector_number_controls(
    ctx: InspectorDriverSyncContext,
    number_controls: Query<(Entity, &NumberControlRoot)>,
    mut value_fields: Query<&mut NumberField>,
    mut kind_dropdowns: Query<(&mut Dropdown, &mut Node)>,
) {
    if !ctx.changed() {
        return;
    }
    for (entity, control) in number_controls.iter() {
        if !ctx.is_control(entity, INSPECTOR_DRIVER_NUMBER) {
            continue;
        }

        let value = ctx.read_for(entity, read_number_field);
        let is_number = ctx
            .read_for(entity, |field| {
                Some(field.try_downcast_ref::<Number>().is_some())
            })
            .unwrap_or(false);

        let Some(value) = value else {
            if let Ok(mut field) = value_fields.get_mut(control.value_input) {
                field.value = Number::F32(0.0);
                field.disabled = true;
            }
            if let Ok((mut dropdown, mut node)) = kind_dropdowns.get_mut(control.kind_input) {
                dropdown.options = number_kind_options();
                dropdown.selected = 0;
                dropdown.expanded = false;
                dropdown.disabled = true;
                node.display = Display::None;
            }
            continue;
        };

        if let Ok(mut field) = value_fields.get_mut(control.value_input) {
            field.value = value;
            field.disabled = false;
        }
        if let Ok((mut dropdown, mut node)) = kind_dropdowns.get_mut(control.kind_input) {
            dropdown.options = number_kind_options();
            dropdown.selected = number_kind_index(value.kind());
            dropdown.expanded = false;
            dropdown.disabled = !is_number;
            node.display = if is_number {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

fn number_kind_options() -> Vec<String> {
    [
        NumberKind::I8,
        NumberKind::I16,
        NumberKind::I32,
        NumberKind::I64,
        NumberKind::Isize,
        NumberKind::U8,
        NumberKind::U16,
        NumberKind::U32,
        NumberKind::U64,
        NumberKind::Usize,
        NumberKind::F32,
        NumberKind::F64,
    ]
    .into_iter()
    .map(|kind| kind.name().to_owned())
    .collect()
}

fn number_kind_index(kind: NumberKind) -> usize {
    match kind {
        NumberKind::I8 => 0,
        NumberKind::I16 => 1,
        NumberKind::I32 => 2,
        NumberKind::I64 => 3,
        NumberKind::Isize => 4,
        NumberKind::U8 => 5,
        NumberKind::U16 => 6,
        NumberKind::U32 => 7,
        NumberKind::U64 => 8,
        NumberKind::Usize => 9,
        NumberKind::F32 => 10,
        NumberKind::F64 => 11,
    }
}

fn number_kind_from_index(index: usize) -> Option<NumberKind> {
    match index {
        0 => Some(NumberKind::I8),
        1 => Some(NumberKind::I16),
        2 => Some(NumberKind::I32),
        3 => Some(NumberKind::I64),
        4 => Some(NumberKind::Isize),
        5 => Some(NumberKind::U8),
        6 => Some(NumberKind::U16),
        7 => Some(NumberKind::U32),
        8 => Some(NumberKind::U64),
        9 => Some(NumberKind::Usize),
        10 => Some(NumberKind::F32),
        11 => Some(NumberKind::F64),
        _ => None,
    }
}
