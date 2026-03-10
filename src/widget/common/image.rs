use super::*;

pub struct ImageWidgetPlugin;

impl Plugin for ImageWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, sync_image_widgets);
    }
}

#[derive(Widget, Reflect, Component, Clone, Default, Debug, ShowInInspector)]
#[widget("common/image", children = "exact(0)")]
#[builder(ImageBuilder)]
pub struct ImageWidget {
    #[property(hidden)]
    pub image: Option<Handle<Image>>,
    #[property(label = "Color")]
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
        let widget = self.widget;
        (
            widget.clone(),
            Node {
                width: self.width,
                height: self.height,
                ..default()
            },
            widget.image.map(ImageNode::new).unwrap_or_default(),
            BackgroundColor(widget.color),
        )
    }
}

impl DefaultWidgetBuilder for ImageBuilder {
    fn spawn_default(
        commands: &mut Commands,
        _theme: Option<&crate::theme::Theme>,
    ) -> WidgetSpawnResult {
        commands.spawn(ImageBuilder::new().build()).id().into()
    }
}

fn sync_image_widgets(
    mut query: Query<(&ImageWidget, &mut ImageNode, &mut BackgroundColor), Changed<ImageWidget>>,
) {
    for (widget, mut image, mut background) in query.iter_mut() {
        image.image = widget.image.clone().unwrap_or_default();
        if background.0 != widget.color {
            background.0 = widget.color;
        }
    }
}
