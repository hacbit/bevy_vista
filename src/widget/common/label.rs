use super::*;

#[derive(Widget, Reflect, Clone, Debug)]
#[widget("common/label")]
#[builder(LabelBuilder)]
pub struct LabelWidget {
    pub text: String,
    pub font_size: f32,
    pub color: Color,
}

impl Default for LabelWidget {
    fn default() -> Self {
        Self {
            text: "Label".to_owned(),
            font_size: 14.0,
            color: Color::srgb(0.9, 0.9, 0.9),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LabelBuilder {
    widget: LabelWidget,
}

impl Default for LabelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LabelBuilder {
    pub fn new() -> Self {
        Self {
            widget: LabelWidget::default(),
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.widget.text = text.into();
        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.widget.font_size = font_size.max(1.0);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.widget.color = color;
        self
    }

    pub fn build(self) -> impl Bundle {
        (
            // Name::new("Label"),
            Label,
            Text(self.widget.text),
            TextFont {
                font_size: self.widget.font_size,
                ..default()
            },
            TextColor(self.widget.color),
        )
    }
}

impl DefaultWidgetBuilder for LabelBuilder {
    fn spawn_default(commands: &mut Commands, _theme: Option<&crate::theme::Theme>) -> Entity {
        commands.spawn(LabelBuilder::new().build()).id()
    }
}
