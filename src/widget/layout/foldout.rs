use bevy::prelude::*;

use crate::{
    icons::{Icons, IconsManager},
    theme::Theme,
};

use super::*;

pub struct FoldoutPlugin;

impl Plugin for FoldoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, initialize_foldout_caret);
    }
}

#[derive(Component, Reflect, Clone, Widget)]
#[widget("layout/foldout")]
#[builder(FoldoutBuilder)]
pub struct Foldout {
    pub expanded: bool,
}

impl Default for Foldout {
    fn default() -> Self {
        Self { expanded: true }
    }
}

#[derive(Component)]
struct FoldoutState {
    expanded: bool,
    content_wrapper: Entity,
    caret: Entity,
}

#[derive(Component)]
struct FoldoutHeader;

#[derive(Clone)]
pub struct FoldoutBuilder {
    foldout: Foldout,
    title: String,
    width: Val,
}

impl Default for FoldoutBuilder {
    fn default() -> Self {
        Self::new("Foldout")
    }
}

impl FoldoutBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            foldout: Foldout::default(),
            title: title.into(),
            width: Val::Percent(100.0),
        }
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.foldout.expanded = expanded;
        self
    }

    pub fn width(mut self, width: Val) -> Self {
        self.width = width;
        self
    }

    pub fn build_with_entity(
        self,
        commands: &mut Commands,
        content: Entity,
        theme: Option<&Theme>,
    ) -> Entity {
        let foldout = self.foldout;
        let (header_bg, text_color, font) = match theme {
            Some(t) => (
                t.palette.surface_variant,
                t.palette.on_surface,
                t.typography.body_medium.font.clone(),
            ),
            None => (
                Color::srgb(0.18, 0.18, 0.18),
                Color::srgb(0.85, 0.85, 0.85),
                TextFont::from_font_size(14.0),
            ),
        };

        let caret = commands.spawn_empty().id();
        let title = commands
            .spawn((Text::new(self.title), font, TextColor(text_color)))
            .id();

        let header = commands
            .spawn((
                Name::new("Foldout Header"),
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                    column_gap: Val::Px(6.0),
                    ..default()
                },
                BackgroundColor(header_bg),
                BorderRadius::all(Val::Px(6.0)),
                FoldoutHeader,
            ))
            .add_children(&[caret, title])
            .id();

        commands
            .entity(content)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.width = Val::Percent(100.0);
                node.min_width = Val::Px(0.0);
            });

        let content_wrapper = commands
            .spawn((
                Name::new("Foldout Content Wrapper"),
                Node {
                    width: Val::Percent(100.0),
                    min_width: Val::Px(0.0),
                    padding: UiRect::left(Val::Px(12.0)),
                    flex_direction: FlexDirection::Column,
                    display: if foldout.expanded {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    ..default()
                },
            ))
            .add_child(content)
            .id();

        let root = commands
            .spawn((
                Name::new("Foldout"),
                Node {
                    width: self.width,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                foldout.clone(),
                FoldoutState {
                    expanded: foldout.expanded,
                    content_wrapper,
                    caret,
                },
            ))
            .add_children(&[header, content_wrapper])
            .id();

        commands.entity(header).observe(on_foldout_header_click);
        root
    }

    pub fn build<B: Bundle>(
        self,
        commands: &mut Commands,
        content: B,
        theme: Option<&Theme>,
    ) -> Entity {
        let content_entity = commands.spawn(content).id();
        self.build_with_entity(commands, content_entity, theme)
    }
}

impl DefaultWidgetBuilder for FoldoutBuilder {
    fn spawn_default(commands: &mut Commands, theme: Option<&crate::theme::Theme>) -> Entity {
        FoldoutBuilder::new("Foldout").build(
            commands,
            (
                Node {
                    width: px(220.0),
                    height: px(80.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
            ),
            theme,
        )
    }
}

fn initialize_foldout_caret(
    mut commands: Commands,
    query: Query<&FoldoutState, Added<FoldoutState>>,
    mut icons_mgr: ResMut<IconsManager>,
    mut images: ResMut<Assets<Image>>,
) {
    for state in query.iter() {
        commands.entity(state.caret).insert((
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(2.)),
                width: Val::Px(16.),
                height: Val::Px(16.),
                ..default()
            },
            ImageNode::new(
                icons_mgr
                    .get_icon(&mut images, Icons::TriangleRight)
                    .unwrap(),
            ),
            UiTransform::from_rotation(if state.expanded {
                ROT_TO_DOWN
            } else {
                ROT_TO_RIGHT
            }),
        ));
    }
}

const ROT_TO_RIGHT: Rot2 = Rot2::IDENTITY;
const ROT_TO_DOWN: Rot2 = Rot2::FRAC_PI_2;

fn on_foldout_header_click(
    event: On<Pointer<Click>>,
    headers: Query<&ChildOf, With<FoldoutHeader>>,
    mut states: Query<&mut FoldoutState>,
    mut layout: Query<&mut Node>,
    mut images: Query<&mut UiTransform, With<ImageNode>>,
) {
    let Ok(child_of) = headers.get(event.event_target()) else {
        return;
    };
    let Ok(mut state) = states.get_mut(child_of.parent()) else {
        return;
    };

    state.expanded = !state.expanded;
    if let Ok(mut content_node) = layout.get_mut(state.content_wrapper) {
        content_node.display = if state.expanded {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut caret) = images.get_mut(state.caret) {
        caret.rotation = if state.expanded {
            ROT_TO_DOWN
        } else {
            ROT_TO_RIGHT
        };
    }
}
