struct OverlayOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_overlay(
    @builtin(vertex_index) vid: u32,
) -> OverlayOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );

    let corner = corners[vid];
    let uv = vec2<f32>(corner.x * 0.5 + 0.5, 1.0 - (corner.y * 0.5 + 0.5));

    var out: OverlayOut;
    out.pos = vec4<f32>(corner, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_overlay(
    @location(0) uv: vec2<f32>,
) -> @location(0) vec4<f32> {
    var out_pm = vec3<f32>(0.0);
    var out_a: f32 = 0.0;

    if (globals.slider_radius > 1e-5) {
        let pf = globals.playfield_rect;
        let pf_size = pf.zw - pf.xy;
        let pf_scale = pf_size / vec2<f32>(512.0, 384.0);
        let px_per_pf = max(1e-6, min(pf_scale.x, pf_scale.y));

        let center_px = pf.xy + globals.slider_position * pf_scale;
        let radius_scale: f32 = 59.0 / 64.0;
        let radius_px = globals.slider_radius * radius_scale * px_per_pf;

        let follow_s = max(1e-6, globals.slider_follow_circle_scaling * skin_meta.sliderfollowcircle_scale);
        let ball_s = max(1e-6, skin_meta.sliderball_scale * 2.0);
        let max_s = max(follow_s, ball_s);

        let screen_px = uv * globals.screen_size;
        let local = (screen_px - center_px) / max(1e-6, radius_px * max_s);

        let follow_uv = local * (max_s / follow_s) + vec2<f32>(0.5);
        let ball_uv = local * (max_s / ball_s) + vec2<f32>(0.5);

        if (all(follow_uv >= vec2<f32>(0.0)) && all(follow_uv <= vec2<f32>(1.0))) {
            let layers = max(1u, textureNumLayers(sliderfollowcircle_tex));
            let t = max(globals.slider_ball_rotation_index, 0);
            let li = i32(u32(t) % layers);
            let fc = textureSample(sliderfollowcircle_tex, skin_samp, follow_uv, li);
            let src_a = fc.a;
            let src_pm = fc.rgb * src_a;
            out_pm = src_pm + out_pm * (1.0 - src_a);
            out_a = src_a + out_a * (1.0 - src_a);
        }

        if (globals.slider_ball_rotation_index >= 0) {
            let ball_uv_rot = rotate_inv(ball_uv - vec2<f32>(0.5), globals.slider_ball_direction) + vec2<f32>(0.5);

            if (all(ball_uv_rot >= vec2<f32>(0.0)) && all(ball_uv_rot <= vec2<f32>(1.0))) {
                let layers = max(1u, textureNumLayers(sliderball_tex));
                let li = i32(u32(globals.slider_ball_rotation_index) % layers);
                let sb = textureSample(sliderball_tex, skin_samp, ball_uv_rot, li);
                let src_a = sb.a;
                let tinted_rgb = sb.rgb * globals.slider_color;
                let src_pm = tinted_rgb * src_a;
                out_pm = src_pm + out_pm * (1.0 - src_a);
                out_a = src_a + out_a * (1.0 - src_a);
            }
        }
    }

    if ((globals.snap_meta.y != 0u && globals.snap_marker_style.x > 0.0)
        || (globals.snap_meta.z != 0u && globals.movable_snap_marker_style.x > 0.0)) {
        let pf = globals.playfield_rect;
        let pf_size = pf.zw - pf.xy;
        let pf_scale = pf_size / vec2<f32>(512.0, 384.0);
        let screen_px = uv * globals.screen_size;
        let aa = 1.0;

        if (globals.snap_meta.y != 0u && globals.snap_marker_style.x > 0.0) {
            let marker_radius = globals.snap_marker_style.x;
            for (var i: u32 = 0u; i < globals.snap_meta.x; i = i + 1u) {
                let marker_pf = snap_positions[i];
                let marker_px = pf.xy + marker_pf * pf_scale;
                let dist = length(screen_px - marker_px);
                let marker_a = 1.0 - smoothstep(marker_radius - aa, marker_radius + aa, dist);
                if (marker_a > 1e-5) {
                    let src_a = marker_a * globals.snap_marker_rgba.a;
                    let src_pm = globals.snap_marker_rgba.rgb * src_a;
                    out_pm = src_pm + out_pm * (1.0 - src_a);
                    out_a = src_a + out_a * (1.0 - src_a);
                }
            }
        }

        if (globals.snap_meta.z != 0u && globals.movable_snap_marker_style.x > 0.0) {
            let movable_radius = globals.movable_snap_marker_style.x;
            for (var i: u32 = 0u; i < globals.snap_meta.z; i = i + 1u) {
                let marker_pf = snap_positions[globals.snap_meta.x + i];
                let marker_px = pf.xy + marker_pf * pf_scale;
                let dist = length(screen_px - marker_px);
                let marker_a = 1.0 - smoothstep(movable_radius - aa, movable_radius + aa, dist);
                if (marker_a > 1e-5) {
                    let src_a = marker_a * globals.movable_snap_marker_rgba.a;
                    let src_pm = globals.movable_snap_marker_rgba.rgb * src_a;
                    out_pm = src_pm + out_pm * (1.0 - src_a);
                    out_a = src_a + out_a * (1.0 - src_a);
                }
            }
        }
    }

    if (globals.selection_drag_pos_right.z > 0.5 || globals.selection_drag_pos_left.z > 0.5) {
        let screen_px = uv * globals.screen_size;
        let marker_radius = globals.drag_state_marker_style.x;

        if (globals.selection_drag_pos_right.z > 0.5 && globals.selection_origin_right.w <= 0.5) {
            let drag_pos = globals.selection_drag_pos_right.xy;
            let d = length(screen_px - drag_pos);
            let ring_a = 1.0 - smoothstep(2.0, 3.5, abs(d - marker_radius));
            let center_a = 1.0 - smoothstep(2.0, 3.5, d);
            if (ring_a > 1e-3 || center_a > 1e-3) {
                let c = globals.drag_state_marker_rgba;
                let ring_col = vec4<f32>(c.rgb, c.a * ring_a);
                let center_col = vec4<f32>(c.rgb, c.a * center_a);
                var tmp = over_pm(out_pm, out_a, ring_col);
                out_pm = tmp.rgb;
                out_a = tmp.a;
                tmp = over_pm(out_pm, out_a, center_col);
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }

        if (globals.selection_drag_pos_left.z > 0.5 && globals.selection_origin_left.w <= 0.5) {
            let drag_pos = globals.selection_drag_pos_left.xy;
            let d = length(screen_px - drag_pos);
            let ring_a = 1.0 - smoothstep(2.0, 3.5, abs(d - marker_radius));
            let center_a = 1.0 - smoothstep(2.0, 3.5, d);
            if (ring_a > 1e-3 || center_a > 1e-3) {
                let c = globals.drag_state_marker_rgba;
                let ring_col = vec4<f32>(c.rgb, c.a * ring_a);
                let center_col = vec4<f32>(c.rgb, c.a * center_a);
                var tmp = over_pm(out_pm, out_a, ring_col);
                out_pm = tmp.rgb;
                out_a = tmp.a;
                tmp = over_pm(out_pm, out_a, center_col);
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }
    }

    if (globals.selection_meta.y != 0u && globals.selection_origin_right.w > 0.5) {
        let px = uv * globals.screen_size;
        let origin = globals.selection_origin_right.xy;
        let hovered = globals.selection_origin_right.z > 0.5;
        let dragging = globals.selection_origin_right.w > 0.5;
        let pos_locked = globals.selection_lock_meta.y != 0u;
        let hover_f = select(0.0, 1.0, hovered);
        let drag_f = select(0.0, 1.0, dragging);
        let d = length(px - origin);
        let ring_a = 1.0 - smoothstep(3.5, 5.5, abs(d - 21.0));
        let center_a = 1.0 - smoothstep(5.0, 7.0, d);
        let glow_a = 1.0 - smoothstep(20.0, 36.0, d);
        let active_glow_a = (1.0 - smoothstep(24.0, 54.0, d)) * max(hover_f, drag_f);
        if (ring_a > 1e-3 || center_a > 1e-3 || glow_a > 1e-3 || active_glow_a > 1e-3) {
            let c = select(
                select(
                    select(
                        side_selection_color(1u, SC_SELECTION_ORIGIN),
                        side_selection_color(1u, SC_SELECTION_ORIGIN_HOVERED),
                        hovered,
                    ),
                    side_selection_color(1u, SC_SELECTION_ORIGIN_CLICKED),
                    dragging,
                ),
                side_selection_color(1u, SC_SELECTION_ORIGIN_LOCKED),
                pos_locked,
            );
            let glow_strength = 0.18 + hover_f * 0.34 + drag_f * 0.42;
            let glow = vec4<f32>(c.rgb, c.a * glow_strength * glow_a);
            let active_glow = vec4<f32>(mix(c.rgb, vec3<f32>(1.0), 0.52), c.a * (0.40 + hover_f * 0.22 + drag_f * 0.32) * active_glow_a);
            let ring_col = vec4<f32>(mix(c.rgb, vec3<f32>(1.0), 0.44), max(c.a, 0.95) * (1.0 + hover_f * 0.70 + drag_f * 0.95) * ring_a);
            let center_col = vec4<f32>(vec3<f32>(1.0), max(c.a, 1.0) * (1.0 + hover_f * 0.28 + drag_f * 0.38) * center_a);
            var tmp = over_pm(out_pm, out_a, glow);
            out_pm = tmp.rgb;
            out_a = tmp.a;
            tmp = over_pm(out_pm, out_a, active_glow);
            out_pm = tmp.rgb;
            out_a = tmp.a;
            tmp = over_pm(out_pm, out_a, ring_col);
            out_pm = tmp.rgb;
            out_a = tmp.a;
            tmp = over_pm(out_pm, out_a, center_col);
            out_pm = tmp.rgb;
            out_a = tmp.a;
        }
    }

    if (globals.selection_meta.x != 0u && globals.selection_origin_left.w > 0.5) {
        let px = uv * globals.screen_size;
        let origin = globals.selection_origin_left.xy;
        let hovered = globals.selection_origin_left.z > 0.5;
        let dragging = globals.selection_origin_left.w > 0.5;
        let pos_locked = globals.selection_lock_meta.x != 0u;
        let hover_f = select(0.0, 1.0, hovered);
        let drag_f = select(0.0, 1.0, dragging);
        let d = length(px - origin);
        let ring_a = 1.0 - smoothstep(3.5, 5.5, abs(d - 21.0));
        let center_a = 1.0 - smoothstep(5.0, 7.0, d);
        let glow_a = 1.0 - smoothstep(20.0, 36.0, d);
        let active_glow_a = (1.0 - smoothstep(24.0, 54.0, d)) * max(hover_f, drag_f);
        if (ring_a > 1e-3 || center_a > 1e-3 || glow_a > 1e-3 || active_glow_a > 1e-3) {
            let c = select(
                select(
                    select(
                        side_selection_color(0u, SC_SELECTION_ORIGIN),
                        side_selection_color(0u, SC_SELECTION_ORIGIN_HOVERED),
                        hovered,
                    ),
                    side_selection_color(0u, SC_SELECTION_ORIGIN_CLICKED),
                    dragging,
                ),
                side_selection_color(0u, SC_SELECTION_ORIGIN_LOCKED),
                pos_locked,
            );
            let glow_strength = 0.18 + hover_f * 0.34 + drag_f * 0.42;
            let glow = vec4<f32>(c.rgb, c.a * glow_strength * glow_a);
            let active_glow = vec4<f32>(mix(c.rgb, vec3<f32>(1.0), 0.52), c.a * (0.40 + hover_f * 0.22 + drag_f * 0.32) * active_glow_a);
            let ring_col = vec4<f32>(mix(c.rgb, vec3<f32>(1.0), 0.44), max(c.a, 0.95) * (1.0 + hover_f * 0.70 + drag_f * 0.95) * ring_a);
            let center_col = vec4<f32>(vec3<f32>(1.0), max(c.a, 1.0) * (1.0 + hover_f * 0.28 + drag_f * 0.38) * center_a);
            var tmp = over_pm(out_pm, out_a, glow);
            out_pm = tmp.rgb;
            out_a = tmp.a;
            tmp = over_pm(out_pm, out_a, active_glow);
            out_pm = tmp.rgb;
            out_a = tmp.a;
            tmp = over_pm(out_pm, out_a, ring_col);
            out_pm = tmp.rgb;
            out_a = tmp.a;
            tmp = over_pm(out_pm, out_a, center_col);
            out_pm = tmp.rgb;
            out_a = tmp.a;
        }
    }

    if (out_a <= 1e-5) {
        discard;
    }
    return vec4<f32>(out_pm, out_a);
}
