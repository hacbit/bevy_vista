use super::*;

#[derive(Debug, Reflect, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerAxis {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Component, Reflect, Clone, Widget, ShowInInspector)]
#[widget("layout/divider", children = "exact(0)")]
#[builder(DividerBuilder)]
pub struct Divider {
    #[property(label = "Axis")]
    pub axis: DividerAxis,
    #[property(label = "Thickness")]
    pub thickness: Val,
    #[property(label = "Color")]
    pub color: Option<Color>,
    #[property(label = "Hover Color")]
    pub hover_color: Option<Color>,
}

impl Default for Divider {
    fn default() -> Self {
        Self {
            axis: DividerAxis::Vertical,
            thickness: Val::Px(2.0),
            color: None,
            hover_color: None,
        }
    }
}

#[derive(Clone)]
pub struct DividerBuilder {
    divider: Divider,
}

impl Default for DividerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DividerBuilder {
    pub fn new() -> Self {
        Self {
            divider: Divider::default(),
        }
    }

    pub fn axis(mut self, axis: DividerAxis) -> Self {
        self.divider.axis = axis;
        self
    }

    pub fn thickness(mut self, thickness: Val) -> Self {
        self.divider.thickness = thickness;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.divider.color = Some(color);
        self
    }

    pub fn hover_color(mut self, color: Color) -> Self {
        self.divider.hover_color = Some(color);
        self
    }

    pub fn build(self, theme: Option<&Theme>) -> impl Bundle {
        let divider = self.divider;
        let base_color = resolve_divider_color(theme, &divider);

        let node = match divider.axis {
            DividerAxis::Vertical => Node {
                width: divider.thickness,
                height: Val::Percent(100.),
                flex_grow: 0.0,
                flex_shrink: 0.0,
                ..default()
            },
            DividerAxis::Horizontal => Node {
                width: Val::Percent(100.),
                height: divider.thickness,
                flex_grow: 0.0,
                flex_shrink: 0.0,
                ..default()
            },
        };

        (node, BackgroundColor(base_color), divider)
    }
}

impl DefaultWidgetBuilder for DividerBuilder {
    fn spawn_default(
        commands: &mut Commands,
        theme: Option<&crate::core::theme::Theme>,
    ) -> WidgetSpawnResult {
        commands
            .spawn(DividerBuilder::new().build(theme))
            .id()
            .into()
    }
}

pub fn resolve_divider_color(theme: Option<&Theme>, divider: &Divider) -> Color {
    divider
        .color
        .or_else(|| theme.map(|t| t.palette.outline_variant))
        .unwrap_or(Color::srgb(0.3, 0.3, 0.3))
}

pub fn resolve_divider_hover_color(theme: Option<&Theme>, divider: &Divider) -> Color {
    divider
        .hover_color
        .or_else(|| theme.map(|t| t.palette.primary.with_alpha(0.75)))
        .unwrap_or(Color::srgb(0.45, 0.45, 0.45))
}
