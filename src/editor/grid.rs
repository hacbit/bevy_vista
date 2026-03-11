use bevy::app::App;
use bevy::asset::Asset;
use bevy::asset::{Handle, load_internal_asset, uuid_handle};
use bevy::math::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::{Shader, ShaderRef};
use bevy::ui_render::prelude::UiMaterial;

pub fn load_grid_shader(app: &mut App) {
    load_internal_asset!(app, GRID_SHADER_HANDLE, "grid.wgsl", Shader::from_wgsl);
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct GridUiMaterial {
    #[uniform(0)]
    pub grid_color_primary: Vec4,
    #[uniform(1)]
    pub grid_color_secondary: Vec4,
    /// x => cell_size
    ///
    /// y => line_width (0 - 1)
    ///
    /// z => scale_factor
    ///
    /// w => primary_interval
    #[uniform(2)]
    pub grid_params: Vec4,
    /// only use xy
    #[uniform(3)]
    pub offset: Vec2,
    #[uniform(4)]
    pub anti_alias: f32,
}

impl Default for GridUiMaterial {
    fn default() -> Self {
        Self {
            grid_color_primary: Vec4::new(0.50, 0.50, 0.54, 0.42),
            grid_color_secondary: Vec4::new(0.28, 0.28, 0.32, 0.22),
            grid_params: Vec4::new(20.0, 1.0, 1.0, 5.0),
            offset: Vec2::ZERO,
            anti_alias: 1.0,
        }
    }
}

const GRID_SHADER_HANDLE: Handle<Shader> = uuid_handle!("557a20ca-ecdd-99e7-4a51-8d5b59b4b8f0");

impl UiMaterial for GridUiMaterial {
    fn fragment_shader() -> ShaderRef {
        GRID_SHADER_HANDLE.into()
    }
}
