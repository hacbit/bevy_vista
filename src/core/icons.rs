use bevy::asset::{Assets, Handle, RenderAssetUsages};
use bevy::image::{CompressedImageFormats, Image, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy_vista_macros::generate_icons;

/// Provides common editor icons
///
/// # Examples
/// ```rust
/// use bevy::prelude::*;
/// use bevy_vista::prelude::*;
///
/// struct MyPlugin;
///
/// impl Plugin for MyPlugin {
///     fn build(&self, app: &mut App) {
///         app.add_systems(Startup, test_load_icon);
///     }
/// }
///
/// fn test_load_icon(
///     mut commands: Commands,
/// ) {
///     commands.spawn((
///         Node {
///             width: px(30.),
///             height: px(30.),
///             ..default()
///         },
///         Icons::ArrowLeft,
///     ));
/// }
/// ```
pub struct EditorIconsPlugin;

impl Plugin for EditorIconsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<IconsManager>()
            .register_type::<Icons>()
            .add_systems(PostUpdate, sync_icon_components);
    }
}

generate_icons! {
    #[derive(Resource, Default)]
    #[icons_data("./src/core/icons_data.txt")]
    struct IconsManager

    #[derive(Component, Hash, PartialEq, Eq, Clone, Copy)]
    enum Icons
}

impl IconsManager {
    /// Get specified icon image handle.
    ///
    /// See [`IconsManager`] example.
    pub fn get_icon(&mut self, images: &mut Assets<Image>, icon: Icons) -> Option<Handle<Image>> {
        if let Some(handle) = self.handles.get(&icon) {
            Some(handle.clone())
        } else {
            let bytes = match decode_b64_image(icon.to_raw_data()) {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Failed to decode icon: {}", e);
                    return None;
                }
            };
            let image = match Image::from_buffer(
                &bytes,
                ImageType::MimeType("image/png"),
                CompressedImageFormats::all(),
                false,
                ImageSampler::Default,
                RenderAssetUsages::all(),
            ) {
                Ok(image) => image,
                Err(e) => {
                    error!("Failed to create image from bytes: {}", e);
                    return None;
                }
            };
            let handle = images.add(image);
            self.handles.insert(icon, handle.clone());
            Some(handle)
        }
    }
}

fn sync_icon_components(
    mut commands: Commands,
    mut icons_mgr: ResMut<IconsManager>,
    mut images: ResMut<Assets<Image>>,
    mut with_image: Query<(&Icons, &mut ImageNode), Or<(Added<Icons>, Changed<Icons>)>>,
    without_image: Query<
        (Entity, &Icons),
        (Or<(Added<Icons>, Changed<Icons>)>, Without<ImageNode>),
    >,
) {
    for (icon, mut image_node) in &mut with_image {
        let Some(handle) = icons_mgr.get_icon(&mut images, *icon) else {
            continue;
        };
        image_node.image = handle;
    }

    for (entity, icon) in &without_image {
        let Some(handle) = icons_mgr.get_icon(&mut images, *icon) else {
            continue;
        };
        commands.entity(entity).insert(ImageNode::new(handle));
    }
}

fn decode_b64_image(image_str: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    STANDARD
        .decode(image_str)
        .map_err(|e| format!("base64 decode error: {}", e))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bevy::reflect::Typed;

    use super::*;

    #[test]
    fn reflected_variants_cover_all_icons() {
        let reflected = Icons::reflected_variants();

        let bevy::reflect::TypeInfo::Enum(enum_info) = Icons::type_info() else {
            panic!("Icons should be a reflected enum");
        };

        assert_eq!(reflected.len(), enum_info.variant_len());
        assert!(reflected.contains(&("ArrowLeft", Icons::ArrowLeft)));

        let unique_names = reflected
            .iter()
            .map(|(name, _)| *name)
            .collect::<HashSet<_>>();
        assert_eq!(unique_names.len(), reflected.len());
    }
}
