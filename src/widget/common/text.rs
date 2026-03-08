use super::*;

pub struct TextBuilder {
    pub text: String,
}

impl TextBuilder {
    pub fn new() -> Self {
        Self {
            text: "Text".to_owned(),
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    pub fn build(self) -> impl Bundle {
        (
            Name::new("Text"),
            Text(self.text),
            TextFont {
                font_size: 14.0,
                ..default()
            },
        )
    }
}

impl DefaultWidgetBuilder for TextBuilder {
    fn spawn_default(commands: &mut Commands, _theme: Option<&crate::theme::Theme>) -> Entity {
        commands.spawn(TextBuilder::new().build()).id()
    }
}

#[derive(Widget, Clone, Default)]
#[widget("common/text")]
#[builder(TextBuilder)]
pub struct TextWidget;
