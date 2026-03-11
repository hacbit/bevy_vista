use bevy::prelude::*;
use bevy::reflect::PartialReflect;

use crate::core::icons::Icons;
use crate::core::inspector::runtime::{
    InspectorBindingTarget, InspectorContentRoot, InspectorControlRegistry,
    InspectorFieldDecoration, InspectorFieldLabel, InspectorFieldRow, InspectorNameField,
    InspectorPanelState, InspectorResetButton, InspectorWidgetSectionRoot,
    InspectorWidgetSectionState, apply_style_change, clear_widget_prop_change, find_ancestor_with,
};
use crate::core::inspector::{
    InspectorEditorRegistry, InspectorEntryDescriptor, InspectorFieldDescriptor,
    InspectorHeaderDescriptor, WidgetBlueprintDocument, read_reflect_path, read_reflect_path_mut,
};
use crate::core::theme::{EditorTheme, Theme};
use crate::core::widget::{
    ButtonWidget, FoldoutBuilder, LabelBuilder, ScrollViewBuilder, TextFieldBuilder,
    WidgetRegistry, WidgetStyle,
};

use super::VistaMarker::Inspector;
use super::resources::VistaEditorViewOptions;

pub(crate) fn init_inspector_panel(
    mut commands: Commands,
    inspector: Single<Entity, With<Inspector>>,
    editor_theme: Res<EditorTheme>,
    registry: Res<InspectorEditorRegistry>,
    control_registry: Res<InspectorControlRegistry>,
) {
    let theme = Some(&editor_theme.0);
    let panel_bg = editor_theme.0.palette.surface;
    let text_color = editor_theme.0.palette.on_surface;
    let font_size = editor_theme.0.typography.body_medium.font.font_size;

    commands
        .entity(*inspector)
        .insert(BackgroundColor(panel_bg));

    let name_label = commands
        .spawn((
            Name::new("Inspector Name Label"),
            LabelBuilder::new()
                .text("Name")
                .font_size(font_size)
                .color(text_color)
                .build(),
        ))
        .id();
    let name_field = TextFieldBuilder::new()
        .width(percent(100.0))
        .height(px(28.0))
        .disabled(true)
        .build(&mut commands, theme);
    commands.entity(name_field).insert(InspectorNameField);

    let name_row = commands
        .spawn((
            Name::new("Inspector Name Row"),
            Node {
                width: percent(100.0),
                min_width: px(0.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: px(8.0),
                ..default()
            },
        ))
        .add_children(&[name_label, name_field])
        .id();

    let widget_section = commands
        .spawn((
            Name::new("Inspector Widget Section"),
            Node {
                width: percent(100.0),
                min_width: px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(8.0),
                display: Display::None,
                ..default()
            },
            InspectorWidgetSectionRoot,
            InspectorWidgetSectionState::default(),
        ))
        .id();

    let style_header = commands
        .spawn((
            Name::new("Inspector Inline Style Header"),
            Node {
                width: percent(100.0),
                min_width: px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((LabelBuilder::new()
                .text("Inline Style")
                .font_size(font_size)
                .color(text_color)
                .build(),));
            parent.spawn((
                Node {
                    width: percent(100.0),
                    height: px(1.0),
                    ..default()
                },
                BackgroundColor(editor_theme.0.palette.outline_variant),
            ));
        })
        .id();

    let property_list_content = build_property_entries(
        &mut commands,
        registry.entries_for::<WidgetStyle>(),
        &control_registry,
        theme,
        font_size,
        text_color,
        InspectorBindingTarget::Style,
    );
    let property_list = ScrollViewBuilder::new()
        .width(percent(100.0))
        .height(percent(100.0))
        .show_horizontal(false)
        .build_with_entities(&mut commands, [property_list_content]);
    commands
        .entity(property_list)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.min_width = px(0.0);
            node.min_height = px(0.0);
            node.flex_grow = 1.0;
            node.flex_shrink = 1.0;
        });

    let content_root = commands
        .spawn((
            Name::new("Inspector Content Root"),
            Node {
                width: percent(100.0),
                height: percent(100.0),
                padding: UiRect::all(px(8.0)),
                min_width: px(0.0),
                min_height: px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(8.0),
                display: Display::None,
                ..default()
            },
            InspectorContentRoot,
        ))
        .add_children(&[name_row, widget_section, style_header, property_list])
        .id();

    commands.entity(*inspector).add_child(content_root);
}

pub(crate) fn build_property_entries(
    commands: &mut Commands,
    entries: Vec<InspectorEntryDescriptor>,
    control_registry: &InspectorControlRegistry,
    theme: Option<&Theme>,
    font_size: f32,
    text_color: Color,
    target: InspectorBindingTarget,
) -> Entity {
    struct GroupFrame {
        header: InspectorHeaderDescriptor,
        children: Vec<Entity>,
    }

    let mut root_children = Vec::new();
    let mut group_stack: Vec<GroupFrame> = Vec::new();

    let finish_group =
        |commands: &mut Commands, target_children: &mut Vec<Entity>, frame: GroupFrame| {
            let header = frame.header;
            let group_children = frame.children;
            let content = commands
                .spawn((
                    Name::new(format!("Inspector {} Group Content", header.title)),
                    Node {
                        width: percent(100.0),
                        min_width: px(0.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4.0),
                        ..default()
                    },
                ))
                .id();
            if !group_children.is_empty() {
                commands.entity(content).add_children(&group_children);
            }

            let foldout = FoldoutBuilder::new(header.title)
                .expanded(header.default_open)
                .width(percent(100.0))
                .build_with_entity(commands, content, theme);
            target_children.push(foldout);
        };

    for entry in entries {
        match entry {
            InspectorEntryDescriptor::Header(header) => {
                if header.implicit_close_previous
                    && group_stack
                        .last()
                        .is_some_and(|frame| frame.header.implicit_close_previous)
                {
                    let frame = group_stack.pop().expect("checked group_stack is not empty");
                    if let Some(parent) = group_stack.last_mut() {
                        finish_group(commands, &mut parent.children, frame);
                    } else {
                        finish_group(commands, &mut root_children, frame);
                    }
                }
                group_stack.push(GroupFrame {
                    header,
                    children: Vec::new(),
                });
            }
            InspectorEntryDescriptor::Field(field) => {
                let row = spawn_property_row(
                    commands,
                    &field,
                    control_registry,
                    theme,
                    font_size,
                    text_color,
                    target.clone(),
                );
                if let Some(group) = group_stack.last_mut() {
                    group.children.push(row);
                } else {
                    root_children.push(row);
                }
            }
            InspectorEntryDescriptor::EndHeader => {
                let Some(frame) = group_stack.pop() else {
                    continue;
                };
                if let Some(parent) = group_stack.last_mut() {
                    finish_group(commands, &mut parent.children, frame);
                } else {
                    finish_group(commands, &mut root_children, frame);
                }
            }
        }
    }

    while let Some(frame) = group_stack.pop() {
        if let Some(parent) = group_stack.last_mut() {
            finish_group(commands, &mut parent.children, frame);
        } else {
            finish_group(commands, &mut root_children, frame);
        }
    }

    let root = commands
        .spawn((
            Name::new("Inspector Property List"),
            Node {
                width: percent(100.0),
                min_width: px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.0),
                ..default()
            },
        ))
        .id();
    if !root_children.is_empty() {
        commands.entity(root).add_children(&root_children);
    }
    root
}

pub(crate) fn spawn_property_row(
    commands: &mut Commands,
    field: &InspectorFieldDescriptor,
    control_registry: &InspectorControlRegistry,
    theme: Option<&Theme>,
    font_size: f32,
    text_color: Color,
    target: InspectorBindingTarget,
) -> Entity {
    let decoration = InspectorFieldDecoration {
        field_path: field.field_path.clone(),
        target: target.clone(),
    };
    let label = commands
        .spawn((
            Name::new(format!("Inspector {} Label", field.label)),
            LabelBuilder::new()
                .text(field.label.clone())
                .font_size(font_size)
                .color(text_color)
                .build(),
            decoration.clone(),
            InspectorFieldLabel,
        ))
        .id();
    let control = control_registry.build(commands, field, theme, target);
    let button_widget = ButtonWidget::default();
    let reset_button = commands
        .spawn((
            Button,
            button_widget.clone(),
            Interaction::default(),
            Node {
                width: px(28.0),
                height: px(24.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::px(2.0, 2.0, 1.0, 1.0),
                ..default()
            },
            BackgroundColor(button_widget.bg_normal_color),
            Name::new(format!("Inspector {} Reset Button", field.label)),
            decoration.clone(),
            InspectorResetButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: px(14.0),
                    height: px(14.0),
                    ..default()
                },
                Icons::Undo,
            ));
        })
        .observe(on_inspector_reset_button_click)
        .id();
    commands
        .entity(reset_button)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.display = Display::None;
        });
    let control_cluster = commands
        .spawn((
            Name::new(format!("Inspector {} Controls", field.label)),
            Node {
                min_width: px(0.0),
                align_items: AlignItems::Center,
                column_gap: px(6.0),
                ..default()
            },
        ))
        .add_child(control)
        .with_children(|parent| {
            parent
                .spawn((Node {
                    width: px(38.0),
                    min_width: px(38.0),
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .add_child(reset_button);
        })
        .id();
    commands
        .spawn((
            Name::new(format!("Inspector {} Row", field.label)),
            Node {
                width: percent(100.0),
                min_width: px(0.0),
                padding: UiRect::left(px(6.0)),
                border: UiRect::left(px(3.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: px(8.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            decoration,
            InspectorFieldRow,
        ))
        .add_children(&[label, control_cluster])
        .id()
}

pub(crate) fn on_inspector_reset_button_click(
    mut event: On<Pointer<Click>>,
    options: Res<VistaEditorViewOptions>,
    panel_state: Res<InspectorPanelState>,
    mut document: ResMut<WidgetBlueprintDocument>,
    widget_registry: Res<WidgetRegistry>,
    buttons: Query<&InspectorFieldDecoration, With<InspectorResetButton>>,
    parents: Query<&ChildOf>,
) {
    if options.is_preview_mode {
        return;
    }
    let Some(button_entity) = find_ancestor_with(event.event_target(), &parents, |entity| {
        buttons.contains(entity)
    }) else {
        return;
    };
    let Ok(decoration) = buttons.get(button_entity) else {
        return;
    };
    let Some(node_id) = panel_state.selected_node else {
        return;
    };

    match decoration.target {
        InspectorBindingTarget::Style => {
            let Some(mut style) = document.nodes.get(&node_id).map(|node| node.style.clone())
            else {
                return;
            };
            let default_style = WidgetStyle::default();
            let style_reflect: &mut dyn PartialReflect = &mut style;
            let Some(field) = read_reflect_path_mut(style_reflect, &decoration.field_path) else {
                return;
            };
            let Some(default_field) = read_reflect_path(&default_style, &decoration.field_path)
            else {
                return;
            };
            field.apply(default_field);
            apply_style_change(node_id, style, &mut document, &widget_registry);
        }
        InspectorBindingTarget::WidgetProp => {
            clear_widget_prop_change(
                node_id,
                &decoration.field_path,
                &mut document,
                &widget_registry,
            );
        }
    }

    event.propagate(false);
}
