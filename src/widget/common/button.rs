use bevy::app::PostUpdate;

use super::*;

pub struct ButtonWidgetPlugin;

impl Plugin for ButtonWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (update_button_widgets_on_changed, button_on_hover),
        );
    }
}

fn update_button_widgets_on_changed(
    mut query: Query<(&ButtonWidget, &mut BackgroundColor), Changed<ButtonWidget>>,
) {
    for (options, mut bg_color) in query.iter_mut() {
        bg_color.0 = options.bg_normal_color;
    }
}

fn button_on_hover(
    mut query: Query<(&mut BackgroundColor, &Interaction, &ButtonWidget), Changed<Interaction>>,
) {
    for (mut bg, interaction, options) in query.iter_mut() {
        let color = match *interaction {
            Interaction::None => options.bg_normal_color,
            Interaction::Hovered => options.bg_hover_color,
            Interaction::Pressed => options.bg_pressed_color,
        };

        if bg.0 != color {
            bg.0 = color;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ButtonBuilder {
    pub text: String,
    pub width: Val,
    pub height: Val,
}

impl Default for ButtonBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ButtonBuilder {
    pub fn new() -> Self {
        Self {
            text: "Button".to_owned(),
            width: Val::Px(120.),
            height: Val::Px(40.),
        }
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = text.to_owned();
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
        self.build_with(ButtonWidget::default())
    }

    pub fn build_with(self, widget: ButtonWidget) -> impl Bundle {
        let normal_color = widget.bg_normal_color;

        (
            Name::new("Button"),
            Button,
            widget,
            Interaction::default(),
            Node {
                width: self.width,
                height: self.height,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::px(2., 2., 1., 1.),
                ..default()
            },
            BackgroundColor(normal_color),
            children![(LabelBuilder::new().text(self.text).build())],
        )
    }
}

impl DefaultWidgetBuilder for ButtonBuilder {
    fn spawn_default(commands: &mut Commands, _theme: Option<&crate::theme::Theme>) -> Entity {
        commands.spawn(ButtonBuilder::new().build()).id()
    }
}

#[derive(Widget, Reflect, Component)]
#[widget("common/button")]
#[builder(ButtonBuilder)]
pub struct ButtonWidget {
    pub bg_normal_color: Color,
    pub bg_hover_color: Color,
    pub bg_pressed_color: Color,
    pub bg_disabled_color: Color,
}

const BG_NORMAL_COLOR: Color = Color::srgb(0.486, 0.486, 0.529);
const BG_HOVER_COLOR: Color = Color::srgb(0.576, 0.576, 0.619);
const BG_PRESSED_COLOR: Color = Color::srgb(0.396, 0.396, 0.439);
const BG_DISABLED_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);

impl Default for ButtonWidget {
    fn default() -> Self {
        Self {
            bg_normal_color: BG_NORMAL_COLOR,
            bg_hover_color: BG_HOVER_COLOR,
            bg_pressed_color: BG_PRESSED_COLOR,
            bg_disabled_color: BG_DISABLED_COLOR,
        }
    }
}
