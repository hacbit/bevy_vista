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
