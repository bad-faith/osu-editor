fn over_pm(dst_pm: vec3<f32>, dst_a: f32, src: vec4<f32>) -> vec4<f32> {
    let src_pm = src.rgb * src.a;
    let out_pm = src_pm + dst_pm * (1.0 - src.a);
    let out_a = src.a + dst_a * (1.0 - src.a);
    return vec4<f32>(out_pm, out_a);
}

fn digit_aspect(di: u32) -> f32 {
    let xf = digits_meta.uv_xform[di];
    let w_px = max(1e-6, xf.x * digits_meta.max_size_px.x);
    let h_px = max(1e-6, xf.y * digits_meta.max_size_px.y);
    return w_px / h_px;
}

fn sample_digit_at_px(px: vec2<f32>, tl: vec2<f32>, h: f32, di: u32) -> vec4<f32> {
    let w = h * digit_aspect(di);
    let local = (px - tl) / vec2<f32>(w, h);
    if (all(local >= vec2<f32>(0.0)) && all(local <= vec2<f32>(1.0))) {
        let xf = digits_meta.uv_xform[di];
        let uv2 = local * xf.xy + xf.zw;
        // digits are sampled from their own layer
        return textureSample(numbers_tex, skin_samp, uv2, i32(di));
    }
    return vec4<f32>(0.0);
}

fn rect_alpha(px: vec2<f32>, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    if (px.x >= x0 && px.x <= x1 && px.y >= y0 && px.y <= y1) {
        return 1.0;
    }
    return 0.0;
}

fn colon_alpha(px: vec2<f32>, x: f32, y: f32, h: f32) -> f32 {
    let dot = max(2.0, h * 0.12);
    let gap = h * 0.22;
    let cx = x + dot * 0.5;
    let y0 = y + h * 0.35;
    let y1 = y + h * 0.65;
    let a0 = rect_alpha(px, cx - dot * 0.5, y0 - dot * 0.5, cx + dot * 0.5, y0 + dot * 0.5);
    let a1 = rect_alpha(px, cx - dot * 0.5, y1 - dot * 0.5, cx + dot * 0.5, y1 + dot * 0.5);
    return max(a0, a1);
}

fn dot_alpha(px: vec2<f32>, x: f32, y: f32, h: f32) -> f32 {
    let dot = max(2.0, h * 0.12);
    let cx = x + dot * 0.5;
    let cy = y + h * 0.65;
    return rect_alpha(px, cx - dot * 0.5, cy - dot * 0.5, cx + dot * 0.5, cy + dot * 0.5);
}

fn glyph5x7_row_bits(ch: u32, row: u32) -> u32 {
    if (row >= 7u) {
        return 0u;
    }

    var c = ch;
    if (c >= 97u && c <= 122u) {
        c = c - 32u;
    }

    if (c == 32u) { return 0u; } // ' '

    // Digits
    if (c == 48u) { let r = array<u32, 7>(14u, 17u, 19u, 21u, 25u, 17u, 14u); return r[row]; } // 0
    if (c == 49u) { let r = array<u32, 7>(4u, 12u, 4u, 4u, 4u, 4u, 14u); return r[row]; } // 1
    if (c == 50u) { let r = array<u32, 7>(14u, 17u, 1u, 2u, 4u, 8u, 31u); return r[row]; } // 2
    if (c == 51u) { let r = array<u32, 7>(30u, 1u, 1u, 14u, 1u, 1u, 30u); return r[row]; } // 3
    if (c == 52u) { let r = array<u32, 7>(2u, 6u, 10u, 18u, 31u, 2u, 2u); return r[row]; } // 4
    if (c == 53u) { let r = array<u32, 7>(31u, 16u, 16u, 30u, 1u, 1u, 30u); return r[row]; } // 5
    if (c == 54u) { let r = array<u32, 7>(14u, 16u, 16u, 30u, 17u, 17u, 14u); return r[row]; } // 6
    if (c == 55u) { let r = array<u32, 7>(31u, 1u, 2u, 4u, 8u, 8u, 8u); return r[row]; } // 7
    if (c == 56u) { let r = array<u32, 7>(14u, 17u, 17u, 14u, 17u, 17u, 14u); return r[row]; } // 8
    if (c == 57u) { let r = array<u32, 7>(14u, 17u, 17u, 15u, 1u, 1u, 14u); return r[row]; } // 9

    // Punctuation
    if (c == 45u) { let r = array<u32, 7>(0u, 0u, 0u, 14u, 0u, 0u, 0u); return r[row]; } // -
    if (c == 46u) { let r = array<u32, 7>(0u, 0u, 0u, 0u, 0u, 4u, 4u); return r[row]; } // .
    if (c == 58u) { let r = array<u32, 7>(0u, 4u, 4u, 0u, 4u, 4u, 0u); return r[row]; } // :
    if (c == 95u) { let r = array<u32, 7>(0u, 0u, 0u, 0u, 0u, 0u, 31u); return r[row]; } // _
    if (c == 124u) { let r = array<u32, 7>(4u, 4u, 4u, 4u, 4u, 4u, 4u); return r[row]; } // |

    // Uppercase letters used by the stats box.
    if (c == 65u) { let r = array<u32, 7>(14u, 17u, 17u, 31u, 17u, 17u, 17u); return r[row]; } // A
    if (c == 66u) { let r = array<u32, 7>(30u, 17u, 17u, 30u, 17u, 17u, 30u); return r[row]; } // B
    if (c == 67u) { let r = array<u32, 7>(14u, 17u, 16u, 16u, 16u, 17u, 14u); return r[row]; } // C
    if (c == 68u) { let r = array<u32, 7>(30u, 17u, 17u, 17u, 17u, 17u, 30u); return r[row]; } // D
    if (c == 69u) { let r = array<u32, 7>(31u, 16u, 16u, 30u, 16u, 16u, 31u); return r[row]; } // E
    if (c == 70u) { let r = array<u32, 7>(31u, 16u, 16u, 30u, 16u, 16u, 16u); return r[row]; } // F
    if (c == 71u) { let r = array<u32, 7>(14u, 17u, 16u, 19u, 17u, 17u, 14u); return r[row]; } // G
    if (c == 72u) { let r = array<u32, 7>(17u, 17u, 17u, 31u, 17u, 17u, 17u); return r[row]; } // H
    if (c == 73u) { let r = array<u32, 7>(31u, 4u, 4u, 4u, 4u, 4u, 31u); return r[row]; } // I
    if (c == 74u) { let r = array<u32, 7>(7u, 2u, 2u, 2u, 2u, 18u, 12u); return r[row]; } // J
    if (c == 75u) { let r = array<u32, 7>(17u, 18u, 20u, 24u, 20u, 18u, 17u); return r[row]; } // K
    if (c == 76u) { let r = array<u32, 7>(16u, 16u, 16u, 16u, 16u, 16u, 31u); return r[row]; } // L
    if (c == 77u) { let r = array<u32, 7>(17u, 27u, 21u, 21u, 17u, 17u, 17u); return r[row]; } // M
    if (c == 78u) { let r = array<u32, 7>(17u, 25u, 21u, 19u, 17u, 17u, 17u); return r[row]; } // N
    if (c == 79u) { let r = array<u32, 7>(14u, 17u, 17u, 17u, 17u, 17u, 14u); return r[row]; } // O
    if (c == 80u) { let r = array<u32, 7>(30u, 17u, 17u, 30u, 16u, 16u, 16u); return r[row]; } // P
    if (c == 81u) { let r = array<u32, 7>(14u, 17u, 17u, 17u, 21u, 18u, 13u); return r[row]; } // Q
    if (c == 82u) { let r = array<u32, 7>(30u, 17u, 17u, 30u, 20u, 18u, 17u); return r[row]; } // R
    if (c == 83u) { let r = array<u32, 7>(15u, 16u, 16u, 14u, 1u, 1u, 30u); return r[row]; } // S
    if (c == 84u) { let r = array<u32, 7>(31u, 4u, 4u, 4u, 4u, 4u, 4u); return r[row]; } // T
    if (c == 85u) { let r = array<u32, 7>(17u, 17u, 17u, 17u, 17u, 17u, 14u); return r[row]; } // U
    if (c == 86u) { let r = array<u32, 7>(17u, 17u, 17u, 17u, 17u, 10u, 4u); return r[row]; } // V
    if (c == 87u) { let r = array<u32, 7>(17u, 17u, 17u, 21u, 21u, 21u, 10u); return r[row]; } // W
    if (c == 88u) { let r = array<u32, 7>(17u, 10u, 4u, 4u, 4u, 10u, 17u); return r[row]; } // X
    if (c == 89u) { let r = array<u32, 7>(17u, 10u, 4u, 4u, 4u, 4u, 4u); return r[row]; } // Y
    if (c == 90u) { let r = array<u32, 7>(31u, 1u, 2u, 4u, 8u, 16u, 31u); return r[row]; } // Z

    return 0u;
}

fn glyph5x7_alpha(px: vec2<f32>, tl: vec2<f32>, h: f32, ch: u32) -> f32 {
    if (ch == 32u) {
        return 0.0;
    }

    let scale = max(1.0, h / 7.0);
    let w = 5.0 * scale;
    let local = px - tl;
    if (!(local.x >= 0.0 && local.y >= 0.0 && local.x < w && local.y < 7.0 * scale)) {
        return 0.0;
    }

    let col = u32(floor(local.x / scale));
    let row = u32(floor(local.y / scale));
    if (col >= 5u || row >= 7u) {
        return 0.0;
    }

    let bits = glyph5x7_row_bits(ch, row);
    let bit = (bits >> (4u - col)) & 1u;
    return select(0.0, 1.0, bit == 1u);
}

fn tri_alpha(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> f32 {
    // Barycentric inside-triangle test.
    let v0 = c - a;
    let v1 = b - a;
    let v2 = p - a;
    let dot00 = dot(v0, v0);
    let dot01 = dot(v0, v1);
    let dot02 = dot(v0, v2);
    let dot11 = dot(v1, v1);
    let dot12 = dot(v1, v2);

    let denom = dot00 * dot11 - dot01 * dot01;
    if (abs(denom) <= 1e-6) {
        return 0.0;
    }
    let inv = 1.0 / denom;
    let u = (dot11 * dot02 - dot01 * dot12) * inv;
    let v = (dot00 * dot12 - dot01 * dot02) * inv;

    if (u >= 0.0 && v >= 0.0 && (u + v) <= 1.0) {
        return 1.0;
    }
    return 0.0;
}

fn play_icon_alpha(px: vec2<f32>, x: f32, y: f32, w: f32, h: f32) -> f32 {
    let p = px;
    if !(p.x >= x && p.x <= x + w && p.y >= y && p.y <= y + h) {
        return 0.0;
    }

    // Right-pointing triangle with a small inset.
    let inset = max(2.0, min(w, h) * 0.12);
    let a = vec2<f32>(x + inset, y + inset);
    let b = vec2<f32>(x + inset, y + h - inset);
    let c = vec2<f32>(x + w - inset, y + h * 0.5);
    return tri_alpha(p, a, b, c);
}

fn pause_icon_alpha(px: vec2<f32>, x: f32, y: f32, w: f32, h: f32) -> f32 {
    if !(px.x >= x && px.x <= x + w && px.y >= y && px.y <= y + h) {
        return 0.0;
    }
    let inset = max(2.0, min(w, h) * 0.12);
    let gap = max(2.0, w * 0.12);
    let bar_w = (w - 2.0 * inset - gap) * 0.5;
    let x0 = x + inset;
    let x1 = x0 + bar_w;
    let x2 = x1 + gap;
    let x3 = x2 + bar_w;
    let y0 = y + inset;
    let y1 = y + h - inset;
    let a0 = rect_alpha(px, x0, y0, x1, y1);
    let a1 = rect_alpha(px, x2, y0, x3, y1);
    return max(a0, a1);
}

fn rotate2(p: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

fn angle_norm(a: f32) -> f32 {
    var t = a;
    if (t < 0.0) {
        t = t + 6.283185307;
    }
    return t;
}

fn arc_sweep_t(ang: f32, start: f32, sweep: f32) -> f32 {
    let a = angle_norm(ang - start);
    if (a > sweep) {
        return -1.0;
    }
    return a / max(sweep, 1e-6);
}

fn ring_alpha(p: vec2<f32>, r: f32, thickness: f32, start_ang: f32, end_ang: f32) -> f32 {
    let dist = length(p);
    let half = thickness * 0.5;
    if (dist < r - half || dist > r + half) {
        return 0.0;
    }
    let ang = angle_norm(atan2(p.y, p.x));
    let s = angle_norm(start_ang);
    let e = angle_norm(end_ang);
    let in_sweep = select(ang >= s && ang <= e, ang >= s || ang <= e, s <= e);
    return select(0.0, 1.0, in_sweep);
}

fn ring_arrow_alpha_local(p: vec2<f32>, size: f32) -> f32 {
    let r = size * 0.5;
    let thickness = size * 0.18;
    let gap = 1.2;
    let sweep = 6.283185307 - gap;
    let start = gap * 0.5;
    let end = start + sweep;

    let dist = length(p);
    let half = thickness * 0.5;
    let radial = 1.0 - smoothstep(half - 1.0, half + 1.0, abs(dist - r));
    var ring = 0.0;
    if (dist >= r - half && dist <= r + half) {
        let ang = atan2(p.y, p.x);
        let t = arc_sweep_t(ang, start, sweep);
        if (t >= 0.0) {
            let fade = mix(0.25, 1.0, t);
            ring = radial * select(fade, 1.0, t > 0.85);
        }
    }

    let dir = vec2<f32>(cos(end), sin(end));
    let tangent = vec2<f32>(-dir.y, dir.x);
    let normal = dir;
    let tip = (-tangent) * (r + thickness * 1.1);
    let base = (-tangent) * (r + thickness * 0.2);
    let head_w = thickness;
    var a = tip;
    var b = base + normal * (head_w * 0.5);
    var c = base - normal * (head_w * 0.5);
    let center = (a + b + c) / 3.0;
    let rot = -1.5707963;
    a = rotate2(a - center, rot) + center;
    b = rotate2(b - center, rot) + center;
    c = rotate2(c - center, rot) + center;
    let head = tri_alpha(p, a, b, c);

    return max(ring, head);
}

fn cover_uv(screen_uv: vec2<f32>) -> vec2<f32> {
    let res = globals.screen_size;
    let tex_dim = vec2<f32>(textureDimensions(background_tex));

    // Compute a center-crop UV transform ("cover"), preserving aspect ratio.
    // scale_px is how much we scale the texture in pixel space to cover the screen.
    let safe_tex = max(tex_dim, vec2<f32>(1.0));
    let safe_res = max(res, vec2<f32>(1.0));
    let scale_px = max(safe_res.x / safe_tex.x, safe_res.y / safe_tex.y);

    // Fraction of the texture visible in UV space after scaling (<= 1.0).
    let visible_uv = safe_res / (safe_tex * scale_px);

    return (screen_uv - vec2<f32>(0.5)) * visible_uv + vec2<f32>(0.5);
}

fn break_alpha_multiplier(time_ms: f32, break_time: vec2<f32>, break_time_lightness: f32) -> f32 {
    let start = break_time.x;
    let end = break_time.y;
    if (end <= start) {
        return 1.0;
    }

    let ramp_ms = 500.0;
    let t_from_ends = min(time_ms-start, end - time_ms);
    let w = clamp(t_from_ends / ramp_ms, 0.0, 1.0);
    return mix(1.0, 1.0 - break_time_lightness, w);
}

fn break_spinner_alpha(time_ms: f32, break_time: vec2<f32>) -> f32 {
    let start = break_time.x;
    let end = break_time.y;
    if (end <= start) {
        return 0.0;
    }

    let ramp_ms = 500.0;
    let t_from_start = time_ms - start;
    let t_from_end = end - time_ms;
    let fade_in = clamp(t_from_start / ramp_ms, 0.0, 1.0);
    let fade_out = clamp(t_from_end / ramp_ms, 0.0, 1.0);
    return min(fade_in, fade_out);
}

fn dist_point_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let ba = b - a;
    let pa = p - a;
    let denom = max(dot(ba, ba), 1e-6);
    let h = clamp(dot(pa, ba) / denom, 0.0, 1.0);
    return length(pa - ba * h);
}

fn dist_point_segment_t(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    let ba = b - a;
    let pa = p - a;
    let denom = max(dot(ba, ba), 1e-6);
    let t = clamp(dot(pa, ba) / denom, 0.0, 1.0);
    let d = length(pa - ba * t);
    return vec2<f32>(d, t);
}

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn fade_out_grow(now_ms: f32, t_ms: f32, fade_out_ms: f32) -> f32 {
    // Grow from 1.0 -> 1.25 while fading out.
    let t = saturate((now_ms - t_ms) / max(fade_out_ms, 1e-6));
    // Ease-out so the growth is noticeable before alpha gets too low.
    let te = 1.0 - (1.0 - t) * (1.0 - t);
    return mix(1.0, 1.2, te);
}

// Rotate a 2D vector by -angle(rot), where rot = (cosθ, sinθ).
// This is equivalent to multiplying by the complex conjugate so that
// sampling coordinates rotate the sprite forward by +θ.
fn rotate_inv(v: vec2<f32>, rot: vec2<f32>) -> vec2<f32> {
    // (vx + i vy) * (c - i s) = (vx*c + vy*s) + i(vy*c - vx*s)
    return vec2<f32>(
        v.x * rot.x + v.y * rot.y,
        v.y * rot.x - v.x * rot.y,
    );
}
