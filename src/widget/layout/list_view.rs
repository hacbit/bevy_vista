use bevy::prelude::*;

use crate::theme::Theme;

use super::*;

pub struct ListViewPlugin;

impl Plugin for ListViewPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Component, Reflect, Clone, Widget)]
#[widget("layout/list_view")]
#[builder(ListViewBuilder)]
pub struct ListView {
    pub direction: FlexDirection,
    pub item_gap: f32,
    pub selectable: bool,
}

impl Default for ListView {
    fn default() -> Self {
        Self {
            direction: FlexDirection::Column,
            item_gap: 4.0,
            selectable: true,
        }
    }
}

#[derive(Component, Reflect, Clone)]
pub struct ListViewItem {
    pub selected: bool,
    pub hovered: bool,
    pub normal_color: Color,
    pub hover_color: Color,
    pub selected_color: Color,
}

#[derive(Clone)]
pub struct ListViewBuilder {
    list: ListView,
    width: Val,
    height: Val,
    padding: UiRect,
}

impl Default for ListViewBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ListViewBuilder {
    pub fn new() -> Self {
        Self {
            list: ListView::default(),
            width: Val::Auto,
            height: Val::Auto,
            padding: UiRect::all(Val::Px(4.0)),
        }
    }

    pub fn direction(mut self, direction: FlexDirection) -> Self {
        self.list.direction = direction;
        self
    }

    pub fn item_gap(mut self, gap: f32) -> Self {
        self.list.item_gap = gap.max(0.0);
        self
    }

    pub fn selectable(mut self, selectable: bool) -> Self {
        self.list.selectable = selectable;
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

    pub fn padding(mut self, padding: UiRect) -> Self {
        self.padding = padding;
        self
    }

    pub fn build_with_entities(
        self,
        commands: &mut Commands,
        items: impl IntoIterator<Item = Entity>,
    ) -> Entity {
        let list = self.list;
        let content = commands
            .spawn((
                Name::new("List View Content"),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Auto,
                    flex_direction: list.direction,
                    row_gap: Val::Px(list.item_gap),
                    column_gap: Val::Px(list.item_gap),
                    padding: self.padding,
                    ..default()
                },
            ))
            .id();
        let children: Vec<Entity> = items.into_iter().collect();
        commands.entity(content).add_children(&children);

        let root = ScrollViewBuilder::new()
            .width(self.width)
            .height(self.height)
            .show_horizontal(false)
            .vertical_bar(ScrollbarVisibility::Auto)
            .build_with_entities(commands, [content]);
        commands.entity(root).insert(list);
        root
    }

    pub fn build_text_items<'a>(
        self,
        commands: &mut Commands,
        labels: impl IntoIterator<Item = &'a str>,
        theme: Option<&Theme>,
    ) -> Entity {
        let item_entities: Vec<Entity> = labels
            .into_iter()
            .map(|label| spawn_text_item(commands, label, theme))
            .collect();
        self.build_with_entities(commands, item_entities)
    }
}

impl DefaultWidgetBuilder for ListViewBuilder {
    fn spawn_default(commands: &mut Commands, _theme: Option<&crate::theme::Theme>) -> Entity {
        ListViewBuilder::new()
            .width(px(220.0))
            .height(px(120.0))
            .build_with_entities(commands, std::iter::empty::<Entity>())
    }
}

fn spawn_text_item(commands: &mut Commands, label: &str, theme: Option<&Theme>) -> Entity {
    let (normal, hover, selected, text_color, font) = match theme {
        Some(t) => (
            t.palette.surface_variant,
            t.palette.outline_variant,
            t.palette.primary_container,
            t.palette.on_surface,
            t.typography.body_medium.font.clone(),
        ),
        None => (
            Color::srgb(0.18, 0.18, 0.18),
            Color::srgb(0.25, 0.25, 0.25),
            Color::srgb(0.22, 0.35, 0.52),
            Color::srgb(0.85, 0.85, 0.85),
            TextFont::from_font_size(14.0),
        ),
    };

    commands
        .spawn((
            Name::new("List Item"),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(normal),
            BorderRadius::all(Val::Px(6.0)),
            ListViewItem {
                selected: false,
                hovered: false,
                normal_color: normal,
                hover_color: hover,
                selected_color: selected,
            },
            children![(Text::new(label.to_owned()), font, TextColor(text_color),)],
        ))
        .observe(on_item_over)
        .observe(on_item_out)
        .observe(on_item_click)
        .id()
}

fn on_item_over(
    event: On<Pointer<Over>>,
    mut items: Query<(&mut ListViewItem, &mut BackgroundColor)>,
) {
    if let Ok((mut item, mut bg)) = items.get_mut(event.event_target()) {
        item.hovered = true;
        if !item.selected {
            bg.0 = item.hover_color;
        }
    }
}

fn on_item_out(
    event: On<Pointer<Out>>,
    mut items: Query<(&mut ListViewItem, &mut BackgroundColor)>,
) {
    if let Ok((mut item, mut bg)) = items.get_mut(event.event_target()) {
        item.hovered = false;
        if !item.selected {
            bg.0 = item.normal_color;
        }
    }
}

fn on_item_click(
    event: On<Pointer<Click>>,
    parents: Query<&ChildOf>,
    lists: Query<&ListView>,
    mut items: Query<(&mut ListViewItem, &mut BackgroundColor)>,
) {
    let mut cursor = event.event_target();
    let mut selectable = true;
    loop {
        if let Ok(list) = lists.get(cursor) {
            selectable = list.selectable;
            break;
        }
        let Ok(parent) = parents.get(cursor) else {
            break;
        };
        cursor = parent.parent();
    }
    if !selectable {
        return;
    }

    if let Ok((mut item, mut bg)) = items.get_mut(event.event_target()) {
        item.selected = !item.selected;
        bg.0 = if item.selected {
            item.selected_color
        } else if item.hovered {
            item.hover_color
        } else {
            item.normal_color
        };
    }
}
