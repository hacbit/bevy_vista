use bevy::ecs::prelude::*;
use bevy::picking::events::{Drag, Pointer};
use bevy::ui::prelude::*;

use super::*;

pub(super) fn on_drag(
    event: On<Pointer<Drag>>,
    mut query: Query<(&mut UiTransform, &ComputedNode), With<EditorRoot>>,
    window: Single<&Window>,
    mode: Res<VistaEditorMode>,
) {
    if matches!(*mode, VistaEditorMode::Fullscreen) {
        return;
    }
    if let Ok((mut transform, computed_node)) = query.single_mut() {
        let editor_size = computed_node.size / window.scale_factor();
        let delta = event.delta;
        let (left_top, right_bottom) = (Vec2::ZERO, window.size());
        if let Val2 {
            x: Val::Px(left),
            y: Val::Px(top),
        } = transform.translation
        {
            let new_left = (left + delta.x).clamp(left_top.x, right_bottom.x - editor_size.x);
            let new_top = (top + delta.y).clamp(left_top.y, right_bottom.y - editor_size.y);
            transform.translation = Val2 {
                x: Val::Px(new_left),
                y: Val::Px(new_top),
            };
        }
    }
}
