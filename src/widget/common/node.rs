use super::*;

#[derive(Widget, Reflect, Clone, Default)]
#[widget("common/node")]
#[builder(NodeBuilder)]
pub struct NodeWidget;

#[derive(Debug, Clone)]
pub struct NodeBuilder {
    width: Val,
    height: Val,
    flex_direction: FlexDirection,
    align_items: AlignItems,
    justify_content: JustifyContent,
    background: Option<Color>,
}

impl Default for NodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeBuilder {
    pub fn new() -> Self {
        Self {
            width: Val::Px(120.0),
            height: Val::Px(80.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            background: None,
        }
    }

    pub fn width(mut self, width: Val) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Val) -> Self {
        self.height = height;
        self
    }

    pub fn flex_direction(mut self, direction: FlexDirection) -> Self {
        self.flex_direction = direction;
        self
    }

    pub fn align_items(mut self, align_items: AlignItems) -> Self {
        self.align_items = align_items;
        self
    }

    pub fn justify_content(mut self, justify_content: JustifyContent) -> Self {
        self.justify_content = justify_content;
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn build(self) -> impl Bundle {
        (
            Name::new("Node"),
            Node {
                width: self.width,
                height: self.height,
                flex_direction: self.flex_direction,
                align_items: self.align_items,
                justify_content: self.justify_content,
                ..default()
            },
            BackgroundColor(self.background.unwrap_or(Color::NONE)),
        )
    }
}

impl DefaultWidgetBuilder for NodeBuilder {
    fn spawn_default(commands: &mut Commands, _theme: Option<&crate::theme::Theme>) -> Entity {
        commands.spawn(NodeBuilder::new().build()).id()
    }
}
