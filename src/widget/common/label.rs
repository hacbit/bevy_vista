use super::*;
use bevy::app::PostUpdate;
use bevy::prelude::Component;
use bevy_vista_macros::ShowInInspector;

pub struct LabelWidgetPlugin;

impl Plugin for LabelWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, sync_label_widgets);
    }
}

#[derive(Widget, Reflect, Component, Clone, Debug, ShowInInspector)]
#[widget("common/label")]
#[builder(LabelBuilder)]
pub struct LabelWidget {
    pub text: String,
    #[property(label = "Font Size", min = 1.0)]
    pub font_size: f32,
    #[property(label = "Color")]
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
        let widget = self.widget;
        (
            widget.clone(),
            Label,
            Text(widget.text),
            TextFont {
                font_size: widget.font_size,
                ..default()
            },
            TextColor(widget.color),
        )
    }
}

impl DefaultWidgetBuilder for LabelBuilder {
    fn spawn_default(commands: &mut Commands, _theme: Option<&crate::theme::Theme>) -> Entity {
        commands.spawn(LabelBuilder::new().build()).id()
    }
}

fn sync_label_widgets(
    mut query: Query<
        (&LabelWidget, &mut Text, &mut TextFont, &mut TextColor),
        Changed<LabelWidget>,
    >,
) {
    for (widget, mut text, mut font, mut color) in query.iter_mut() {
        if text.0 != widget.text {
            text.0 = widget.text.clone();
        }
        if font.font_size != widget.font_size {
            font.font_size = widget.font_size;
        }
        if color.0 != widget.color {
            color.0 = widget.color;
        }
    }
}
