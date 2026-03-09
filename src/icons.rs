use bevy::app::Plugin;
use bevy::asset::{Assets, Handle, RenderAssetUsages};
use bevy::ecs::resource::Resource;
use bevy::image::{CompressedImageFormats, Image, ImageSampler, ImageType};
use bevy::log::*;
use bevy_vista_macros::generate_icons;

/// Provides common editor icons
pub struct EditorIconsPlugin;

impl Plugin for EditorIconsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<IconsManager>().register_type::<Icons>();
    }
}

generate_icons! {
    /// This resource handle the registered editor icons
    ///
    /// # Examples
    /// ```rust
    /// use bevy::prelude::*;
    /// use bevy_vista::prelude::*;
    ///
    /// struct MyPlugin;
    ///
    /// impl Plugin for MyPlugin {
    ///     fn build(app: &mut App) {
    ///         app.add_systems(Startup, test_load_icon);
    ///     }
    /// }
    ///
    /// fn test_load_icon(
    ///     mut commands: Commands,
    ///     mut icons_mgr: ResMut<IconsManager>,
    ///     mut images: ResMut<Assets<Image>>,
    /// ) {
    ///     commands.spawn((
    ///         Node {
    ///             width: px(30.),
    ///             height: px(30.),
    ///             ..default()
    ///         },
    ///         ImageNode(icons_mgr.get_icon(&mut images, Icons::ArrowLeft).unwrap()),
    ///     ));
    /// }
    /// ```
    #[derive(Resource, Default)]
    #[icons_data("./src/icons_data.txt")]
    struct IconsManager

    #[derive(Hash, PartialEq, Eq, Clone, Copy)]
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
