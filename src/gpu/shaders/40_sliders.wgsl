@vertex
fn vs_slider_box(
    @builtin(vertex_index) vid: u32,
    @builtin(instance_index) iid: u32,
) -> SliderVsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );

    let b = slider_boxes[iid];
    let obj_iid = b.obj_iid;
    let p = circles[obj_iid];

    let bb_min_pf = b.bbox_min;
    let bb_max_pf = b.bbox_max;

    let pf = globals.playfield_rect;
    let pf_size = pf.zw - pf.xy;
    let pf_scale = pf_size / vec2<f32>(512.0, 384.0);

    // UV for the bbox quad.
    let corner = corners[vid];
    let uv = corner * 0.5 + vec2<f32>(0.5);

    // Interpolate bbox corners in playfield space, then map to pixel coords.
    let pf_pos = mix(bb_min_pf, bb_max_pf, uv);
    let px = pf.xy + pf_pos * pf_scale;

    let res = globals.screen_size;
    let ndc = vec2<f32>(
        (px.x / res.x) * 2.0 - 1.0,
        1.0 - (px.y / res.y) * 2.0,
    );

    var out: SliderVsOut;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.pf_pos = pf_pos;
    out.color = p.color;
    out.time_ms = p.time_ms;
    out.preempt_ms = p.preempt_ms;
    out.slider_end_time_ms = p.slider_end_time_ms;
    out.radius = p.radius;
    out.seg_start = b.seg_start;
    out.seg_count = b.seg_count;
    out.obj_iid = obj_iid;
    out.bbox_min = b.bbox_min;
    out.bbox_max = b.bbox_max;
    return out;
}

@fragment
fn fs_slider_box(
    @location(0) pf_pos: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) @interpolate(flat) time_ms: f32,
    @location(3) @interpolate(flat) preempt_ms: f32,
    @location(4) @interpolate(flat) slider_end_time_ms: f32,
    @location(5) @interpolate(flat) radius: f32,
    @location(6) @interpolate(flat) seg_start: u32,
    @location(7) @interpolate(flat) seg_count: u32,
    @location(8) @interpolate(flat) obj_iid: u32,
    @location(9) @interpolate(flat) bbox_min: vec2<f32>,
    @location(10) @interpolate(flat) bbox_max: vec2<f32>,
) -> @location(0) vec4<f32> {
    let now_ms: f32 = globals.time_ms;
    let appear_ms: f32 = time_ms - preempt_ms;
    let p = circles[obj_iid];
    let is_selected: bool = p.selected_side != 0u;

    let fade_in_ms: f32 = max(preempt_ms * 0.6666666666666666666666666667, 1e-6);
    let fade_in_alpha: f32 = saturate((now_ms - appear_ms) / fade_in_ms);

    // Start fading out when the slider ends (not when it starts).
    let fade_out_ms: f32 = 250.0;
    let fade_out_alpha: f32 = 1.0 - saturate((now_ms - slider_end_time_ms) / fade_out_ms);

    let alpha_raw: f32 = fade_in_alpha * fade_out_alpha;
    let selected_fade_in_cap = clamp(globals.selected_fade_in_opacity_cap, 0.0, 1.0);
    let selected_fade_out_cap = clamp(globals.selected_fade_out_opacity_cap, 0.0, 1.0);
    let selected_cap = select(selected_fade_in_cap, selected_fade_out_cap, now_ms > slider_end_time_ms);
    let alpha: f32 = select(alpha_raw, max(alpha_raw, selected_cap), is_selected);
    if (alpha <= 1e-4) {
        if (!is_selected) {
            discard;
        }
    }

    if (seg_count < 1u) {
        discard;
    }

    // This pass renders the slider body (from the path polyline) per box.
    var out_a: f32 = 0.0;
    var out_pm: vec3<f32> = vec3<f32>(0.0);

    let border_start_rgb = vec3<f32>(
        f32(p.slider_start_border_color[0]),
        f32(p.slider_start_border_color[1]),
        f32(p.slider_start_border_color[2]),
    ) / 255.0;
    let border_end_rgb = vec3<f32>(
        f32(p.slider_end_border_color[0]),
        f32(p.slider_end_border_color[1]),
        f32(p.slider_end_border_color[2]),
    ) / 255.0;

    // --- Slider body (GPU-generated from the slider path) ---
    // We compute the minimum distance to any ridge segment and shade a capsule strip.
    let max_segs_per_box: u32 = 1024u;
    let seg_n = min(seg_count, max_segs_per_box);

    var min_d: f32 = 1e9;
    var min_prog: f32 = 0.0;
    var si: u32 = 0u;
    loop {
        if (si >= seg_n) {
            break;
        }
        let s = slider_segs[seg_start + si];
        let dt = dist_point_segment_t(pf_pos, s.ridge0.xy, s.ridge1.xy);
        if (dt.x < min_d) {
            min_d = dt.x;
            min_prog = mix(s.ridge0.z, s.ridge1.z, dt.y);
        }
        si = si + 1u;
    }

    // Anti-aliased capsule strip (same shading as the old per-segment pass).
    // Convert AA width from pixels to playfield units so high-res renders look crisp.
    let pf = globals.playfield_rect;
    let pf_size = pf.zw - pf.xy;
    let pf_scale = pf_size / vec2<f32>(512.0, 384.0);
    let px_per_pf = max(1e-6, min(pf_scale.x, pf_scale.y));
    let aa = 0.75 / px_per_pf;
    let radius_scale: f32 = 59.0 / 64.0;
    let base_r = radius * radius_scale;
    let inner_t_ratio = saturate(globals.slider_border_thickness);
    let outer_t_ratio = saturate(globals.slider_border_outer_thickness);
    if (min_d <= base_r + base_r * outer_t_ratio + aa) {
        let inner_r = base_r * (1.0 - inner_t_ratio);
        let outer_t = base_r * outer_t_ratio;
        let outer_r = base_r + outer_t;

        // If this pixel is outside the 640x480 outer playfield, recolor while
        // preserving the exact AA coverage/alpha from normal slider shading.
        let px = pf.xy + pf_pos * pf_scale;
        let os = globals.osu_rect;
        let outside_osu = px.x < os.x || px.x > os.z || px.y < os.y || px.y > os.w;

        let outer_edge_a = 1.0 - smoothstep(outer_r - aa, outer_r + aa, min_d);
        let base_edge_a = 1.0 - smoothstep(base_r - aa, base_r + aa, min_d);
        let inner_edge_a = smoothstep(inner_r - aa, inner_r + aa, min_d);

        // Fill: configurable ridge/body colors (still fades with slider alpha).
        let fill_a_outside: f32 = (1.0 - inner_edge_a) * alpha * globals.slider_body_rgba.a;
        let fill_a_center: f32 = (1.0 - inner_edge_a) * alpha * globals.slider_ridge_rgba.a;
        let border_alpha: f32 = alpha;

        let fill_color_outside: vec3<f32> = globals.slider_body_rgba.rgb;
        let fill_color_center: vec3<f32> = globals.slider_ridge_rgba.rgb;

        let distance_from_ridge_01 = min_d / base_r;
        
        // Darken fill towards ridge center.
        let fill_rgb = mix(fill_color_center, fill_color_outside, saturate(distance_from_ridge_01));
        let fill_a = mix(fill_a_center, fill_a_outside, saturate(distance_from_ridge_01));

        // Inner border: inner_r..base_r (gradient from start -> end by progress).
        let inner_border_a: f32 = (base_edge_a * inner_edge_a) * border_alpha;
        let inner_border_rgb = mix(border_start_rgb, border_end_rgb, saturate(min_prog));
        let inner_border_pm: vec3<f32> = inner_border_rgb * inner_border_a;

        // Premultiplied composite: inner border over fill.
        out_a = inner_border_a + fill_a * (1.0 - inner_border_a);
        out_pm = inner_border_pm + (fill_rgb * fill_a) * (1.0 - inner_border_a);

        // Outer border: base_r..outer_r (solid start border color).
        let outer_border_a: f32 = (outer_edge_a * (1.0 - base_edge_a)) * border_alpha;
        let outer_border_rgb = border_start_rgb;
        let outer_border_pm: vec3<f32> = outer_border_rgb * outer_border_a;

        out_a = outer_border_a + out_a * (1.0 - outer_border_a);
        out_pm = outer_border_pm + out_pm * (1.0 - outer_border_a);

        if (outside_osu) {
            let offscreen_alpha_mult: f32 = select(0.6, 0.4, now_ms > slider_end_time_ms);
            out_a = out_a * offscreen_alpha_mult;
            out_pm = globals.offscreen_osu_tint_rgba.rgb * out_a;
        }
    }

    if (out_a <= 1e-6) {
        discard;
    }
    return vec4<f32>(out_pm, out_a);
}

@vertex
fn vs_slider_caps(
    @builtin(vertex_index) vid: u32,
    @builtin(instance_index) iid: u32,
) -> SliderCapsVsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );

    let obj_iid = slider_draw_indices[iid];
    let p = circles[obj_iid];
    let is_selected: bool = p.selected_side != 0u;

    let start_pf = p.center_xy;
    let end_pf = p.slider_end_center_xy;

    let pf = globals.playfield_rect;
    let pf_size = pf.zw - pf.xy;
    let pf_scale = pf_size / vec2<f32>(512.0, 384.0);

    let now_ms: f32 = globals.time_ms;
    let fade_out_grow_ms: f32 = 250.0;
    let grow: f32 = fade_out_grow(now_ms, p.slider_end_time_ms, fade_out_grow_ms);

    let scaled_radius = p.radius;
    let endcap_scale = max(skin_meta.sliderendcircle_scale, skin_meta.sliderendcircleoverlay_scale);
    let endcap_extent_pf = max(1e-6, (2.0 * scaled_radius) * endcap_scale) * grow;
    let arrow_extent_pf = max(1e-6, (2.0 * scaled_radius) * skin_meta.reversearrow_scale);
    let extent_pf = max(endcap_extent_pf, arrow_extent_pf);

    let bb_min_pf = min(start_pf, end_pf) - vec2<f32>(extent_pf);
    let bb_max_pf = max(start_pf, end_pf) + vec2<f32>(extent_pf);

    let corner = corners[vid];
    let uv = corner * 0.5 + vec2<f32>(0.5);
    let pf_pos = mix(bb_min_pf, bb_max_pf, uv);
    let px = pf.xy + pf_pos * pf_scale;

    let res = globals.screen_size;
    let ndc = vec2<f32>(
        (px.x / res.x) * 2.0 - 1.0,
        1.0 - (px.y / res.y) * 2.0,
    );

    var out: SliderCapsVsOut;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.pf_pos = pf_pos;
    out.obj_iid = obj_iid;
    return out;
}

@fragment
fn fs_slider_caps(
    @location(0) pf_pos: vec2<f32>,
    @location(1) @interpolate(flat) obj_iid: u32,
) -> @location(0) vec4<f32> {
    let p = circles[obj_iid];
    let is_selected: bool = p.selected_side != 0u;

    let now_ms: f32 = globals.time_ms;
    let appear_ms: f32 = p.time_ms - p.preempt_ms;

    let fade_in_ms: f32 = max(p.preempt_ms * 0.6666666666666666666666666667, 1e-6);
    let fade_in_alpha: f32 = saturate((now_ms - appear_ms) / fade_in_ms);

    // Start fading out when the slider ends (not when it starts).
    let fade_out_ms: f32 = 250.0;
    let fade_out_alpha: f32 = 1.0 - saturate((now_ms - p.slider_end_time_ms) / fade_out_ms);

    let alpha_raw: f32 = fade_in_alpha * fade_out_alpha;
    let selected_fade_in_cap = clamp(globals.selected_fade_in_opacity_cap, 0.0, 1.0);
    let selected_fade_out_cap = clamp(globals.selected_fade_out_opacity_cap, 0.0, 1.0);
    let selected_cap = select(selected_fade_in_cap, selected_fade_out_cap, now_ms > p.slider_end_time_ms);
    let alpha: f32 = select(alpha_raw, max(alpha_raw, selected_cap), is_selected);
    if (alpha <= 1e-4) {
        discard;
    }

    let c0 = vec3<f32>(
        f32(p.slider_start_border_color[0]),
        f32(p.slider_start_border_color[1]),
        f32(p.slider_start_border_color[2]),
    ) / 255.0;
    let c1 = vec3<f32>(
        f32(p.slider_end_border_color[0]),
        f32(p.slider_end_border_color[1]),
        f32(p.slider_end_border_color[2]),
    ) / 255.0;

    var out_a: f32 = 0.0;
    var out_pm: vec3<f32> = vec3<f32>(0.0);

    let start_pf = p.center_xy;
    let end_pf = p.slider_end_center_xy;
    let slides = p.slides;

    // Compute the final endpoint based on parity (osu reverses direction each slide).
    let even = (slides % 2u) == 0u;
    let final_end_pf = select(end_pf, start_pf, even);
    // Tint for the final end-cap: if we end at the start (even slides), use start border color.
    // Otherwise, use end border color.
    let endcap_tint_rgb = select(c1, c0, even);

    // Convert playfield deltas to pixel deltas so sprites don't get squished if the
    // playfield is scaled non-uniformly.
    let pf = globals.playfield_rect;
    let pf_size = pf.zw - pf.xy;
    let pf_scale = pf_size / vec2<f32>(512.0, 384.0);
    let px_per_pf = max(1e-6, min(pf_scale.x, pf_scale.y));

    let fade_out_grow_ms: f32 = 250.0;
    let grow: f32 = select(fade_out_grow(now_ms, p.slider_end_time_ms, fade_out_grow_ms), 1.0, is_selected);

    let scaled_radius = p.radius;
    let endcap_scale = max(skin_meta.sliderendcircle_scale, skin_meta.sliderendcircleoverlay_scale);
    let endcap_extent_px = max(1e-6, (2.0 * scaled_radius) * endcap_scale * px_per_pf) * grow;
    let arrow_extent_px = max(1e-6, (2.0 * scaled_radius) * skin_meta.reversearrow_scale * px_per_pf);

    {
        let uv_end = ((pf_pos - final_end_pf) * pf_scale) / endcap_extent_px + vec2<f32>(0.5);
        let in_end = all(uv_end >= vec2<f32>(0.0)) && all(uv_end <= vec2<f32>(1.0));
        if (in_end) {
            let base = textureSample(sliderendcircle_tex, skin_samp, uv_end);
            let over = textureSample(sliderendcircle_overlay_tex, skin_samp, uv_end);

            // Premultiply and composite base + overlay first.
            let base_a = base.a;
            let over_a = over.a;
            let comp_a = base_a + over_a * (1.0 - base_a);
            // Tint base by the slider end-cap border color; keep overlay as-authored.
            let base_rgb = base.rgb * endcap_tint_rgb;
            let comp_pm = base_rgb * base_a * (1.0 - over_a) + over.rgb * over_a;

            let src_a = comp_a * alpha;
            let src_pm = comp_pm * alpha;
            out_pm = src_pm + out_pm * (1.0 - src_a);
            out_a = src_a + out_a * (1.0 - src_a);
        }
    }

    // Reverse arrows: show up to 2 repeat markers (end then start), unrotated for now.
    // slides >= 2 => reverse at end after first slide
    // slides >= 3 => reverse at start after second slide
    if (slides >= 2u) {
        let local_r1 = ((pf_pos - end_pf) * pf_scale) / arrow_extent_px;
        let uv_r1 = rotate_inv(local_r1, p.slider_end_rotation) + vec2<f32>(0.5);
        let in_r1 = all(uv_r1 >= vec2<f32>(0.0)) && all(uv_r1 <= vec2<f32>(1.0));
        if (in_r1) {
            let t = textureSample(reverse_arrow_tex, skin_samp, uv_r1);
            let src_a = t.a * alpha;
            let src_pm = t.rgb * src_a;
            out_pm = src_pm + out_pm * (1.0 - src_a);
            out_a = src_a + out_a * (1.0 - src_a);
        }
    }

    if (slides >= 3u) {
        let local_r2 = ((pf_pos - start_pf) * pf_scale) / arrow_extent_px;
        let uv_r2 = rotate_inv(local_r2, p.slider_head_rotation) + vec2<f32>(0.5);
        let in_r2 = all(uv_r2 >= vec2<f32>(0.0)) && all(uv_r2 <= vec2<f32>(1.0));
        if (in_r2) {
            let t = textureSample(reverse_arrow_tex, skin_samp, uv_r2);
            let src_a = t.a * alpha;
            let src_pm = t.rgb * src_a;
            out_pm = src_pm + out_pm * (1.0 - src_a);
            out_a = src_a + out_a * (1.0 - src_a);
        }
    }

    let px = pf.xy + pf_pos * pf_scale;
    let os = globals.osu_rect;
    let outside_osu = px.x < os.x || px.x > os.z || px.y < os.y || px.y > os.w;
    if (outside_osu && out_a > 1e-4) {
        let offscreen_alpha_mult: f32 = select(0.6, 0.4, now_ms > p.slider_end_time_ms);
        out_a = out_a * offscreen_alpha_mult;
        out_pm = globals.offscreen_osu_tint_rgba.rgb * out_a;
    }

    if (out_a <= 1e-6) {
        discard;
    }
    return vec4<f32>(out_pm, out_a);
}
