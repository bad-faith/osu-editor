struct ScreenUniform {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> screen: ScreenUniform;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) ch: u32,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) size: vec2<f32>,
    @location(3) @interpolate(flat) ch: u32,
};

@vertex
fn vs_main(input: VsIn, @builtin(vertex_index) vid: u32) -> VsOut {
    var unit = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );

    let local = unit[vid] * input.size;
    let px = input.pos + local;
    let ndc = vec2<f32>(
        (px.x / screen.screen_size.x) * 2.0 - 1.0,
        1.0 - (px.y / screen.screen_size.y) * 2.0
    );

    var out: VsOut;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.local = local;
    out.color = input.color;
    out.size = input.size;
    out.ch = input.ch;
    return out;
}

fn glyph5x7_row_bits(ch: u32, row: u32) -> u32 {
    if (row >= 7u) {
        return 0u;
    }

    var c = ch;
    if (c >= 97u && c <= 122u) {
        c = c - 32u;
    }

    if (c == 32u) { return 0u; }
    if (c == 33u) { let r = array<u32, 7>(4u, 4u, 4u, 4u, 4u, 0u, 4u); return r[row]; }
    if (c == 34u) { let r = array<u32, 7>(10u, 10u, 10u, 0u, 0u, 0u, 0u); return r[row]; }
    if (c == 35u) { let r = array<u32, 7>(10u, 10u, 31u, 10u, 31u, 10u, 10u); return r[row]; }
    if (c == 36u) { let r = array<u32, 7>(4u, 15u, 20u, 14u, 5u, 30u, 4u); return r[row]; }
    if (c == 37u) { let r = array<u32, 7>(17u, 18u, 4u, 8u, 19u, 17u, 0u); return r[row]; }
    if (c == 38u) { let r = array<u32, 7>(12u, 18u, 20u, 8u, 21u, 18u, 13u); return r[row]; }
    if (c == 39u) { let r = array<u32, 7>(4u, 4u, 4u, 0u, 0u, 0u, 0u); return r[row]; }
    if (c == 40u) { let r = array<u32, 7>(2u, 4u, 8u, 8u, 8u, 4u, 2u); return r[row]; }
    if (c == 41u) { let r = array<u32, 7>(8u, 4u, 2u, 2u, 2u, 4u, 8u); return r[row]; }
    if (c == 42u) { let r = array<u32, 7>(0u, 4u, 21u, 14u, 21u, 4u, 0u); return r[row]; }
    if (c == 43u) { let r = array<u32, 7>(0u, 4u, 4u, 31u, 4u, 4u, 0u); return r[row]; }
    if (c == 44u) { let r = array<u32, 7>(0u, 0u, 0u, 0u, 0u, 4u, 8u); return r[row]; }
    if (c == 48u) { let r = array<u32, 7>(14u, 17u, 19u, 21u, 25u, 17u, 14u); return r[row]; }
    if (c == 49u) { let r = array<u32, 7>(4u, 12u, 4u, 4u, 4u, 4u, 14u); return r[row]; }
    if (c == 50u) { let r = array<u32, 7>(14u, 17u, 1u, 2u, 4u, 8u, 31u); return r[row]; }
    if (c == 51u) { let r = array<u32, 7>(30u, 1u, 1u, 14u, 1u, 1u, 30u); return r[row]; }
    if (c == 52u) { let r = array<u32, 7>(2u, 6u, 10u, 18u, 31u, 2u, 2u); return r[row]; }
    if (c == 53u) { let r = array<u32, 7>(31u, 16u, 16u, 30u, 1u, 1u, 30u); return r[row]; }
    if (c == 54u) { let r = array<u32, 7>(14u, 16u, 16u, 30u, 17u, 17u, 14u); return r[row]; }
    if (c == 55u) { let r = array<u32, 7>(31u, 1u, 2u, 4u, 8u, 8u, 8u); return r[row]; }
    if (c == 56u) { let r = array<u32, 7>(14u, 17u, 17u, 14u, 17u, 17u, 14u); return r[row]; }
    if (c == 57u) { let r = array<u32, 7>(14u, 17u, 17u, 15u, 1u, 1u, 14u); return r[row]; }
    if (c == 45u) { let r = array<u32, 7>(0u, 0u, 0u, 14u, 0u, 0u, 0u); return r[row]; }
    if (c == 46u) { let r = array<u32, 7>(0u, 0u, 0u, 0u, 0u, 4u, 4u); return r[row]; }
    if (c == 47u) { let r = array<u32, 7>(1u, 2u, 4u, 8u, 16u, 0u, 0u); return r[row]; }
    if (c == 63u) { let r = array<u32, 7>(14u, 17u, 1u, 2u, 4u, 0u, 4u); return r[row]; }
    if (c == 58u) { let r = array<u32, 7>(0u, 4u, 4u, 0u, 4u, 4u, 0u); return r[row]; }
    if (c == 59u) { let r = array<u32, 7>(0u, 4u, 4u, 0u, 4u, 4u, 8u); return r[row]; }
    if (c == 60u) { let r = array<u32, 7>(2u, 4u, 8u, 16u, 8u, 4u, 2u); return r[row]; }
    if (c == 61u) { let r = array<u32, 7>(0u, 0u, 31u, 0u, 31u, 0u, 0u); return r[row]; }
    if (c == 62u) { let r = array<u32, 7>(8u, 4u, 2u, 1u, 2u, 4u, 8u); return r[row]; }
    if (c == 64u) { let r = array<u32, 7>(14u, 17u, 23u, 21u, 23u, 16u, 14u); return r[row]; }
    if (c == 91u) { let r = array<u32, 7>(14u, 8u, 8u, 8u, 8u, 8u, 14u); return r[row]; }
    if (c == 92u) { let r = array<u32, 7>(16u, 8u, 4u, 2u, 1u, 0u, 0u); return r[row]; }
    if (c == 93u) { let r = array<u32, 7>(14u, 2u, 2u, 2u, 2u, 2u, 14u); return r[row]; }
    if (c == 94u) { let r = array<u32, 7>(4u, 10u, 17u, 0u, 0u, 0u, 0u); return r[row]; }
    if (c == 95u) { let r = array<u32, 7>(0u, 0u, 0u, 0u, 0u, 0u, 31u); return r[row]; }
    if (c == 96u) { let r = array<u32, 7>(8u, 4u, 2u, 0u, 0u, 0u, 0u); return r[row]; }
    if (c == 123u) { let r = array<u32, 7>(2u, 4u, 4u, 8u, 4u, 4u, 2u); return r[row]; }
    if (c == 124u) { let r = array<u32, 7>(4u, 4u, 4u, 4u, 4u, 4u, 4u); return r[row]; }
    if (c == 125u) { let r = array<u32, 7>(8u, 4u, 4u, 2u, 4u, 4u, 8u); return r[row]; }
    if (c == 126u) { let r = array<u32, 7>(0u, 0u, 13u, 18u, 0u, 0u, 0u); return r[row]; }
    if (c == 65u) { let r = array<u32, 7>(14u, 17u, 17u, 31u, 17u, 17u, 17u); return r[row]; }
    if (c == 66u) { let r = array<u32, 7>(30u, 17u, 17u, 30u, 17u, 17u, 30u); return r[row]; }
    if (c == 67u) { let r = array<u32, 7>(14u, 17u, 16u, 16u, 16u, 17u, 14u); return r[row]; }
    if (c == 68u) { let r = array<u32, 7>(30u, 17u, 17u, 17u, 17u, 17u, 30u); return r[row]; }
    if (c == 69u) { let r = array<u32, 7>(31u, 16u, 16u, 30u, 16u, 16u, 31u); return r[row]; }
    if (c == 70u) { let r = array<u32, 7>(31u, 16u, 16u, 30u, 16u, 16u, 16u); return r[row]; }
    if (c == 71u) { let r = array<u32, 7>(14u, 17u, 16u, 19u, 17u, 17u, 14u); return r[row]; }
    if (c == 72u) { let r = array<u32, 7>(17u, 17u, 17u, 31u, 17u, 17u, 17u); return r[row]; }
    if (c == 73u) { let r = array<u32, 7>(31u, 4u, 4u, 4u, 4u, 4u, 31u); return r[row]; }
    if (c == 74u) { let r = array<u32, 7>(7u, 2u, 2u, 2u, 2u, 18u, 12u); return r[row]; }
    if (c == 75u) { let r = array<u32, 7>(17u, 18u, 20u, 24u, 20u, 18u, 17u); return r[row]; }
    if (c == 76u) { let r = array<u32, 7>(16u, 16u, 16u, 16u, 16u, 16u, 31u); return r[row]; }
    if (c == 77u) { let r = array<u32, 7>(17u, 27u, 21u, 21u, 17u, 17u, 17u); return r[row]; }
    if (c == 78u) { let r = array<u32, 7>(17u, 25u, 21u, 19u, 17u, 17u, 17u); return r[row]; }
    if (c == 79u) { let r = array<u32, 7>(14u, 17u, 17u, 17u, 17u, 17u, 14u); return r[row]; }
    if (c == 80u) { let r = array<u32, 7>(30u, 17u, 17u, 30u, 16u, 16u, 16u); return r[row]; }
    if (c == 81u) { let r = array<u32, 7>(14u, 17u, 17u, 17u, 21u, 18u, 13u); return r[row]; }
    if (c == 82u) { let r = array<u32, 7>(30u, 17u, 17u, 30u, 20u, 18u, 17u); return r[row]; }
    if (c == 83u) { let r = array<u32, 7>(15u, 16u, 16u, 14u, 1u, 1u, 30u); return r[row]; }
    if (c == 84u) { let r = array<u32, 7>(31u, 4u, 4u, 4u, 4u, 4u, 4u); return r[row]; }
    if (c == 85u) { let r = array<u32, 7>(17u, 17u, 17u, 17u, 17u, 17u, 14u); return r[row]; }
    if (c == 86u) { let r = array<u32, 7>(17u, 17u, 17u, 17u, 17u, 10u, 4u); return r[row]; }
    if (c == 87u) { let r = array<u32, 7>(17u, 17u, 17u, 21u, 21u, 21u, 10u); return r[row]; }
    if (c == 88u) { let r = array<u32, 7>(17u, 10u, 4u, 4u, 4u, 10u, 17u); return r[row]; }
    if (c == 89u) { let r = array<u32, 7>(17u, 10u, 4u, 4u, 4u, 4u, 4u); return r[row]; }
    if (c == 90u) { let r = array<u32, 7>(31u, 1u, 2u, 4u, 8u, 16u, 31u); return r[row]; }

    return 0u;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let scale = max(1.0, input.size.y / 7.0);
    let col = u32(floor(input.local.x / scale));
    let row = u32(floor(input.local.y / scale));

    if (col >= 5u || row >= 7u) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let bits = glyph5x7_row_bits(input.ch, row);
    let bit = (bits >> (4u - col)) & 1u;
    let alpha = select(0.0, input.color.a, bit == 1u);
    return vec4<f32>(input.color.rgb, alpha);
}
