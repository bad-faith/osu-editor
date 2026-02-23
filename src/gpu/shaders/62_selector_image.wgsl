struct ScreenUniform {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> u_screen: ScreenUniform;

@group(1) @binding(0)
var image_tex: texture_2d<f32>;

@group(1) @binding(1)
var image_sampler: sampler;

struct VertexInput {
    @location(0) pos_px: vec2<f32>,
    @location(1) size_px: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput, @builtin(vertex_index) vid: u32) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );

    let corner = corners[vid];
    let p = in.pos_px + corner * in.size_px;

    let ndc_x = (p.x / u_screen.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (p.y / u_screen.screen_size.y) * 2.0;

    var out: VertexOutput;
    out.pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = corner;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dims_u = textureDimensions(image_tex);
    let dims = vec2<f32>(f32(dims_u.x), f32(dims_u.y));
    if dims.x <= 0.0 || dims.y <= 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let tex_aspect = dims.x / dims.y;
    var uv = in.uv;

    if tex_aspect > 1.0 {
        let used_h = 1.0 / tex_aspect;
        let pad_y = (1.0 - used_h) * 0.5;
        if uv.y < pad_y || uv.y > 1.0 - pad_y {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        uv.y = (uv.y - pad_y) / used_h;
    } else if tex_aspect < 1.0 {
        let used_w = tex_aspect;
        let pad_x = (1.0 - used_w) * 0.5;
        if uv.x < pad_x || uv.x > 1.0 - pad_x {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        uv.x = (uv.x - pad_x) / used_w;
    }

    return textureSample(image_tex, image_sampler, uv);
}
