use bevy::prelude::*;
use bevy_vista_macros::ShowInInspector;

use crate::icons::Icons;
use crate::theme::Theme;

use super::*;

pub struct TreeViewPlugin;

impl Plugin for TreeViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, sync_tree_node_child_presence);
    }
}

#[derive(Component, Reflect, Clone, Widget, ShowInInspector)]
#[widget("layout/tree_view")]
#[builder(TreeViewBuilder)]
pub struct TreeView {
    #[property(label = "Indent", min = 0.0)]
    pub indent: f32,
    #[property(label = "Item Gap", min = 0.0)]
    pub item_gap: f32,
}

impl Default for TreeView {
    fn default() -> Self {
        Self {
            indent: 14.0,
            item_gap: 2.0,
        }
    }
}

#[derive(Component)]
pub struct TreeNodeState {
    expanded: bool,
    children_container: Entity,
    caret: Entity,
    has_children: bool,
}

impl TreeNodeState {
    pub const fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub const fn has_children(&self) -> bool {
        self.has_children
    }
}

#[derive(Component)]
pub struct TreeNodeHeader;

#[derive(Component, Copy, Clone)]
pub struct TreeNodeItemId(pub u64);

pub struct TreeViewBuildResult {
    pub root: Entity,
    pub content: Entity,
}

#[derive(Clone, Copy)]
pub struct SpawnedTreeNode {
    pub node: Entity,
    pub header: Entity,
    pub children_container: Entity,
    pub caret: Entity,
    pub label: Entity,
}

#[derive(Component, Copy, Clone)]
struct TreeNodeHeaderStyle {
    normal: Color,
    hover: Color,
    pressed: Color,
}

#[derive(Component, Copy, Clone, Default)]
struct TreeNodeHeaderState {
    hovered: bool,
    pressed: bool,
}

#[derive(Debug, Clone)]
pub struct TreeNodeBuilder {
    pub label: String,
    pub expanded: bool,
    pub children: Vec<TreeNodeBuilder>,
    pub item_id: Option<u64>,
}

impl TreeNodeBuilder {
    pub fn leaf(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            expanded: false,
            children: Vec::new(),
            item_id: None,
        }
    }

    pub fn branch(
        label: impl Into<String>,
        expanded: bool,
        children: Vec<TreeNodeBuilder>,
    ) -> Self {
        Self {
            label: label.into(),
            expanded,
            children,
            item_id: None,
        }
    }

    pub fn with_item_id(mut self, item_id: u64) -> Self {
        self.item_id = Some(item_id);
        self
    }

    pub fn from_paths<'a>(
        label: impl Into<String>,
        expanded: bool,
        paths: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let mut children: Vec<TreeNodeBuilder> = Vec::new();

        for path in paths {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.is_empty() {
                continue;
            }

            let mut level = &mut children;
            for part in parts.into_iter().filter(|p| !p.is_empty()) {
                let index = match level.iter().position(|node| node.label == part) {
                    Some(index) => index,
                    None => {
                        level.push(TreeNodeBuilder {
                            label: part.to_string(),
                            expanded: false,
                            children: Vec::new(),
                            item_id: None,
                        });
                        level.len() - 1
                    }
                };
                level = &mut level[index].children;
            }
        }

        Self {
            label: label.into(),
            expanded,
            children,
            item_id: None,
        }
    }
}

#[derive(Clone)]
pub struct TreeViewBuilder {
    tree: TreeView,
    width: Val,
    height: Val,
    padding: UiRect,
}

impl Default for TreeViewBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeViewBuilder {
    pub fn new() -> Self {
        Self {
            tree: TreeView::default(),
            width: Val::Auto,
            height: Val::Auto,
            padding: UiRect::all(Val::Px(4.0)),
        }
    }

    pub fn indent(mut self, indent: f32) -> Self {
        self.tree.indent = indent.max(0.0);
        self
    }

    pub fn item_gap(mut self, gap: f32) -> Self {
        self.tree.item_gap = gap.max(0.0);
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

    pub fn build(
        self,
        commands: &mut Commands,
        roots: impl IntoIterator<Item = TreeNodeBuilder>,
        theme: Option<&Theme>,
    ) -> Entity {
        self.build_with_result(commands, roots, theme).root
    }

    pub fn build_with_result(
        self,
        commands: &mut Commands,
        roots: impl IntoIterator<Item = TreeNodeBuilder>,
        theme: Option<&Theme>,
    ) -> TreeViewBuildResult {
        let tree = self.tree;
        let indent = tree.indent;
        let item_gap = tree.item_gap;
        let content_entity = commands
            .spawn((
                Name::new("Tree View Content"),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Auto,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(item_gap),
                    padding: self.padding,
                    ..default()
                },
            ))
            .id();

        let root_nodes: Vec<Entity> = roots
            .into_iter()
            .map(|node| spawn_tree_node(commands, node, theme, indent, item_gap).node)
            .collect();
        commands.entity(content_entity).add_children(&root_nodes);
        let root = ScrollViewBuilder::new()
            .width(self.width)
            .height(self.height)
            .show_horizontal(false)
            .vertical_bar(ScrollbarVisibility::Auto)
            .build_with_entities(commands, [content_entity]);
        commands.entity(root).insert(tree);
        TreeViewBuildResult {
            root,
            content: content_entity,
        }
    }
}

impl DefaultWidgetBuilder for TreeViewBuilder {
    fn spawn_default(commands: &mut Commands, theme: Option<&crate::theme::Theme>) -> Entity {
        TreeViewBuilder::new()
            .width(px(240.0))
            .height(px(180.0))
            .build(commands, std::iter::empty::<TreeNodeBuilder>(), theme)
    }
}

pub fn spawn_tree_node(
    commands: &mut Commands,
    node: TreeNodeBuilder,
    theme: Option<&Theme>,
    indent: f32,
    item_gap: f32,
) -> SpawnedTreeNode {
    spawn_tree_node_inner(commands, node, theme, indent, item_gap)
}

fn spawn_tree_node_inner(
    commands: &mut Commands,
    node: TreeNodeBuilder,
    theme: Option<&Theme>,
    indent: f32,
    item_gap: f32,
) -> SpawnedTreeNode {
    let has_children = !node.children.is_empty();
    let item_id = node.item_id;

    let (header_color, hover_color, pressed_color, text_color, text_font) = match theme {
        Some(t) => (
            t.palette.surface_variant,
            t.palette.outline_variant,
            t.palette.primary_container,
            t.palette.on_surface,
            t.typography.body_medium.font.clone(),
        ),
        None => (
            Color::srgb(0.17, 0.17, 0.17),
            Color::srgb(0.25, 0.25, 0.25),
            Color::srgb(0.22, 0.35, 0.52),
            Color::srgb(0.85, 0.85, 0.85),
            TextFont::from_font_size(14.0),
        ),
    };

    let caret_entity = commands
        .spawn((
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(2.)),
                width: Val::Px(16.),
                height: Val::Px(16.),
                ..default()
            },
            Icons::TriangleRight,
            UiTransform::from_rotation(if node.expanded {
                ROT_TO_DOWN
            } else {
                ROT_TO_RIGHT
            }),
        ))
        .id();
    let label_entity = commands
        .spawn((Text::new(node.label), text_font, TextColor(text_color)))
        .id();

    let header_entity = commands
        .spawn((
            Name::new("Tree Node Header"),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                column_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(header_color),
            BorderRadius::all(Val::Px(5.0)),
            TreeNodeHeader,
            TreeNodeHeaderStyle {
                normal: header_color,
                hover: hover_color,
                pressed: pressed_color,
            },
            TreeNodeHeaderState::default(),
        ))
        .add_children(&[caret_entity, label_entity])
        .id();
    if let Some(item_id) = item_id {
        commands
            .entity(header_entity)
            .insert(TreeNodeItemId(item_id));
    }

    let child_entities: Vec<Entity> = node
        .children
        .into_iter()
        .map(|child| spawn_tree_node_inner(commands, child, theme, indent, item_gap).node)
        .collect();

    let children_container = commands
        .spawn((
            Name::new("Tree Node Children"),
            Node {
                display: if has_children && node.expanded {
                    Display::Flex
                } else {
                    Display::None
                },
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(item_gap),
                margin: UiRect::left(Val::Px(indent)),
                ..default()
            },
        ))
        .add_children(&child_entities)
        .id();

    let node_entity = commands
        .spawn((
            Name::new("Tree Node"),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(item_gap),
                ..default()
            },
            TreeNodeState {
                expanded: node.expanded,
                children_container,
                caret: caret_entity,
                has_children,
            },
        ))
        .add_children(&[header_entity, children_container])
        .id();

    commands
        .entity(header_entity)
        .observe(on_tree_header_over)
        .observe(on_tree_header_press)
        .observe(on_tree_header_out)
        .observe(on_tree_header_cancel)
        .observe(on_tree_header_click);

    SpawnedTreeNode {
        node: node_entity,
        header: header_entity,
        children_container,
        caret: caret_entity,
        label: label_entity,
    }
}

const ROT_TO_RIGHT: Rot2 = Rot2::IDENTITY;
const ROT_TO_DOWN: Rot2 = Rot2::FRAC_PI_2;

fn sync_tree_node_child_presence(
    mut states: Query<&mut TreeNodeState>,
    container_children: Query<&Children>,
    mut containers: Query<&mut Node>,
    mut caret_visibility: Query<&mut Visibility>,
) {
    for mut state in states.iter_mut() {
        let has_children = container_children
            .get(state.children_container)
            .map(|children| !children.is_empty())
            .unwrap_or(false);
        if has_children == state.has_children {
            continue;
        }

        state.has_children = has_children;
        if !has_children {
            state.expanded = false;
        }

        if let Ok(mut node) = containers.get_mut(state.children_container) {
            node.display = if has_children && state.expanded {
                Display::Flex
            } else {
                Display::None
            };
        }
        if let Ok(mut visibility) = caret_visibility.get_mut(state.caret) {
            *visibility = if has_children {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn on_tree_header_over(
    event: On<Pointer<Over>>,
    parents: Query<&ChildOf>,
    headers: Query<(), With<TreeNodeHeader>>,
    mut header_bg: Query<
        (
            &TreeNodeHeaderStyle,
            &mut TreeNodeHeaderState,
            &mut BackgroundColor,
        ),
        With<TreeNodeHeader>,
    >,
) {
    let Some(header_entity) = find_ancestor_with(event.event_target(), &parents, |entity| {
        headers.contains(entity)
    }) else {
        return;
    };

    if let Ok((style, mut state, mut bg)) = header_bg.get_mut(header_entity) {
        state.hovered = true;
        if !state.pressed {
            bg.0 = style.hover;
        }
    }
}

fn on_tree_header_press(
    event: On<Pointer<Press>>,
    parents: Query<&ChildOf>,
    headers: Query<(), With<TreeNodeHeader>>,
    mut header_bg: Query<
        (
            &TreeNodeHeaderStyle,
            &mut TreeNodeHeaderState,
            &mut BackgroundColor,
        ),
        With<TreeNodeHeader>,
    >,
) {
    let Some(header_entity) = find_ancestor_with(event.event_target(), &parents, |entity| {
        headers.contains(entity)
    }) else {
        return;
    };

    if let Ok((style, mut state, mut bg)) = header_bg.get_mut(header_entity) {
        state.pressed = true;
        bg.0 = style.pressed;
    }
}

fn on_tree_header_out(
    event: On<Pointer<Out>>,
    parents: Query<&ChildOf>,
    headers: Query<(), With<TreeNodeHeader>>,
    mut header_bg: Query<
        (
            &TreeNodeHeaderStyle,
            &mut TreeNodeHeaderState,
            &mut BackgroundColor,
        ),
        With<TreeNodeHeader>,
    >,
) {
    let Some(header_entity) = find_ancestor_with(event.event_target(), &parents, |entity| {
        headers.contains(entity)
    }) else {
        return;
    };

    if let Ok((style, mut state, mut bg)) = header_bg.get_mut(header_entity) {
        state.hovered = false;
        state.pressed = false;
        bg.0 = style.normal;
    }
}

fn on_tree_header_cancel(
    event: On<Pointer<Cancel>>,
    parents: Query<&ChildOf>,
    headers: Query<(), With<TreeNodeHeader>>,
    mut header_bg: Query<
        (
            &TreeNodeHeaderStyle,
            &mut TreeNodeHeaderState,
            &mut BackgroundColor,
        ),
        With<TreeNodeHeader>,
    >,
) {
    let Some(header_entity) = find_ancestor_with(event.event_target(), &parents, |entity| {
        headers.contains(entity)
    }) else {
        return;
    };

    if let Ok((style, mut state, mut bg)) = header_bg.get_mut(header_entity) {
        state.pressed = false;
        bg.0 = if state.hovered {
            style.hover
        } else {
            style.normal
        };
    }
}

fn on_tree_header_click(
    event: On<Pointer<Click>>,
    parents: Query<&ChildOf>,
    headers: Query<(), With<TreeNodeHeader>>,
    mut header_bg: Query<
        (
            &TreeNodeHeaderStyle,
            &mut TreeNodeHeaderState,
            &mut BackgroundColor,
        ),
        With<TreeNodeHeader>,
    >,
    mut nodes: Query<&mut TreeNodeState>,
    mut node_layout: Query<&mut Node>,
    mut images: Query<&mut UiTransform, With<ImageNode>>,
    container_children: Query<&Children>,
) {
    let Some(header_entity) = find_ancestor_with(event.event_target(), &parents, |entity| {
        headers.contains(entity)
    }) else {
        return;
    };
    let Ok(header_parent) = parents.get(header_entity) else {
        return;
    };
    let node_entity = header_parent.parent();

    if let Ok((style, mut state, mut bg)) = header_bg.get_mut(header_entity) {
        state.pressed = false;
        bg.0 = if state.hovered {
            style.hover
        } else {
            style.normal
        };
    }

    let Ok(mut node_state) = nodes.get_mut(node_entity) else {
        return;
    };
    let has_children = container_children
        .get(node_state.children_container)
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    if !has_children {
        return;
    }

    node_state.expanded = !node_state.expanded;
    if let Ok(mut children_node) = node_layout.get_mut(node_state.children_container) {
        children_node.display = if node_state.expanded {
            Display::Flex
        } else {
            Display::None
        };
    }

    if let Ok(mut caret) = images.get_mut(node_state.caret) {
        caret.rotation = if node_state.expanded {
            ROT_TO_DOWN
        } else {
            ROT_TO_RIGHT
        };
    }
}

fn find_ancestor_with<F>(
    mut current: Entity,
    parents: &Query<&ChildOf>,
    mut predicate: F,
) -> Option<Entity>
where
    F: FnMut(Entity) -> bool,
{
    loop {
        if predicate(current) {
            return Some(current);
        }
        let Ok(parent) = parents.get(current) else {
            return None;
        };
        current = parent.parent();
    }
}
