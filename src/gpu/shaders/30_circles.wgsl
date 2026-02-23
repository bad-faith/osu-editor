@vertex
fn vs_main(
    @builtin(vertex_index) vid: u32,
    @builtin(instance_index) iid: u32,
) -> VsOut {
    // Two triangles (6 verts) forming a quad.
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );

    let p = circles[iid];
    // Circles are provided in 512x384 playfield coordinates (osu! playfield space).
    // Convert them to screen pixel coordinates using the centered/scaled playfield rect.
    let pf = globals.playfield_rect;
    let pf_size = pf.zw - pf.xy;
    let pf_scale = pf_size / vec2<f32>(512.0, 384.0);

    let center_px = pf.xy + p.center_xy * pf_scale;
    // The playfield is aspect-locked, so pf_scale.x == pf_scale.y.
    let radius_px = p.radius * pf_scale.x;
    let time_ms = p.time_ms;

    let color = p.color;
    let preempt_ms = p.preempt_ms;

    // While fading out, slightly grow the sprite (osu!-like hit animation).
    // Must match fs_main's fade-out timing.
    let now_ms: f32 = globals.time_ms;
    let fade_out_ms: f32 = 250.0;
    let is_selected: bool = p.selected_side != 0u;
    let grow_boost: f32 = 1.2;
    let grow_raw: f32 = fade_out_grow(now_ms, time_ms, fade_out_ms);
    let grow: f32 = select(
        1.0 + (grow_raw - 1.0) * grow_boost,
        1.0,
        is_selected,
    );

    let approach_start = p.approach_circle_start_scale;
    let approach_end = p.approach_circle_end_scale;

    let combo = p.combo;
    let is_slider = p.is_slider;
    let corner = corners[vid];

    // Local UV for sampling (0..1)
    let uv = corner * 0.5 + vec2<f32>(0.5);

    // Convert from pixel coords (top-left origin) to clip space.
    let res = globals.screen_size;
    // This quad is sized to the maximum of approach-circle scale and any skin sprite scale.
    let base_s = select(skin_meta.hitcircle_scale, skin_meta.sliderstartcircle_scale, is_slider != 0u);
    let over_s = select(skin_meta.hitcircleoverlay_scale, skin_meta.sliderstartcircleoverlay_scale, is_slider != 0u);
    let sprite_s = max(base_s, over_s);
    let max_scale_unselected = max(1.0, max(max(approach_start, approach_end), sprite_s));
    let max_scale_selected = max_scale_unselected;
    let max_scale = select(max_scale_unselected, max_scale_selected, is_selected);
    let px = center_px + corner * (radius_px * max_scale * grow);
    let ndc = vec2<f32>(
        (px.x / res.x) * 2.0 - 1.0,
        1.0 - (px.y / res.y) * 2.0,
    );

    var out: VsOut;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = uv;
    out.color = color;
    out.combo = combo;
    out.time_ms = time_ms;
    out.preempt_ms = preempt_ms;
    out.is_slider = is_slider;
    out.approach_start = approach_start;
    out.approach_end = approach_end;
    out.selected_side = p.selected_side;
    out.screen_px = px;
    out.center_screen_px = center_px;
    return out;
}

@fragment
fn fs_main(
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) @interpolate(flat) combo: u32,
    @location(3) @interpolate(flat) time_ms: f32,
    @location(4) @interpolate(flat) preempt_ms: f32,
    @location(5) @interpolate(flat) is_slider: u32,
    @location(6) @interpolate(flat) approach_start: f32,
    @location(7) @interpolate(flat) approach_end: f32,
    @location(8) @interpolate(flat) selected_side: u32,
    @location(9) screen_px: vec2<f32>,
    @location(10) @interpolate(flat) center_screen_px: vec2<f32>,
) -> @location(0) vec4<f32> {
    // --- Time-based opacity (shared by hitcircle + approach circle) ---
    // `globals.time_ms` is editor time in milliseconds.
    let now_ms: f32 = globals.time_ms;
    let appear_ms: f32 = time_ms - preempt_ms;

    let fade_in_ms: f32 = max(preempt_ms * 0.6666666666666666666666666667, 1e-6);
    let fade_in_alpha: f32 = saturate((now_ms - appear_ms) / fade_in_ms);

    // Fade out shortly after hit time.
    let fade_out_ms: f32 = 250.0;
    let fade_out_t: f32 = saturate((now_ms - time_ms) / fade_out_ms);
    let fade_out_alpha: f32 = pow(1.0 - fade_out_t, 2.0);

    // While fading out, slightly grow the sprite. Must match vs_main.
    let is_selected: bool = selected_side != 0u;
    let grow_boost: f32 = 1.2;
    let grow_raw: f32 = fade_out_grow(now_ms, time_ms, fade_out_ms);
    let grow: f32 = select(
        1.0 + (grow_raw - 1.0) * grow_boost,
        1.0,
        is_selected,
    );

    let alpha: f32 = fade_in_alpha * fade_out_alpha;
    if (alpha <= 1e-4) {
        // Early-out to avoid texture sampling for invisible instances.
        if (!is_selected) {
            discard;
        }
    }

    let left_selection_rgb = globals.left_selection_colors[10].rgb;
    let right_selection_rgb = globals.right_selection_colors[10].rgb;
    let dual_selected = selected_side == 3u;
    let selection_rgb = select(
        select(left_selection_rgb, right_selection_rgb, selected_side == 2u),
        select(left_selection_rgb, right_selection_rgb, uv.x >= 0.5),
        dual_selected,
    );
    let selection_mix_strength = clamp(globals.selection_color_mix_strength, 0.0, 1.0);
    let selected_draw_color = mix(color, selection_rgb, selection_mix_strength);
    let draw_color = select(color, selected_draw_color, is_selected);

    // Must match vs_main's max_scale calculation (excluding `grow`).
    let base_s = select(skin_meta.hitcircle_scale, skin_meta.sliderstartcircle_scale, is_slider != 0u);
    let over_s = select(skin_meta.hitcircleoverlay_scale, skin_meta.sliderstartcircleoverlay_scale, is_slider != 0u);
    let sprite_s = max(base_s, over_s);
    let max_scale_unselected = max(1.0, max(max(approach_start, approach_end), sprite_s));
    let max_scale_selected = max_scale_unselected;
    let max_scale = select(max_scale_unselected, max_scale_selected, is_selected);

    // Hitcircle UV (scale=1.0 relative to max quad).
    // The base/overlay textures can be authored bigger than nominal (128px or 256px for @2x).
    // We emulate osu!'s behavior by letting those textures occupy a larger fraction of this quad.
    // NOTE: Avoid `select(textureSample(...), ...)` because it still samples even when out-of-bounds.
    let hit_uv_base = (uv - vec2<f32>(0.5)) * (max_scale / max(base_s, 1e-6)) + vec2<f32>(0.5);
    let hit_uv_over = (uv - vec2<f32>(0.5)) * (max_scale / max(over_s, 1e-6)) + vec2<f32>(0.5);
    let hit_in_base = all(hit_uv_base >= vec2<f32>(0.0)) && all(hit_uv_base <= vec2<f32>(1.0));
    let hit_in_over = all(hit_uv_over >= vec2<f32>(0.0)) && all(hit_uv_over <= vec2<f32>(1.0));

    var base = vec4<f32>(0.0);
    var over = vec4<f32>(0.0);
    if (hit_in_base || hit_in_over) {
        if (is_slider != 0u) {
            if (hit_in_base) {
                base = textureSample(slidercircle_tex, skin_samp, hit_uv_base);
            }
            if (hit_in_over) {
                over = textureSample(slidercircle_overlay_tex, skin_samp, hit_uv_over);
            }
        } else {
            if (hit_in_base) {
                base = textureSample(hitcircle_tex, skin_samp, hit_uv_base);
            }
            if (hit_in_over) {
                over = textureSample(hitcircle_overlay_tex, skin_samp, hit_uv_over);
            }
        }
    }

    // --- Approach circle (behind hitcircle) ---
    var approach_texel = vec4<f32>(0.0);
    let denom: f32 = max(preempt_ms, 1e-6);
    // Clamp the approach-circle scaling animation at hit time so it stays at the end size
    // during fade-out.
    let anim_ms: f32 = min(now_ms, time_ms);
    let t01: f32 = saturate((anim_ms - appear_ms) / denom);

    // Scale from start -> end over the preempt window.
    let approach_scale: f32 = mix(approach_start, approach_end, t01);
    // Compensate for the quad growth so the approach circle itself doesn't grow.
    let rel: f32 = max(1e-6, approach_scale / (max_scale * grow));
    let approach_uv = (uv - vec2<f32>(0.5)) / rel + vec2<f32>(0.5);
    let approach_in = all(approach_uv >= vec2<f32>(0.0)) && all(approach_uv <= vec2<f32>(1.0));
    if (approach_in) {
        approach_texel = textureSample(approach_circle_tex, skin_samp, approach_uv);
    }

    // Tint by combo color; premultiply.
    let approach_rgb = approach_texel.rgb * draw_color;
    let approach_a = approach_texel.a;
    let approach_pm = approach_rgb * approach_a;

    // Tint the base hitcircle by the combo color; keep overlay as-is.
    let base_rgb = base.rgb * draw_color;

    // Composite hitcircle in premultiplied-alpha space to avoid edge/quad artifacts when
    // sampling straight-alpha textures with linear filtering.
    let base_pm = base_rgb * base.a;
    let over_pm = over.rgb * over.a;
    var a = base.a + over.a * (1.0 - base.a);
    var pm = base_pm * (1.0 - over.a) + over_pm;

    // Draw combo number at the center of the hitcircle.
    // IMPORTANT: size digits in *hitcircle UV space* so approach-circle quad scaling
    // doesn't make the numbers appear larger.
    if (hit_in_base) {
        let digits_uv = (hit_uv_base - vec2<f32>(0.5)) * max(grow, 1e-6) + vec2<f32>(0.5);

        // We composite up to 3 digits here; spacing depends on each digit's real width.
        let n = combo;
        var digits: array<u32, 3>;
        var count: u32 = 1u;
        if (n >= 100u) {
            digits[0] = (n / 100u) % 10u;
            digits[1] = (n / 10u) % 10u;
            digits[2] = n % 10u;
            count = 3u;
        } else if (n >= 10u) {
            digits[0] = (n / 10u) % 10u;
            digits[1] = n % 10u;
            digits[2] = 0u;
            count = 2u;
        } else {
            digits[0] = n % 10u;
            digits[1] = 0u;
            digits[2] = 0u;
            count = 1u;
        }

        // Height fraction of the hitcircle (in hitcircle UV-space).
        let digit_scale_y: f32 = 0.38;
        let digit_gap: f32 = 0.03 * digit_scale_y;

        // Compute total width from per-digit aspect ratios.
        var widths: array<f32, 3>;
        var total_w: f32 = 0.0;
        for (var i: u32 = 0u; i < 3u; i = i + 1u) {
            if (i >= count) {
                widths[i] = 0.0;
                continue;
            }
            let di = digits[i];
            let xf = digits_meta.uv_xform[di];
            let digit_w_px = max(1e-6, xf.x * digits_meta.max_size_px.x);
            let digit_h_px = max(1e-6, xf.y * digits_meta.max_size_px.y);
            let aspect = digit_w_px / digit_h_px;
            let w_uv = digit_scale_y * aspect;
            widths[i] = w_uv;
            total_w = total_w + w_uv;
        }
        if (count > 1u) {
            total_w = total_w + digit_gap * f32(count - 1u);
        }

        let start_x = 0.5 - total_w * 0.5;
        var cursor_x: f32 = start_x;
        for (var i: u32 = 0u; i < 3u; i = i + 1u) {
            if (i >= count) {
                break;
            }
            let di = digits[i];
            let w_uv = widths[i];
            let center = vec2<f32>(cursor_x + w_uv * 0.5, 0.5);
            let local_uv = (digits_uv - center) / vec2<f32>(w_uv, digit_scale_y) + vec2<f32>(0.5);
            if (all(local_uv >= vec2<f32>(0.0)) && all(local_uv <= vec2<f32>(1.0))) {
                let xf = digits_meta.uv_xform[di];
                let digit_uv2 = local_uv * xf.xy + xf.zw;
                let digit = textureSample(numbers_tex, skin_samp, digit_uv2, i32(di));
                let digit_pm = digit.rgb * digit.a;
                pm = pm * (1.0 - digit.a) + digit_pm;
                a = a + digit.a * (1.0 - a);
            }
            cursor_x = cursor_x + w_uv + digit_gap;
        }
    }

    // Black combo tint only on the exact portion outside gameplay playfield
    // (top/left/right only), and only when at least half of the circle is
    // outside for that edge (center is beyond the edge).
    // Applied to hitcircle composition only, before approach-circle compositing.
    let pf = globals.playfield_rect;
    let outside_playfield_tlr =
        (screen_px.y < pf.y && center_screen_px.y < pf.y)
        || (screen_px.x < pf.x && center_screen_px.x < pf.x)
        || (screen_px.x > pf.z && center_screen_px.x > pf.z);
    if (outside_playfield_tlr) {
        pm = globals.offscreen_playfield_tint_rgba.rgb * a;
    }

    // If this pixel is outside the 640x480 outer playfield, make the hitcircle
    // very visible. This is intentionally applied before approach compositing so
    // approach circles keep their normal appearance.
    let os = globals.osu_rect;
    let outside_osu =
        screen_px.x < os.x || screen_px.x > os.z || screen_px.y < os.y || screen_px.y > os.w;
    if (outside_osu) {
        pm = globals.offscreen_osu_tint_rgba.rgb * a;
    }
    let offscreen_tinted = outside_playfield_tlr || outside_osu;

    // Apply selected opacity floor only to hitcircle/overlay content.
    let selected_fade_in_cap = clamp(globals.selected_fade_in_opacity_cap, 0.0, 1.0);
    let selected_fade_out_cap = clamp(globals.selected_fade_out_opacity_cap, 0.0, 1.0);
    let selected_cap = select(selected_fade_in_cap, selected_fade_out_cap, now_ms > time_ms);
    let body_alpha = select(alpha, max(alpha, selected_cap), is_selected);
    pm = pm * body_alpha;
    a = a * body_alpha;

    if (offscreen_tinted && a > 1e-4) {
        let offscreen_alpha_mult: f32 = select(0.6, 0.4, now_ms > time_ms);
        pm = pm * offscreen_alpha_mult;
        a = a * offscreen_alpha_mult;
    }

    // Approach circle keeps normal fade (no selected 20% floor).
    let approach_alpha = approach_a * alpha;
    let approach_pm_alpha = approach_pm * alpha;
    pm = pm * (1.0 - approach_alpha) + approach_pm_alpha;
    a = a + approach_alpha * (1.0 - a);

    // Prevent empty quad pixels from writing depth.
    if (a <= 1e-5) {
        discard;
    }

    // Output premultiplied RGB; pipeline uses premultiplied alpha blending.
    return vec4<f32>(pm, a);
}
