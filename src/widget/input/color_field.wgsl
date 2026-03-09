#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> params0: vec4<f32>;
@group(1) @binding(1) var<uniform> params1: vec4<f32>;

fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = hsv.x;
    let s = hsv.y;
    let v = hsv.z;
    let k = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(vec3<f32>(h) + k.xyz) * 6.0 - vec3<f32>(k.www));
    return v * mix(vec3<f32>(1.0), clamp(p - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0)), s);
}

fn checker(uv: vec2<f32>, size: vec2<f32>) -> f32 {
    let scaled = floor(uv * size / 8.0);
    return select(0.0, 1.0, i32(scaled.x + scaled.y) % 2 == 0);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let kind = params0.x;
    let uv = clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0));

    if (kind < 0.5) {
        let rgb = hsv_to_rgb(vec3<f32>(params0.y, uv.x, 1.0 - uv.y));
        return vec4<f32>(rgb, 1.0);
    }

    if (kind < 1.5) {
        let rgb = hsv_to_rgb(vec3<f32>(1.0 - uv.y, 1.0, 1.0));
        return vec4<f32>(rgb, 1.0);
    }

    let base = vec3<f32>(params1.y, params1.z, params1.w);
    let alpha = uv.x;
    let bg_a = vec3<f32>(0.18, 0.18, 0.18);
    let bg_b = vec3<f32>(0.30, 0.30, 0.30);
    let bg = mix(bg_a, bg_b, checker(uv, vec2<f32>(in.size)));
    return vec4<f32>(mix(bg, base, alpha), 1.0);
}
