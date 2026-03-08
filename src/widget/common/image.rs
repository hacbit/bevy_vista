use super::*;
use bevy::prelude::{Handle, Image};

#[derive(Widget, Reflect, Clone, Default, Debug)]
#[widget("common/image")]
#[builder(ImageBuilder)]
pub struct ImageWidget {
    pub image: Option<Handle<Image>>,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct ImageBuilder {
    widget: ImageWidget,
    width: Val,
    height: Val,
}

impl Default for ImageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageBuilder {
    pub fn new() -> Self {
        Self {
            widget: ImageWidget {
                image: None,
                color: Color::WHITE,
            },
            width: Val::Px(48.0),
            height: Val::Px(48.0),
        }
    }

    pub fn image(mut self, image: Handle<Image>) -> Self {
        self.widget.image = Some(image);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.widget.color = color;
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

    pub fn build(self) -> impl Bundle {
        (
            Name::new("Image"),
            Node {
                width: self.width,
                height: self.height,
                ..default()
            },
            self.widget.image.map(ImageNode::new).unwrap_or_default(),
            BackgroundColor(self.widget.color),
        )
    }
}

impl DefaultWidgetBuilder for ImageBuilder {
    fn spawn_default(commands: &mut Commands, _theme: Option<&crate::theme::Theme>) -> Entity {
        commands.spawn(ImageBuilder::new().build()).id()
    }
}
