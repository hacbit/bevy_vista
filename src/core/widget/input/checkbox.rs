use super::*;

pub struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CheckboxChange>()
            .add_systems(PostUpdate, sync_checkbox_interaction);
    }
}

#[derive(Component, Reflect, Clone, Widget, ShowInInspector)]
#[widget("input/checkbox", children = "exact(0)")]
#[builder(CheckboxBuilder)]
pub struct Checkbox {
    #[property(label = "Checked")]
    pub checked: bool,
    #[property(label = "Disabled")]
    pub disabled: bool,
}

impl Default for Checkbox {
    fn default() -> Self {
        Self {
            checked: false,
            disabled: false,
        }
    }
}

#[derive(Component)]
struct CheckboxMark;

#[derive(Component, Copy, Clone)]
struct CheckboxColors {
    normal_bg: Color,
    hovered_bg: Color,
    pressed_bg: Color,
    active_bg: Color,
    border: Color,
    mark: Color,
    disabled_bg: Color,
}

#[derive(Message, EntityEvent)]
pub struct CheckboxChange {
    pub entity: Entity,
    pub checked: bool,
}

#[derive(Clone)]
pub struct CheckboxBuilder {
    checkbox: Checkbox,
    size: f32,
}

impl Default for CheckboxBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckboxBuilder {
    pub fn new() -> Self {
        Self {
            checkbox: Checkbox::default(),
            size: 20.0,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checkbox.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.checkbox.disabled = disabled;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(12.0);
        self
    }

    pub fn build(self, commands: &mut Commands, theme: Option<&Theme>) -> Entity {
        let colors = checkbox_colors(theme);
        let mark = commands
            .spawn((
                Name::new("Checkbox Mark"),
                Node {
                    width: px((self.size - 6.0).max(8.0)),
                    height: px((self.size - 6.0).max(8.0)),
                    ..default()
                },
                BackgroundColor(colors.mark),
                Visibility::Hidden,
                CheckboxMark,
                Icons::Checkmark,
            ))
            .id();
        let entity = commands
            .spawn((
                Node {
                    width: px(self.size),
                    height: px(self.size),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                Button,
                Interaction::default(),
                self.checkbox,
                colors,
                BackgroundColor(colors.normal_bg),
                BorderColor::all(colors.border),
                BorderRadius::all(px(4.0)),
            ))
            .add_child(mark)
            .observe(on_checkbox_click)
            .id();
        entity
    }
}

impl DefaultWidgetBuilder for CheckboxBuilder {
    fn spawn_default(
        commands: &mut Commands,
        theme: Option<&crate::core::theme::Theme>,
    ) -> WidgetSpawnResult {
        CheckboxBuilder::new().build(commands, theme).into()
    }
}

fn on_checkbox_click(
    mut event: On<Pointer<Click>>,
    mut checkboxes: Query<&mut Checkbox>,
    mut out: MessageWriter<CheckboxChange>,
) {
    let Ok(mut checkbox) = checkboxes.get_mut(event.entity) else {
        return;
    };
    if checkbox.disabled {
        return;
    }
    checkbox.checked = !checkbox.checked;
    out.write(CheckboxChange {
        entity: event.entity,
        checked: checkbox.checked,
    });
    event.propagate(false);
}

fn sync_checkbox_interaction(
    mut query: Query<
        (
            Entity,
            &Checkbox,
            &CheckboxColors,
            &Interaction,
            &mut BackgroundColor,
            &Children,
        ),
        (
            Or<(Changed<Checkbox>, Changed<Interaction>)>,
            Without<CheckboxMark>,
        ),
    >,
    mut mark_query: Query<(&mut Visibility, &mut BackgroundColor), With<CheckboxMark>>,
) {
    for (_entity, checkbox, colors, interaction, mut background, children) in query.iter_mut() {
        background.0 = if checkbox.disabled {
            colors.disabled_bg
        } else if checkbox.checked {
            colors.active_bg
        } else {
            match *interaction {
                Interaction::Pressed => colors.pressed_bg,
                Interaction::Hovered => colors.hovered_bg,
                Interaction::None => colors.normal_bg,
            }
        };

        for child in children.iter() {
            if let Ok((mut visibility, mut mark_bg)) = mark_query.get_mut(child) {
                *visibility = if checkbox.checked {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                mark_bg.0 = colors.mark;
            }
        }
    }
}

fn checkbox_colors(theme: Option<&Theme>) -> CheckboxColors {
    match theme {
        Some(t) => CheckboxColors {
            normal_bg: t.palette.surface,
            hovered_bg: t.palette.surface_variant,
            pressed_bg: t.palette.outline_variant,
            active_bg: t.palette.primary_container,
            border: t.palette.outline,
            mark: t.palette.on_primary_container,
            disabled_bg: t.palette.disabled_container,
        },
        None => CheckboxColors {
            normal_bg: Color::srgb(0.15, 0.15, 0.15),
            hovered_bg: Color::srgb(0.22, 0.22, 0.22),
            pressed_bg: Color::srgb(0.3, 0.3, 0.3),
            active_bg: Color::srgb(0.22, 0.35, 0.52),
            border: Color::srgb(0.4, 0.4, 0.4),
            mark: Color::WHITE,
            disabled_bg: Color::srgb(0.1, 0.1, 0.1),
        },
    }
}
