#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> grid_color_primary: vec4<f32>;
@group(1) @binding(1) var<uniform> grid_color_secondary: vec4<f32>;
// x => cell size
// y => line width
// z => scale factor
// w => primary interval
@group(1) @binding(2) var<uniform> grid_params: vec4<f32>;
// x, y
@group(1) @binding(3) var<uniform> offset: vec2<f32>;
@group(1) @binding(4) var<uniform> anti_alias: f32;

fn smooth_line(distance: f32, width: f32, aa_width: f32) -> f32 {
    let d = abs(distance);
    return 1.0 - smoothstep(width - aa_width, width + aa_width, d);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let cell_size = grid_params.x;
    let line_width = grid_params.y;
    let scale = max(grid_params.z, 0.0001);
    let primary_interval = grid_params.w;
    let aa_width = anti_alias;

    // Use viewport-local pixels, remapped into canvas-local coordinates.
    let local_pos = (in.uv * vec2(in.size) + offset.xy) / scale;
    let minor_pos = local_pos / cell_size;
    let major_pos = local_pos / (cell_size * primary_interval);

    let minor_x = min(fract(minor_pos.x), 1.0 - fract(minor_pos.x)) * cell_size;
    let minor_y = min(fract(minor_pos.y), 1.0 - fract(minor_pos.y)) * cell_size;
    let major_x = min(fract(major_pos.x), 1.0 - fract(major_pos.x)) * cell_size * primary_interval;
    let major_y = min(fract(major_pos.y), 1.0 - fract(major_pos.y)) * cell_size * primary_interval;

    let minor_line = max(
        smooth_line(minor_x, line_width * 0.5, aa_width),
        smooth_line(minor_y, line_width * 0.5, aa_width),
    );
    let major_line = max(
        smooth_line(major_x, line_width * 0.75, aa_width),
        smooth_line(major_y, line_width * 0.75, aa_width),
    );

    let color = mix(grid_color_secondary, grid_color_primary, major_line);
    let alpha = max(minor_line * grid_color_secondary.a, major_line * grid_color_primary.a);
    return vec4(color.rgb, alpha);
}

// struct GridParams {
//     cell_size: vec2<f32>,
//     grid_size: vec2<u32>,
//     line_color: vec4<f32>,
//     major_line_color: vec4<f32>,
//     background_color: vec4<f32>,
//     center: vec2<f32>,
//     major_line_frequency: u32,
//     fade_edges: u32,
//     fade_distance: f32,
// }

// @group(1) @binding(0)
// var<uniform> grid_params: GridParams;

// struct VertexOutput {
//     @builtin(position) position: vec4<f32>,
//     @location(0) world_pos: vec2<f32>,
// }

// @vertex
// fn vertex(
//     @builtin(vertex_index) vertex_index: u32,
//     @builtin(instance_index) instance_index: u32,
// ) -> VertexOutput {
//     var output: VertexOutput;

//     let total_size = vec2<f32>(
//         f32(grid_params.grid_size.x) * grid_params.cell_size.x,
//         f32(grid_params.grid_size.y) * grid_params.cell_size.y,
//     );

//     let half_size = total_size * 0.5;
//     let min = grid_params.center - half_size;
//     let max = grid_params.center + half_size;

//     let vertex_uv = vec2<f32>(
//         f32(vertex_index % 2u),
//         f32((vertex_index / 2u) % 2u),
//     );

//     output.position = vec4<f32>(vertex_uv * 2.0 - 1.0, 0.0, 1.0);
//     output.world_pos = mix(min, max, vertex_uv);

//     return output;
// }

// @fragment
// fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
//     let total_size = vec2<f32>(
//         f32(grid_params.grid_size.x) * grid_params.cell_size.x,
//         f32(grid_params.grid_size.y) * grid_params.cell_size.y,
//     );

//     let half_size = total_size * 0.5;
//     let min_pos = grid_params.center - half_size;
//     let max_pos = grid_params.center + half_size;

//     var color = grid_params.background_color;

//     let grid_pos = (input.world_pos - min_pos) / grid_params.cell_size;
//     let grid_frac = fract(grid_pos);
//     let grid_int = floor(grid_pos);

//     // thin lines
//     let line_dist = min(
//         min(grid_frac.x, 1.0 - grid_frac.x),
//         min(grid_frac.y, 1.0 - grid_frac.y),
//     ) * min(grid_params.cell_size.x, grid_params.cell_size.y);

//     let line_width = 1.0;

//     if (line_dist < line_width) {
//         color = grid_params.line_color;
//     }

//     // bold lines
//     let major_grid_pos = (input.world_pos - min_pos) / (grid_params.cell_size * f32(grid_params.major_line_frequency));
//     let major_grid_frac = fract(major_grid_pos);

//     let line_dist_major = min(
//         min(major_grid_frac.x, 1.0 - major_grid_frac.x),
//         min(major_grid_frac.y, 1.0 - major_grid_frac.y),
//     ) * min(grid_params.cell_size.x, grid_params.cell_size.y) * f32(grid_params.major_line_frequency);

//     let major_line_width = 1.0;

//     if (line_dist_major < major_line_width) {
//         color = grid_params.major_line_color;
//     }

//     var alpha_multiplier = 1.0;
//     if (grid_params.fade_edges == 1u) {
//         let normalized_x = (input.world_pos.x - min_pos.x) / total_size.x;
//         let normalized_y = (input.world_pos.y - min_pos.y) / total_size.y;

//         let edge_distance_x = abs(normalized_x - 0.5) * 2.0;
//         let edge_distance_y = abs(normalized_y - 0.5) * 2.0;
//         let edge_distance = max(edge_distance_x, edge_distance_y);

//         if (edge_distance > grid_params.fade_distance) {
//             alpha_multiplier = 1.0 - ((edge_distance - grid_params.fade_distance) / 
//                 (1.0 - grid_params.fade_distance));
//         }
//     }

//     color.a *= alpha_multiplier;

//     return color;
// }
