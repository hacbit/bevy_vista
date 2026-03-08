use std::time::Duration;

use bevy::ecs::{query::With, system::Query};
use bevy::ui::UiTransform;

use super::*;

const ROT_TO_RIGHT: Rot2 = Rot2::IDENTITY;
const ROT_TO_DOWN: Rot2 = Rot2::FRAC_PI_2;

pub(super) fn toggle_vista_editor_expanded(
    expanded: Res<VistaEditorExpanded>,
    mode: Res<VistaEditorMode>,
    mut expanded_ui: Query<&mut Node, With<ContentRoot>>,
    mut query: Query<&mut UiTransform, With<TitleFoldButton>>,
) {
    let should_expand = matches!(*mode, VistaEditorMode::Fullscreen) || **expanded;
    if let Ok(mut node) = expanded_ui.single_mut() {
        node.display = if should_expand {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut transform) = query.single_mut() {
        transform.rotation = if should_expand {
            ROT_TO_DOWN
        } else {
            ROT_TO_RIGHT
        };
    }
}

pub(super) fn on_over(
    event: On<Pointer<Over>>,
    mut query: Query<&mut UiTransform, With<TitleFoldButton>>,
) {
    if let Ok(mut transform) = query.get_mut(event.event_target()) {
        transform.scale = Vec2::ONE * 1.2;
    }
}

pub(super) fn on_out(
    event: On<Pointer<Out>>,
    mut query: Query<&mut UiTransform, With<TitleFoldButton>>,
) {
    if let Ok(mut transform) = query.get_mut(event.event_target()) {
        transform.scale = Vec2::ONE;
    }
}

const CLICK_DURATION_THRESHOLD: Duration = Duration::from_millis(100);

pub(super) fn on_click(
    event: On<Pointer<Click>>,
    mut query: Query<&mut UiTransform, With<TitleFoldButton>>,
    mode: Res<VistaEditorMode>,
    mut expanded: ResMut<VistaEditorExpanded>,
) {
    if matches!(*mode, VistaEditorMode::Fullscreen) {
        return;
    }
    if event.duration > CLICK_DURATION_THRESHOLD {
        return;
    }
    if let Ok(mut transform) = query.get_mut(event.event_target()) {
        transform.rotation = if **expanded {
            ROT_TO_DOWN
        } else {
            ROT_TO_RIGHT
        };
        **expanded = !**expanded;
    }
}
