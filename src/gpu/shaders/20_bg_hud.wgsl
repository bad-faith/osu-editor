@vertex
fn vs_bg(@builtin(vertex_index) vid: u32) -> BgOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );

    let corner = corners[vid];
    // NDC Y is bottom->top (-1..1), but texture UV Y is top->bottom (0..1).
    // Flip UV.y so the background image isn't vertically inverted.
    let uv = vec2<f32>(corner.x * 0.5 + 0.5, 1.0 - (corner.y * 0.5 + 0.5));

    var out: BgOut;
    out.pos = vec4<f32>(corner, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@vertex
fn vs_hud(@builtin(vertex_index) vid: u32) -> HudOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );

    let corner = corners[vid];
    // Screen UV: (0,0) top-left, (1,1) bottom-right.
    let uv = vec2<f32>(corner.x * 0.5 + 0.5, 1.0 - (corner.y * 0.5 + 0.5));

    var out: HudOut;
    out.pos = vec4<f32>(corner, 0.0, 1.0);
    out.uv = uv;
    return out;
}

fn timeline_fill_x(total: f32, bar_x0: f32, bar_x1: f32) -> f32 {
    let t = clamp(globals.time_ms, 0.0, total);
    let frac = select(0.0, clamp(t / max(total, 1.0), 0.0, 1.0), total > 0.0);
    return mix(bar_x0, bar_x1, frac);
}

fn px_in_timeline_interval(px_x: f32, total: f32, bar_x0: f32, bar_x1: f32, count: u32) -> bool {
    if (total <= 0.0) {
        return false;
    }
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let interval = timeline_marks[i];
        let start_ms = interval.x;
        let end_ms = interval.y;
        if (end_ms <= start_ms) {
            continue;
        }
        let start_frac = clamp(start_ms / max(total, 1.0), 0.0, 1.0);
        let end_frac = clamp(end_ms / max(total, 1.0), 0.0, 1.0);
        let x0 = mix(bar_x0, bar_x1, min(start_frac, end_frac));
        let x1 = mix(bar_x0, bar_x1, max(start_frac, end_frac));
        if (px_x >= x0 && px_x <= x1) {
            return true;
        }
    }
    return false;
}

struct TimelineSliderBoxOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) px: vec2<f32>,
    @location(1) @interpolate(flat) segment_start: u32,
    @location(2) @interpolate(flat) segment_count: u32,
};

@vertex
fn vs_timeline_slider_boxes(
    @builtin(vertex_index) vid: u32,
    @builtin(instance_index) iid: u32,
) -> TimelineSliderBoxOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );

    let xb = timeline_x_boxes[iid];
    let half_h = globals.timeline_style.x
        + globals.timeline_style.x * globals.timeline_style.y
        + 2.0;
    let y0 = globals.top_timeline_rect.y + 0.5 * (globals.top_timeline_rect.w - globals.top_timeline_rect.y) - half_h;
    let y1 = globals.top_timeline_rect.y + 0.5 * (globals.top_timeline_rect.w - globals.top_timeline_rect.y) + half_h;

    let corner = corners[vid];
    let uv = corner * 0.5 + vec2<f32>(0.5);
    let px = vec2<f32>(mix(xb.x1, xb.x2, uv.x), mix(y0, y1, uv.y));

    let res = globals.screen_size;
    let ndc = vec2<f32>(
        (px.x / res.x) * 2.0 - 1.0,
        1.0 - (px.y / res.y) * 2.0,
    );

    var out: TimelineSliderBoxOut;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.px = px;
    out.segment_start = xb.segment_start;
    out.segment_count = xb.segment_count;
    return out;
}

@fragment
fn fs_timeline_slider_boxes(
    @location(0) px: vec2<f32>,
    @location(1) @interpolate(flat) segment_start: u32,
    @location(2) @interpolate(flat) segment_count: u32,
) -> @location(0) vec4<f32> {
    let opacity = clamp(globals.hud_opacity, 0.0, 1.0);
    if (opacity <= 1e-6 || segment_count < 2u) {
        discard;
    }

    let aa = 1.0;
    var out_pm = vec3<f32>(0.0);
    var out_a: f32 = 0.0;

    let point_end = segment_start + segment_count;
    let seg_end = point_end - 1u;
    var i = segment_start;
    loop {
        if (i >= seg_end) {
            break;
        }

        let p0 = timeline_points[i];
        let p0_is_slider_marker = (p0.is_slide_start | p0.is_slide_repeat | p0.is_slide_end) != 0u;
        if (!p0_is_slider_marker || p0.is_slide_end != 0u) {
            i = i + 1u;
            continue;
        }

        let group_selected = p0.is_selected;
        let group_selected_by_left = p0.is_selected_by_left;
        let group_color = p0.color;

        var min_d: f32 = 1e9;
        var j = i;
        var consumed_pair = false;
        loop {
            if (j >= seg_end) {
                break;
            }

            let a_pt = timeline_points[j];
            let b_pt = timeline_points[j + 1u];
            let a_is_slider_marker = (a_pt.is_slide_start | a_pt.is_slide_repeat | a_pt.is_slide_end) != 0u;
            let b_is_slider_marker = (b_pt.is_slide_start | b_pt.is_slide_repeat | b_pt.is_slide_end) != 0u;
            let valid_pair = a_is_slider_marker
                && b_is_slider_marker
                && a_pt.is_slide_end == 0u
                && b_pt.is_slide_start == 0u
                && a_pt.is_selected == group_selected
                && b_pt.is_selected == group_selected
                && a_pt.is_selected_by_left == group_selected_by_left
                && b_pt.is_selected_by_left == group_selected_by_left;

            if (!valid_pair) {
                break;
            }

            let a = vec2<f32>(a_pt.x, a_pt.center_y);
            let b = vec2<f32>(b_pt.x, b_pt.center_y);
            let d = distance_point_to_segment(px, a, b);
            min_d = min(min_d, d);
            consumed_pair = true;

            j = j + 1u;
            if (b_pt.is_slide_end != 0u) {
                break;
            }
        }

        if (consumed_pair) {
            let outer_r = max(1.0, p0.radius_px);
            let outline_t = max(1.0, outer_r * max(globals.timeline_style.y, 0.0));
            let body_r = max(1.0, outer_r - outline_t);
            let select_t = outline_t;
            let outline_r = outer_r;
            let selected = group_selected != 0u;
            let selected_r = outline_r + select(select_t, 0.0, !selected);

            let body_cov = 1.0 - smoothstep(body_r - aa, body_r + aa, min_d);
            let outline_cov = (1.0 - smoothstep(outline_r - aa, outline_r + aa, min_d))
                * smoothstep(body_r - aa, body_r + aa, min_d);
            let select_cov = select(
                0.0,
                (1.0 - smoothstep(selected_r - aa, selected_r + aa, min_d))
                    * smoothstep(outline_r - aa, outline_r + aa, min_d),
                selected,
            );

            // Keep body shading very subtle for readability (no visible ridge banding).
            let body_shade_01 = 1.0 - 0.04 * (1.0 - saturate(min_d / max(body_r, 1e-6)));
            let body_rgb = group_color.rgb * body_shade_01;
            let body_a = body_cov * group_color.a * 0.7 * opacity;

            let outline_rgb = group_color.rgb;
            let outline_a = outline_cov * 0.9 * opacity;

            let selection_rgb = select(
                side_selection_color(1u, SC_SELECTION_BORDER).rgb,
                side_selection_color(0u, SC_SELECTION_BORDER).rgb,
                group_selected_by_left != 0u,
            );
            let selection_a = select_cov * globals.timeline_slider_outline_rgba.a * opacity;

            var group_pm = vec3<f32>(0.0);
            var group_a: f32 = 0.0;

            var tmp = over_pm(group_pm, group_a, vec4<f32>(body_rgb, body_a));
            group_pm = tmp.rgb;
            group_a = tmp.a;

            tmp = over_pm(group_pm, group_a, vec4<f32>(outline_rgb, outline_a));
            group_pm = tmp.rgb;
            group_a = tmp.a;

            tmp = over_pm(group_pm, group_a, vec4<f32>(selection_rgb, selection_a));
            group_pm = tmp.rgb;
            group_a = tmp.a;

            // Per-point markers for this slider group.
            // Render order (bottom->top): end, repeat, start.
            // This gives: start over repeats, repeats over end.
            var marker_layer: u32 = 0u;
            loop {
                if (marker_layer >= 3u) {
                    break;
                }

                var k = i;
                loop {
                    if (k > j || k >= point_end) {
                        break;
                    }

                    let pt = timeline_points[k];
                    let center = vec2<f32>(pt.x, pt.center_y);
                    let pd = length(px - center);

                    if (marker_layer == 0u && pt.is_slide_end != 0u && pt.is_slider_or_spinner != 0u) {
                        let end_r = max(1.0, outer_r * max(globals.timeline_style.w, 0.0));
                        let end_cov = 1.0 - smoothstep(end_r - aa, end_r + aa, pd);
                        let end_a = end_cov * globals.timeline_slider_end_point_rgba.a * opacity;
                        let ptmp = over_pm(
                            group_pm,
                            group_a,
                            vec4<f32>(globals.timeline_slider_end_point_rgba.rgb, end_a),
                        );
                        group_pm = ptmp.rgb;
                        group_a = ptmp.a;
                    } else if (marker_layer == 1u && pt.is_slide_repeat != 0u) {
                        let repeat_r = max(1.0, outer_r * max(globals.timeline_style.z, 0.0));
                        let repeat_cov = 1.0 - smoothstep(repeat_r - aa, repeat_r + aa, pd);
                        let repeat_a = repeat_cov * globals.timeline_slider_repeat_point_rgba.a * opacity;
                        let ptmp = over_pm(
                            group_pm,
                            group_a,
                            vec4<f32>(globals.timeline_slider_repeat_point_rgba.rgb, repeat_a),
                        );
                        group_pm = ptmp.rgb;
                        group_a = ptmp.a;
                    } else if (marker_layer == 2u && pt.is_slide_start != 0u) {
                        let start_body_cov = 1.0 - smoothstep(body_r - aa, body_r + aa, pd);
                        let start_outline_cov = (1.0 - smoothstep(outer_r - aa, outer_r + aa, pd))
                            * smoothstep(body_r - aa, body_r + aa, pd);

                        let start_body_a = start_body_cov * globals.timeline_slider_head_body_rgba.a * opacity;
                        let start_outline_a = start_outline_cov * globals.timeline_slider_head_overlay_rgba.a * opacity;

                        var ptmp = over_pm(
                            group_pm,
                            group_a,
                            vec4<f32>(globals.timeline_slider_head_body_rgba.rgb, start_body_a),
                        );
                        group_pm = ptmp.rgb;
                        group_a = ptmp.a;

                        ptmp = over_pm(
                            group_pm,
                            group_a,
                            vec4<f32>(globals.timeline_slider_head_overlay_rgba.rgb, start_outline_a),
                        );
                        group_pm = ptmp.rgb;
                        group_a = ptmp.a;
                    }

                    k = k + 1u;
                }

                marker_layer = marker_layer + 1u;
            }

            // Composite this group UNDER previously accumulated groups so smaller X
            // (earlier loop iterations) stay visually on top.
            out_pm = out_pm + group_pm * (1.0 - out_a);
            out_a = out_a + group_a * (1.0 - out_a);

            i = max(i + 1u, j);
            continue;
        }

        i = i + 1u;
    }

    let is_past_side = px.x <= globals.timeline_current_x;
    if (is_past_side && out_a > 1e-6) {
        let grayscale_strength = clamp(globals.timeline_past_grayscale_strength, 0.0, 1.0);
        if (grayscale_strength > 0.0) {
            let rgb = out_pm / out_a;
            let luma = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let gray_rgb = mix(rgb, vec3<f32>(luma), grayscale_strength);
            out_pm = gray_rgb * out_a;
        }

        let tint_alpha = clamp(globals.timeline_past_object_tint_rgba.a, 0.0, 1.0);
        if (tint_alpha > 0.0) {
            let tinted = over_pm(
                out_pm,
                out_a,
                vec4<f32>(globals.timeline_past_object_tint_rgba.rgb, tint_alpha * out_a),
            );
            out_pm = tinted.rgb;
            out_a = tinted.a;
        }
    }

    if (out_a <= 1e-6) {
        discard;
    }
    return vec4<f32>(out_pm, out_a);
}

fn timeline_window_valid() -> bool {
    return globals.timeline_window_ms.y > globals.timeline_window_ms.x + 1e-4;
}

fn timeline_time_to_top_box_x(time_ms: f32, x0: f32, x1: f32) -> f32 {
    let span = max(globals.timeline_window_ms.y - globals.timeline_window_ms.x, 1.0);
    let t = (time_ms - globals.timeline_window_ms.x) / span;
    return mix(x0, x1, t);
}

fn selection_quad_left() -> array<vec2<f32>, 4> {
    return array<vec2<f32>, 4>(
        vec2<f32>(globals.selection_quad_left_01.x, globals.selection_quad_left_01.y),
        vec2<f32>(globals.selection_quad_left_01.z, globals.selection_quad_left_01.w),
        vec2<f32>(globals.selection_quad_left_23.x, globals.selection_quad_left_23.y),
        vec2<f32>(globals.selection_quad_left_23.z, globals.selection_quad_left_23.w),
    );
}

fn selection_quad_right() -> array<vec2<f32>, 4> {
    return array<vec2<f32>, 4>(
        vec2<f32>(globals.selection_quad_right_01.x, globals.selection_quad_right_01.y),
        vec2<f32>(globals.selection_quad_right_01.z, globals.selection_quad_right_01.w),
        vec2<f32>(globals.selection_quad_right_23.x, globals.selection_quad_right_23.y),
        vec2<f32>(globals.selection_quad_right_23.z, globals.selection_quad_right_23.w),
    );
}

fn quad_aabb(quad: array<vec2<f32>, 4>) -> vec4<f32> {
    var x0 = quad[0].x;
    var y0 = quad[0].y;
    var x1 = quad[0].x;
    var y1 = quad[0].y;
    for (var i: u32 = 1u; i < 4u; i = i + 1u) {
        x0 = min(x0, quad[i].x);
        y0 = min(y0, quad[i].y);
        x1 = max(x1, quad[i].x);
        y1 = max(y1, quad[i].y);
    }
    return vec4<f32>(x0, y0, x1, y1);
}

fn point_in_quad(point: vec2<f32>, quad: array<vec2<f32>, 4>) -> bool {
    var has_pos = false;
    var has_neg = false;
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let a = quad[i];
        let b = quad[(i + 1u) % 4u];
        let ab = b - a;
        let ap = point - a;
        let cross = ab.x * ap.y - ab.y * ap.x;
        if (cross > 1e-4) {
            has_pos = true;
        } else if (cross < -1e-4) {
            has_neg = true;
        }
        if (has_pos && has_neg) {
            return false;
        }
    }
    return true;
}

fn distance_point_to_segment(point: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let ab = b - a;
    let ab_len2 = dot(ab, ab);
    if (ab_len2 <= 1e-6) {
        return length(point - a);
    }
    let t = clamp(dot(point - a, ab) / ab_len2, 0.0, 1.0);
    let closest = a + ab * t;
    return length(point - closest);
}

fn quad_edge_distance(point: vec2<f32>, quad: array<vec2<f32>, 4>) -> f32 {
    var d = distance_point_to_segment(point, quad[0], quad[1]);
    d = min(d, distance_point_to_segment(point, quad[1], quad[2]));
    d = min(d, distance_point_to_segment(point, quad[2], quad[3]));
    d = min(d, distance_point_to_segment(point, quad[3], quad[0]));
    return d;
}

fn rect_edge_distance(point: vec2<f32>, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let a = vec2<f32>(x0, y0);
    let b = vec2<f32>(x1, y0);
    let c = vec2<f32>(x1, y1);
    let d = vec2<f32>(x0, y1);
    var dist = distance_point_to_segment(point, a, b);
    dist = min(dist, distance_point_to_segment(point, b, c));
    dist = min(dist, distance_point_to_segment(point, c, d));
    dist = min(dist, distance_point_to_segment(point, d, a));
    return dist;
}

fn rect_outline_alpha(point: vec2<f32>, x0: f32, y0: f32, x1: f32, y1: f32, thickness: f32, aa: f32) -> f32 {
    let edge_dist = rect_edge_distance(point, x0, y0, x1, y1);
    return 1.0 - smoothstep(thickness - aa, thickness + aa, edge_dist);
}

fn quad_outside_border_alpha(point: vec2<f32>, quad: array<vec2<f32>, 4>, width: f32, aa: f32) -> f32 {
    let inside = point_in_quad(point, quad);
    let edge_dist = quad_edge_distance(point, quad);
    let signed_dist = select(edge_dist, -edge_dist, inside);

    let outside = smoothstep(-aa, aa, signed_dist);
    let within_width = 1.0 - smoothstep(width - aa, width + aa, signed_dist);
    return clamp(outside * within_width, 0.0, 1.0);
}

fn segment_line_alpha(point: vec2<f32>, a: vec2<f32>, b: vec2<f32>, width: f32, aa: f32) -> f32 {
    let d = distance_point_to_segment(point, a, b);
    return 1.0 - smoothstep(width - aa, width + aa, d);
}

const SC_DRAG_RECT: u32 = 0u;
const SC_SELECTION_BORDER: u32 = 1u;
const SC_SELECTION_BORDER_HOVERED: u32 = 2u;
const SC_SELECTION_BORDER_DRAGGING: u32 = 3u;
const SC_SELECTION_TINT: u32 = 4u;
const SC_SELECTION_TINT_HOVERED: u32 = 5u;
const SC_SELECTION_TINT_DRAGGING: u32 = 6u;
const SC_SELECTION_ORIGIN: u32 = 7u;
const SC_SELECTION_ORIGIN_HOVERED: u32 = 8u;
const SC_SELECTION_ORIGIN_CLICKED: u32 = 9u;
const SC_SELECTION_ORIGIN_LOCKED: u32 = 10u;
const SC_SELECTION_COMBO_COLOR: u32 = 11u;

fn side_selection_color(side: u32, index: u32) -> vec4<f32> {
    return select(globals.left_selection_colors[index], globals.right_selection_colors[index], side == 1u);
}

fn decimal_u32_x10_alpha(
    px: vec2<f32>,
    start_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    raw_x10: u32,
) -> f32 {
    var line_a: f32 = 0.0;
    var digits: array<u32, 6>;
    var count: u32 = 1u;
    let int_part = raw_x10 / 10u;
    let frac = raw_x10 % 10u;

    if (int_part >= 100000u) {
        digits[0] = (int_part / 100000u) % 10u;
        digits[1] = (int_part / 10000u) % 10u;
        digits[2] = (int_part / 1000u) % 10u;
        digits[3] = (int_part / 100u) % 10u;
        digits[4] = (int_part / 10u) % 10u;
        digits[5] = int_part % 10u;
        count = 6u;
    } else if (int_part >= 10000u) {
        digits[0] = (int_part / 10000u) % 10u;
        digits[1] = (int_part / 1000u) % 10u;
        digits[2] = (int_part / 100u) % 10u;
        digits[3] = (int_part / 10u) % 10u;
        digits[4] = int_part % 10u;
        count = 5u;
    } else if (int_part >= 1000u) {
        digits[0] = (int_part / 1000u) % 10u;
        digits[1] = (int_part / 100u) % 10u;
        digits[2] = (int_part / 10u) % 10u;
        digits[3] = int_part % 10u;
        count = 4u;
    } else if (int_part >= 100u) {
        digits[0] = (int_part / 100u) % 10u;
        digits[1] = (int_part / 10u) % 10u;
        digits[2] = int_part % 10u;
        count = 3u;
    } else if (int_part >= 10u) {
        digits[0] = (int_part / 10u) % 10u;
        digits[1] = int_part % 10u;
        count = 2u;
    } else {
        digits[0] = int_part % 10u;
    }

    var x = start_x;
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + digits[i]));
        x = x + adv;
    }
    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 46u));
    x = x + adv;
    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + frac));
    return line_a;
}

fn decimal_u32_x10_char_count(raw_x10: u32) -> u32 {
    let int_part = raw_x10 / 10u;
    if (int_part >= 100000u) {
        return 8u;
    } else if (int_part >= 10000u) {
        return 7u;
    } else if (int_part >= 1000u) {
        return 6u;
    } else if (int_part >= 100u) {
        return 5u;
    } else if (int_part >= 10u) {
        return 4u;
    }
    return 3u;
}

fn decimal_u32_x10_alpha_right(
    px: vec2<f32>,
    right_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    raw_x10: u32,
) -> f32 {
    let chars = decimal_u32_x10_char_count(raw_x10);
    let start_x = right_x - adv * f32(chars);
    return decimal_u32_x10_alpha(px, start_x, y, text_h, adv, raw_x10);
}

fn decimal_u32_x100_alpha(
    px: vec2<f32>,
    start_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    raw_x100: u32,
) -> f32 {
    var line_a: f32 = 0.0;
    let int_part = raw_x100 / 100u;
    let frac0 = (raw_x100 / 10u) % 10u;
    let frac1 = raw_x100 % 10u;

    var int_digits: array<u32, 10>;
    var int_count: u32 = 0u;
    var n = int_part;
    loop {
        int_digits[int_count] = n % 10u;
        int_count = int_count + 1u;
        if (n < 10u || int_count >= 10u) {
            break;
        }
        n = n / 10u;
    }

    var x = start_x;
    for (var i: u32 = 0u; i < int_count; i = i + 1u) {
        let d = int_digits[int_count - 1u - i];
        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + d));
        x = x + adv;
    }
    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 46u));
    x = x + adv;
    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + frac0));
    x = x + adv;
    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + frac1));
    return line_a;
}

fn decimal_u32_x100_alpha_right(
    px: vec2<f32>,
    right_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    raw_x100: u32,
) -> f32 {
    let int_part = raw_x100 / 100u;
    var int_count: u32 = 1u;
    if (int_part >= 1000000000u) {
        int_count = 10u;
    } else if (int_part >= 100000000u) {
        int_count = 9u;
    } else if (int_part >= 10000000u) {
        int_count = 8u;
    } else if (int_part >= 1000000u) {
        int_count = 7u;
    } else if (int_part >= 100000u) {
        int_count = 6u;
    } else if (int_part >= 10000u) {
        int_count = 5u;
    } else if (int_part >= 1000u) {
        int_count = 4u;
    } else if (int_part >= 100u) {
        int_count = 3u;
    } else if (int_part >= 10u) {
        int_count = 2u;
    }
    let chars = int_count + 3u;
    let start_x = right_x - adv * f32(chars);
    return decimal_u32_x100_alpha(px, start_x, y, text_h, adv, raw_x100);
}

fn decimal_i32_x10_alpha(
    px: vec2<f32>,
    start_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    raw_x10: i32,
) -> f32 {
    var line_a: f32 = 0.0;
    let neg = raw_x10 < 0;
    let abs_i = select(raw_x10, -raw_x10, neg);
    let abs_u = u32(abs_i);
    let int_part = abs_u / 10u;
    let frac = abs_u % 10u;

    var int_digits: array<u32, 10>;
    var int_count: u32 = 0u;
    var n = int_part;
    loop {
        int_digits[int_count] = n % 10u;
        int_count = int_count + 1u;
        if (n < 10u || int_count >= 10u) {
            break;
        }
        n = n / 10u;
    }

    var x = start_x;

    if (neg) {
        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 45u));
    }
    x = x + adv;
    for (var i: u32 = 0u; i < int_count; i = i + 1u) {
        let d = int_digits[int_count - 1u - i];
        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + d));
        x = x + adv;
    }
    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 46u));
    x = x + adv;
    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + frac));
    return line_a;
}

fn decimal_i32_x10_alpha_right(
    px: vec2<f32>,
    right_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    raw_x10: i32,
) -> f32 {
    let neg = raw_x10 < 0;
    let abs_i = select(raw_x10, -raw_x10, neg);
    let abs_u = u32(abs_i);
    let int_part = abs_u / 10u;
    var int_count: u32 = 1u;
    if (int_part >= 1000000000u) {
        int_count = 10u;
    } else if (int_part >= 100000000u) {
        int_count = 9u;
    } else if (int_part >= 10000000u) {
        int_count = 8u;
    } else if (int_part >= 1000000u) {
        int_count = 7u;
    } else if (int_part >= 100000u) {
        int_count = 6u;
    } else if (int_part >= 10000u) {
        int_count = 5u;
    } else if (int_part >= 1000u) {
        int_count = 4u;
    } else if (int_part >= 100u) {
        int_count = 3u;
    } else if (int_part >= 10u) {
        int_count = 2u;
    }
    let chars = int_count + 3u;
    let start_x = right_x - adv * f32(chars);
    return decimal_i32_x10_alpha(px, start_x, y, text_h, adv, raw_x10);
}

fn uint_u32_alpha_right(
    px: vec2<f32>,
    right_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    value: u32,
) -> f32 {
    var line_a: f32 = 0.0;
    var digits: array<u32, 10>;
    var count: u32 = 1u;
    var n = value;
    if (n >= 10u) {
        count = 0u;
        loop {
            digits[count] = n % 10u;
            count = count + 1u;
            if (n < 10u || count >= 10u) {
                break;
            }
            n = n / 10u;
        }
    } else {
        digits[0] = n;
    }

    let start_x = right_x - adv * f32(count);
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let d = digits[count - 1u - i];
        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(start_x + f32(i) * adv, y), text_h, 48u + d));
    }
    return line_a;
}

fn uint_u32_alpha(
    px: vec2<f32>,
    start_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    value: u32,
) -> f32 {
    var line_a: f32 = 0.0;
    var digits: array<u32, 10>;
    var count: u32 = 1u;
    var n = value;
    if (n >= 10u) {
        count = 0u;
        loop {
            digits[count] = n % 10u;
            count = count + 1u;
            if (n < 10u || count >= 10u) {
                break;
            }
            n = n / 10u;
        }
    } else {
        digits[0] = n;
    }

    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let d = digits[count - 1u - i];
        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(start_x + f32(i) * adv, y), text_h, 48u + d));
    }
    return line_a;
}

fn u32_char_count(value: u32) -> u32 {
    if (value >= 1000000000u) {
        return 10u;
    } else if (value >= 100000000u) {
        return 9u;
    } else if (value >= 10000000u) {
        return 8u;
    } else if (value >= 1000000u) {
        return 7u;
    } else if (value >= 100000u) {
        return 6u;
    } else if (value >= 10000u) {
        return 5u;
    } else if (value >= 1000u) {
        return 4u;
    } else if (value >= 100u) {
        return 3u;
    } else if (value >= 10u) {
        return 2u;
    }
    return 1u;
}

fn age_ago_alpha(
    px: vec2<f32>,
    start_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    age_value: u32,
    age_unit_is_min: bool,
) -> f32 {
    var line_a: f32 = 0.0;
    line_a = max(line_a, uint_u32_alpha(px, start_x, y, text_h, adv, age_value));

    let chars = u32_char_count(age_value);
    var x = start_x + adv * f32(chars);
    if (age_unit_is_min) {
        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 77u)); // M
    } else {
        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); // S
    }
    x = x + adv * 2.0;
    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv; // A
    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 71u)); x = x + adv; // G
    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); // O
    return line_a;
}

fn age_ago_alpha_right(
    px: vec2<f32>,
    right_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    age_value: u32,
    age_unit_is_min: bool,
) -> f32 {
    let chars = u32_char_count(age_value);
    let suffix_chars = 5u;
    let start_x = right_x - adv * f32(chars + suffix_chars);
    return age_ago_alpha(px, start_x, y, text_h, adv, age_value, age_unit_is_min);
}

fn current_state_name_char_at(index: u32) -> u32 {
    if (index < 4u) {
        return globals.current_state_name_text_0[index];
    } else if (index < 8u) {
        return globals.current_state_name_text_1[index - 4u];
    } else if (index < 12u) {
        return globals.current_state_name_text_2[index - 8u];
    } else if (index < 16u) {
        return globals.current_state_name_text_3[index - 12u];
    } else if (index < 20u) {
        return globals.current_state_name_text_4[index - 16u];
    } else if (index < 24u) {
        return globals.current_state_name_text_5[index - 20u];
    } else if (index < 28u) {
        return globals.current_state_name_text_6[index - 24u];
    }
    return globals.current_state_name_text_7[index - 28u];
}

fn unpack_ascii_char(word: u32, byte_index: u32) -> u32 {
    let shift = byte_index * 8u;
    return (word >> shift) & 0xFFu;
}

fn undo_prev_state_name_char_at(index: u32) -> u32 {
    let word = globals.undo_prev_state_name_packed[index / 4u];
    return unpack_ascii_char(word, index % 4u);
}

fn undo_next_state_name_len(index: u32) -> u32 {
    if (index < 4u) {
        return globals.undo_next_states_name_len_0[index];
    }
    return globals.undo_next_states_name_len_1[index - 4u];
}

fn undo_next_state_name_char_at(state_index: u32, char_index: u32) -> u32 {
    let packed_row = globals.undo_next_states_name_packed[state_index];
    let packed_word = packed_row[char_index / 4u];
    return unpack_ascii_char(packed_word, char_index % 4u);
}

fn packed_name_alpha(
    px: vec2<f32>,
    start_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    max_chars: u32,
    name_len: u32,
    source_kind: u32,
    source_index: u32,
) -> f32 {
    var line_a: f32 = 0.0;
    let draw_len = min(name_len, max_chars);
    var x = start_x;
    if (source_kind == 0u) {
        for (var i: u32 = 0u; i < draw_len; i = i + 1u) {
            let ch = undo_prev_state_name_char_at(i);
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, ch));
            x = x + adv;
        }
    } else {
        for (var i: u32 = 0u; i < draw_len; i = i + 1u) {
            let ch = undo_next_state_name_char_at(source_index, i);
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, ch));
            x = x + adv;
        }
    }
    return line_a;
}

fn current_state_name_alpha(
    px: vec2<f32>,
    start_x: f32,
    y: f32,
    text_h: f32,
    adv: f32,
    max_chars: u32,
) -> f32 {
    var line_a: f32 = 0.0;
    let name_len = min(globals.current_state_name_meta.x, max_chars);
    var x = start_x;
    for (var i: u32 = 0u; i < name_len; i = i + 1u) {
        let ch = current_state_name_char_at(i);
        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, ch));
        x = x + adv;
    }

    let text_mode = globals.current_state_name_meta.y != 0u;
    let blink_phase = u32(floor(globals.time_elapsed_ms / 500.0)) % 2u;
    let cursor_visible = blink_phase == 0u;
    if (text_mode && cursor_visible && name_len < max_chars) {
        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 124u));
    }
    return line_a;
}

@fragment
fn fs_hud(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let res = globals.screen_size;
    let px = uv * res;

    let opacity = clamp(globals.hud_opacity, 0.0, 1.0);
    if (opacity <= 1e-6) {
        discard;
    }

    let bar_x0 = globals.timeline_rect.x;
    let bar_y0 = globals.timeline_rect.y;
    let bar_x1 = globals.timeline_rect.z;
    let bar_y1 = globals.timeline_rect.w;
    let hitbox_x0 = globals.timeline_hitbox_rect.x;
    let hitbox_y0 = globals.timeline_hitbox_rect.y;
    let hitbox_x1 = globals.timeline_hitbox_rect.z;
    let hitbox_y1 = globals.timeline_hitbox_rect.w;
    let play_pause_x0 = globals.play_pause_button_rect.x;
    let play_pause_y0 = globals.play_pause_button_rect.y;
    let play_pause_x1 = globals.play_pause_button_rect.z;
    let play_pause_y1 = globals.play_pause_button_rect.w;
    let top_bar0_x0 = globals.top_timeline_rect.x;
    let top_bar0_y0 = globals.top_timeline_rect.y;
    let top_bar0_x1 = globals.top_timeline_rect.z;
    let top_bar0_y1 = globals.top_timeline_rect.w;
    let top_hitbox0_x0 = globals.top_timeline_hitbox_rect.x;
    let top_hitbox0_y0 = globals.top_timeline_hitbox_rect.y;
    let top_hitbox0_x1 = globals.top_timeline_hitbox_rect.z;
    let top_hitbox0_y1 = globals.top_timeline_hitbox_rect.w;
    let top_bar1_x0 = globals.top_timeline_second_rect.x;
    let top_bar1_y0 = globals.top_timeline_second_rect.y;
    let top_bar1_x1 = globals.top_timeline_second_rect.z;
    let top_bar1_y1 = globals.top_timeline_second_rect.w;
    let top_hitbox1_x0 = globals.top_timeline_second_hitbox_rect.x;
    let top_hitbox1_y0 = globals.top_timeline_second_hitbox_rect.y;
    let top_hitbox1_x1 = globals.top_timeline_second_hitbox_rect.z;
    let top_hitbox1_y1 = globals.top_timeline_second_hitbox_rect.w;
    let top_bar2_x0 = globals.top_timeline_third_rect.x;
    let top_bar2_y0 = globals.top_timeline_third_rect.y;
    let top_bar2_x1 = globals.top_timeline_third_rect.z;
    let top_bar2_y1 = globals.top_timeline_third_rect.w;
    let top_hitbox2_x0 = globals.top_timeline_third_hitbox_rect.x;
    let top_hitbox2_y0 = globals.top_timeline_third_hitbox_rect.y;
    let top_hitbox2_x1 = globals.top_timeline_third_hitbox_rect.z;
    let top_hitbox2_y1 = globals.top_timeline_third_hitbox_rect.w;

    let text_h = 18.0;
    let text_gap = 6.0;
    let text_y = bar_y0 - text_gap - text_h;

    var out_pm = vec3<f32>(0.0);
    var out_a: f32 = 0.0;

    let total = max(globals.song_total_ms, 0.0);
    let fill_x = timeline_fill_x(total, bar_x0, bar_x1);
    let timeline_rgb = vec3<f32>(1.0);
    let perf_margin = 12.0;
    let perf_box_h = 85.0;
    let perf_text_h = 14.0;
    let perf_adv = (perf_text_h / 7.0) * 6.0;
    let perf_side_padding = 8.0;
    let perf_label_chars = 12.0;
    let perf_value_chars = 10.0;
    let perf_column_gap_chars = 1.0;
    let perf_box_w = perf_side_padding * 2.0 + perf_adv * (perf_label_chars + perf_column_gap_chars + perf_value_chars);
    let perf_box_x1 = res.x - perf_margin;
    let perf_box_x0 = perf_box_x1 - perf_box_w;
    let perf_box_y1 = max(perf_margin + perf_box_h, bar_y0 - perf_margin);
    let perf_box_y0 = perf_box_y1 - perf_box_h;
    let px_in_perf_box = px.x >= perf_box_x0 && px.x <= perf_box_x1 && px.y >= perf_box_y0 && px.y <= perf_box_y1;

    // --- Top-left stats box (skin-independent glyphs) ---
    {
        let box = globals.stats_box_rect;
        let box_x0 = box.x;
        let box_y0 = box.y;
        let box_x1 = box.z;
        let box_y1 = box.w;

        if (px.x >= box_x0 && px.x <= box_x1 && px.y >= box_y0 && px.y <= box_y1) {
            let border = 1.0;
            let on_border =
                px.x <= box_x0 + border ||
                px.x >= box_x1 - border ||
                px.y <= box_y0 + border ||
                px.y >= box_y1 - border;

            let bg = vec4<f32>(vec3<f32>(0.0), 0.6);
            let border_col = vec4<f32>(vec3<f32>(1.0), 0.9);
            let panel = select(bg, border_col, on_border);
            let panel_blend = over_pm(out_pm, out_a, panel);
            out_pm = panel_blend.rgb;
            out_a = panel_blend.a;

            let text_h = 14.0;
            let adv = (text_h / 7.0) * 6.0;
            let text_x = box_x0 + 8.0;
            let value_x = box_x0 + 128.0;
            let value_right_x = box_x1 - 8.0;
            let line_step = text_h + 4.0;
            let text_color = vec4<f32>(vec3<f32>(1.0), 0.95);

            let rate = clamp(globals.playback_rate, 0.0, 99.99);
            let rate100 = u32(round(rate * 100.0));
            let rate_i = rate100 / 100u;
            let rate_f0 = (rate100 / 10u) % 10u;
            let rate_f1 = rate100 % 10u;
            let time_ms_u = u32(max(globals.time_ms, 0.0));
            let vol_audio = clamp(globals.audio_volume, 0.0, 1.0);
            let vol_hs = clamp(globals.hitsound_volume, 0.0, 1.0);
            let pf_w = max(globals.playfield_rect.z - globals.playfield_rect.x, 1e-6);
            let pf_h = max(globals.playfield_rect.w - globals.playfield_rect.y, 1e-6);
            let cursor_game_x = (globals.cursor_pos.x - globals.playfield_rect.x) * (512.0 / pf_w);
            let cursor_game_y = (globals.cursor_pos.y - globals.playfield_rect.y) * (384.0 / pf_h);

            // Line 5: VOL_AUDIO
            {
                let y = box_y0 + 8.0 + line_step * 3.0;
                var line_a: f32 = 0.0;
                var x = text_x;

                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 86u)); x = x + adv; // V
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv; // O
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv; // L
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 95u)); x = x + adv; // _
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv; // A
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 85u)); x = x + adv; // U
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 68u)); x = x + adv; // D
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 73u)); x = x + adv; // I
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); // O

                let vol100 = u32(round(vol_audio * 100.0));
                let int_part = vol100 / 100u;
                let frac0 = (vol100 / 10u) % 10u;
                let frac1 = vol100 % 10u;

                var int_digits: array<u32, 10>;
                var int_count: u32 = 0u;
                var n = int_part;
                loop {
                    int_digits[int_count] = n % 10u;
                    int_count = int_count + 1u;
                    if (n < 10u || int_count >= 10u) {
                        break;
                    }
                    n = n / 10u;
                }

                x = value_x;
                for (var i: u32 = 0u; i < int_count; i = i + 1u) {
                    let d = int_digits[int_count - 1u - i];
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + d));
                    x = x + adv;
                }
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 46u)); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + frac0)); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + frac1));

                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

            // Line 6: VOL_HS
            {
                let y = box_y0 + 8.0 + line_step * 4.0;
                var line_a: f32 = 0.0;
                var x = text_x;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 86u)); x = x + adv; // V
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv; // O
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv; // L
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 95u)); x = x + adv; // _
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 72u)); x = x + adv; // H
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); // S

                let vol100 = u32(round(vol_hs * 100.0));
                let int_part = vol100 / 100u;
                let frac0 = (vol100 / 10u) % 10u;
                let frac1 = vol100 % 10u;

                var int_digits: array<u32, 10>;
                var int_count: u32 = 0u;
                var n = int_part;
                loop {
                    int_digits[int_count] = n % 10u;
                    int_count = int_count + 1u;
                    if (n < 10u || int_count >= 10u) {
                        break;
                    }
                    n = n / 10u;
                }

                x = value_x;
                for (var i: u32 = 0u; i < int_count; i = i + 1u) {
                    let d = int_digits[int_count - 1u - i];
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + d));
                    x = x + adv;
                }
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 46u)); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + frac0)); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + frac1));

                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

            // Line 2: RATE
            {
                let y = box_y0 + 8.0 + line_step * 1.0;
                var line_a: f32 = 0.0;
                var x = text_x;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 82u)); x = x + adv; // R
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv; // A
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 84u)); x = x + adv; // T
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u)); // E

                x = value_x;
                if (rate_i >= 10u) {
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + ((rate_i / 10u) % 10u))); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + (rate_i % 10u))); x = x + adv;
                } else {
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + (rate_i % 10u))); x = x + adv;
                }
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 46u)); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + rate_f0)); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + rate_f1)); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 88u)); // X

                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

            // Line 7: X
            {
                let y = box_y0 + 8.0 + line_step * 6.0;
                var line_a: f32 = 0.0;
                var x = text_x;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 88u)); // X

                let cursor_x_i = i32(round(cursor_game_x));
                let neg = cursor_x_i < 0;
                let abs_u = u32(select(cursor_x_i, -cursor_x_i, neg));
                var rev_digits: array<u32, 10>;
                var rev_count: u32 = 0u;
                var n = abs_u;
                loop {
                    rev_digits[rev_count] = n % 10u;
                    rev_count = rev_count + 1u;
                    if (n < 10u || rev_count >= 10u) {
                        break;
                    }
                    n = n / 10u;
                }

                x = value_x - adv;
                if (neg) {
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 45u));
                }
                x = x + adv;
                for (var i: u32 = 0u; i < rev_count; i = i + 1u) {
                    let d = rev_digits[rev_count - 1u - i];
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + d));
                    x = x + adv;
                }

                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

            // Line 8: Y
            {
                let y = box_y0 + 8.0 + line_step * 7.0;
                var line_a: f32 = 0.0;
                var x = text_x;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 89u)); // Y

                let cursor_y_i = i32(round(cursor_game_y));
                let neg = cursor_y_i < 0;
                let abs_u = u32(select(cursor_y_i, -cursor_y_i, neg));
                var rev_digits: array<u32, 10>;
                var rev_count: u32 = 0u;
                var n = abs_u;
                loop {
                    rev_digits[rev_count] = n % 10u;
                    rev_count = rev_count + 1u;
                    if (n < 10u || rev_count >= 10u) {
                        break;
                    }
                    n = n / 10u;
                }

                x = value_x - adv;
                if (neg) {
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 45u));
                }
                x = x + adv;
                for (var i: u32 = 0u; i < rev_count; i = i + 1u) {
                    let d = rev_digits[rev_count - 1u - i];
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + d));
                    x = x + adv;
                }

                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

            // Line 1: TIME
            {
                let y = box_y0 + 8.0;
                var line_a: f32 = 0.0;
                var x = text_x;

                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 84u)); x = x + adv; // T
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 73u)); x = x + adv; // I
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 77u)); x = x + adv; // M
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u)); // E

                let minutes = (time_ms_u / 60000u) % 100u;
                let seconds = (time_ms_u / 1000u) % 60u;
                let millis = time_ms_u % 1000u;

                x = value_x;
                if (minutes >= 10u) {
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + ((minutes / 10u) % 10u))); x = x + adv;
                }
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + (minutes % 10u))); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 58u)); x = x + adv; // :
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + ((seconds / 10u) % 10u))); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + (seconds % 10u))); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 46u)); x = x + adv; // .
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + ((millis / 100u) % 10u))); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + ((millis / 10u) % 10u))); x = x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + (millis % 10u)));

                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }
        }
    }

    // --- Top-left volume boxes ---
    {
        let parent = globals.stats_box_rect;
        let gap = 8.0;
        let box_w = 236.0;
        let box_h = 28.0;
        let text_h = 14.0;
        let adv = (text_h / 7.0) * 6.0;

        let audio_x0 = parent.z + gap;
        let audio_y0 = parent.y;
        let audio_x1 = audio_x0 + box_w;
        let audio_y1 = audio_y0 + box_h;

        let hs_x0 = audio_x0;
        let hs_y0 = audio_y1 + gap;
        let hs_x1 = audio_x1;
        let hs_y1 = hs_y0 + box_h;

        let scale_x0 = audio_x0;
        let scale_y0 = hs_y1 + gap;
        let scale_x1 = audio_x1;
        let scale_y1 = scale_y0 + box_h;

        let zoom_x0 = audio_x0;
        let zoom_y0 = scale_y1 + gap;
        let zoom_x1 = audio_x1;
        let zoom_y1 = zoom_y0 + box_h;

        let cursor = globals.cursor_pos;
        let audio_hovered = cursor.x >= audio_x0 && cursor.x <= audio_x1 && cursor.y >= audio_y0 && cursor.y <= audio_y1;
        let hs_hovered = cursor.x >= hs_x0 && cursor.x <= hs_x1 && cursor.y >= hs_y0 && cursor.y <= hs_y1;
        let scale_hovered = cursor.x >= scale_x0 && cursor.x <= scale_x1 && cursor.y >= scale_y0 && cursor.y <= scale_y1;
        let zoom_hovered = cursor.x >= zoom_x0 && cursor.x <= zoom_x1 && cursor.y >= zoom_y0 && cursor.y <= zoom_y1;

        let max_fit = min(globals.screen_size.x / 640.0, globals.screen_size.y / 480.0);
        let pf_w = max(globals.playfield_rect.z - globals.playfield_rect.x, 1e-6);
        let scale_from_playfield = pf_w / max(512.0 * max_fit, 1e-6);
        let playfield_scale = clamp(scale_from_playfield, 0.01, 1.0);

        // AUDIO box
        if (px.x >= audio_x0 && px.x <= audio_x1 && px.y >= audio_y0 && px.y <= audio_y1) {
            let border = 1.0;
            let on_border =
                px.x <= audio_x0 + border ||
                px.x >= audio_x1 - border ||
                px.y <= audio_y0 + border ||
                px.y >= audio_y1 - border;

            let bg_a = select(0.60, 0.72, audio_hovered);
            let border_a = select(0.90, 1.00, audio_hovered);
            let fill_a = select(0.20, 0.30, audio_hovered);
            let panel = select(vec4<f32>(vec3<f32>(0.0), bg_a), vec4<f32>(vec3<f32>(1.0), border_a), on_border);
            let panel_blend = over_pm(out_pm, out_a, panel);
            out_pm = panel_blend.rgb;
            out_a = panel_blend.a;

            let vol = clamp(globals.audio_volume, 0.0, 1.0);
            let fill_x = audio_x0 + 1.0 + (audio_x1 - audio_x0 - 2.0) * vol;
            if (px.x >= audio_x0 + 1.0 && px.x <= fill_x && px.y >= audio_y0 + 1.0 && px.y <= audio_y1 - 1.0) {
                let fill = vec4<f32>(vec3<f32>(1.0), fill_a);
                let t = over_pm(out_pm, out_a, fill);
                out_pm = t.rgb;
                out_a = t.a;
            }

            let text_color = vec4<f32>(vec3<f32>(1.0), 0.95);
            let y = audio_y0 + 7.0;
            var line_a: f32 = 0.0;
            var x = audio_x0 + 8.0;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv; // A
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 85u)); x = x + adv; // U
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 68u)); x = x + adv; // D
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 73u)); x = x + adv; // I
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv; // O
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 58u)); // :

            let pct = u32(round(vol * 100.0));
            var digits: array<u32, 3>;
            var count: u32 = 1u;
            if (pct >= 100u) {
                digits[0] = 1u;
                digits[1] = 0u;
                digits[2] = 0u;
                count = 3u;
            } else if (pct >= 10u) {
                digits[0] = (pct / 10u) % 10u;
                digits[1] = pct % 10u;
                count = 2u;
            } else {
                digits[0] = pct;
            }

            let value_right_x = audio_x1 - 8.0;
            var value_x = value_right_x - adv * f32(count + 1u);
            for (var i: u32 = 0u; i < count; i = i + 1u) {
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(value_x + f32(i) * adv, y), text_h, 48u + digits[i]));
            }
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(value_right_x - adv, y), text_h, 37u)); // %

            if (line_a > 0.0) {
                let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                out_pm = t.rgb;
                out_a = t.a;
            }
        }

        // HITSOUNDS box
        if (px.x >= hs_x0 && px.x <= hs_x1 && px.y >= hs_y0 && px.y <= hs_y1) {
            let border = 1.0;
            let on_border =
                px.x <= hs_x0 + border ||
                px.x >= hs_x1 - border ||
                px.y <= hs_y0 + border ||
                px.y >= hs_y1 - border;

            let bg_a = select(0.60, 0.72, hs_hovered);
            let border_a = select(0.90, 1.00, hs_hovered);
            let fill_a = select(0.20, 0.30, hs_hovered);
            let panel = select(vec4<f32>(vec3<f32>(0.0), bg_a), vec4<f32>(vec3<f32>(1.0), border_a), on_border);
            let panel_blend = over_pm(out_pm, out_a, panel);
            out_pm = panel_blend.rgb;
            out_a = panel_blend.a;

            let vol = clamp(globals.hitsound_volume, 0.0, 1.0);
            let fill_x = hs_x0 + 1.0 + (hs_x1 - hs_x0 - 2.0) * vol;
            if (px.x >= hs_x0 + 1.0 && px.x <= fill_x && px.y >= hs_y0 + 1.0 && px.y <= hs_y1 - 1.0) {
                let fill = vec4<f32>(vec3<f32>(1.0), fill_a);
                let t = over_pm(out_pm, out_a, fill);
                out_pm = t.rgb;
                out_a = t.a;
            }

            let text_color = vec4<f32>(vec3<f32>(1.0), 0.95);
            let y = hs_y0 + 7.0;
            var line_a: f32 = 0.0;
            var x = hs_x0 + 8.0;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 72u)); x = x + adv; // H
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 73u)); x = x + adv; // I
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 84u)); x = x + adv; // T
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv; // S
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv; // O
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 85u)); x = x + adv; // U
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 78u)); x = x + adv; // N
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 68u)); x = x + adv; // D
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv; // S
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 58u)); // :

            let pct = u32(round(vol * 100.0));
            var digits: array<u32, 3>;
            var count: u32 = 1u;
            if (pct >= 100u) {
                digits[0] = 1u;
                digits[1] = 0u;
                digits[2] = 0u;
                count = 3u;
            } else if (pct >= 10u) {
                digits[0] = (pct / 10u) % 10u;
                digits[1] = pct % 10u;
                count = 2u;
            } else {
                digits[0] = pct;
            }

            let value_right_x = hs_x1 - 8.0;
            var value_x = value_right_x - adv * f32(count + 1u);
            for (var i: u32 = 0u; i < count; i = i + 1u) {
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(value_x + f32(i) * adv, y), text_h, 48u + digits[i]));
            }
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(value_right_x - adv, y), text_h, 37u)); // %

            if (line_a > 0.0) {
                let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                out_pm = t.rgb;
                out_a = t.a;
            }
        }

        // PLAYFIELD SCALE box
        if (px.x >= scale_x0 && px.x <= scale_x1 && px.y >= scale_y0 && px.y <= scale_y1) {
            let border = 1.0;
            let on_border =
                px.x <= scale_x0 + border ||
                px.x >= scale_x1 - border ||
                px.y <= scale_y0 + border ||
                px.y >= scale_y1 - border;

            let bg_a = select(0.60, 0.72, scale_hovered);
            let border_a = select(0.90, 1.00, scale_hovered);
            let fill_a = select(0.20, 0.30, scale_hovered);
            let panel = select(vec4<f32>(vec3<f32>(0.0), bg_a), vec4<f32>(vec3<f32>(1.0), border_a), on_border);
            let panel_blend = over_pm(out_pm, out_a, panel);
            out_pm = panel_blend.rgb;
            out_a = panel_blend.a;

            let fill_x = scale_x0 + 1.0 + (scale_x1 - scale_x0 - 2.0) * playfield_scale;
            if (px.x >= scale_x0 + 1.0 && px.x <= fill_x && px.y >= scale_y0 + 1.0 && px.y <= scale_y1 - 1.0) {
                let fill = vec4<f32>(vec3<f32>(1.0), fill_a);
                let t = over_pm(out_pm, out_a, fill);
                out_pm = t.rgb;
                out_a = t.a;
            }

            let text_color = vec4<f32>(vec3<f32>(1.0), 0.95);
            let y = scale_y0 + 7.0;
            var line_a: f32 = 0.0;
            var x = scale_x0 + 8.0;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv; // S
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 67u)); x = x + adv; // C
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv; // A
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv; // L
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u)); x = x + adv; // E
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 58u)); // :

            let pct = u32(round(playfield_scale * 100.0));
            var digits: array<u32, 3>;
            var count: u32 = 1u;
            if (pct >= 100u) {
                digits[0] = 1u;
                digits[1] = 0u;
                digits[2] = 0u;
                count = 3u;
            } else if (pct >= 10u) {
                digits[0] = (pct / 10u) % 10u;
                digits[1] = pct % 10u;
                count = 2u;
            } else {
                digits[0] = pct;
            }

            let value_right_x = scale_x1 - 8.0;
            var value_x = value_right_x - adv * f32(count + 1u);
            for (var i: u32 = 0u; i < count; i = i + 1u) {
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(value_x + f32(i) * adv, y), text_h, 48u + digits[i]));
            }
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(value_right_x - adv, y), text_h, 37u)); // %

            if (line_a > 0.0) {
                let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                out_pm = t.rgb;
                out_a = t.a;
            }
        }

        // TIMELINE ZOOM box
        if (px.x >= zoom_x0 && px.x <= zoom_x1 && px.y >= zoom_y0 && px.y <= zoom_y1) {
            let border = 1.0;
            let on_border =
                px.x <= zoom_x0 + border ||
                px.x >= zoom_x1 - border ||
                px.y <= zoom_y0 + border ||
                px.y >= zoom_y1 - border;

            let bg_a = select(0.60, 0.72, zoom_hovered);
            let border_a = select(0.90, 1.00, zoom_hovered);
            let fill_a = select(0.20, 0.30, zoom_hovered);
            let panel = select(vec4<f32>(vec3<f32>(0.0), bg_a), vec4<f32>(vec3<f32>(1.0), border_a), on_border);
            let panel_blend = over_pm(out_pm, out_a, panel);
            out_pm = panel_blend.rgb;
            out_a = panel_blend.a;

            let zoom = clamp(globals.timeline_zoom, 0.1, 10.0);
            let zoom_norm = clamp((log(zoom) / log(10.0) + 1.0) * 0.5, 0.0, 1.0);
            let fill_x = zoom_x0 + 1.0 + (zoom_x1 - zoom_x0 - 2.0) * zoom_norm;
            if (px.x >= zoom_x0 + 1.0 && px.x <= fill_x && px.y >= zoom_y0 + 1.0 && px.y <= zoom_y1 - 1.0) {
                let fill = vec4<f32>(vec3<f32>(1.0), fill_a);
                let t = over_pm(out_pm, out_a, fill);
                out_pm = t.rgb;
                out_a = t.a;
            }

            let text_color = vec4<f32>(vec3<f32>(1.0), 0.95);
            let y = zoom_y0 + 7.0;
            var line_a: f32 = 0.0;
            var x = zoom_x0 + 8.0;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 84u)); x = x + adv; // T
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv; // L
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 95u)); x = x + adv; // _
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 90u)); x = x + adv; // Z
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv; // O
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv; // O
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 77u)); x = x + adv; // M
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 58u)); // :

            let zoom_x100 = u32(round(zoom * 100.0));
            let value_right_x = zoom_x1 - 8.0;
            line_a = max(line_a, decimal_u32_x100_alpha_right(px, value_right_x - adv, y, text_h, adv, zoom_x100));
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(value_right_x - adv, y), text_h, 88u)); // X

            if (line_a > 0.0) {
                let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                out_pm = t.rgb;
                out_a = t.a;
            }
        }
    }

    // --- Bottom-right performance box ---
    {
        let box_h = perf_box_h;
        let text_h = 14.0;
        let adv = (text_h / 7.0) * 6.0;
        let box_x0 = perf_box_x0;
        let box_x1 = perf_box_x1;
        let box_y0 = perf_box_y0;
        let box_y1 = perf_box_y1;

        if (px.x >= box_x0 && px.x <= box_x1 && px.y >= box_y0 && px.y <= box_y1) {
            let border = 1.0;
            let on_border =
                px.x <= box_x0 + border ||
                px.x >= box_x1 - border ||
                px.y <= box_y0 + border ||
                px.y >= box_y1 - border;

            let bg = vec4<f32>(vec3<f32>(0.0), 0.6);
            let border_col = vec4<f32>(vec3<f32>(1.0), 0.9);
            let panel = select(bg, border_col, on_border);
            let panel_blend = over_pm(out_pm, out_a, panel);
            out_pm = panel_blend.rgb;
            out_a = panel_blend.a;

            let text_x = box_x0 + 8.0;
            let value_right_x = box_x1 - 8.0;
            let line_step = text_h + 4.0;
            let text_color = vec4<f32>(vec3<f32>(1.0), 0.95);

            // Line 1: FPS
            {
                let y = box_y0 + 8.0 + line_step * 0.0;
                var line_a: f32 = 0.0;
                var x = text_x;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 70u)); x = x + adv; // F
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 80u)); x = x + adv; // P
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); // S
                line_a = max(line_a, decimal_u32_x10_alpha_right(px, value_right_x, y, text_h, adv, globals.fps_x10));
                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

            // Line 2: FPS_LOW
            {
                let y = box_y0 + 8.0 + line_step * 1.0;
                var line_a: f32 = 0.0;
                var x = text_x;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 70u)); x = x + adv; // F
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 80u)); x = x + adv; // P
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv; // S
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 95u)); x = x + adv; // _
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv; // L
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv; // O
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 87u)); // W
                line_a = max(line_a, decimal_u32_x10_alpha_right(px, value_right_x, y, text_h, adv, globals.fps_low_x10));
                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

            // Line 3: CPU_PASS
            {
                let y = box_y0 + 8.0 + line_step * 2.0;
                var line_a: f32 = 0.0;
                var x = text_x;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 67u)); x = x + adv; // C
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 80u)); x = x + adv; // P
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 85u)); x = x + adv; // U
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 95u)); x = x + adv; // _
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 80u)); x = x + adv; // P
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv; // A
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv; // S
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); // S
                line_a = max(line_a, decimal_u32_x10_alpha_right(px, value_right_x, y, text_h, adv, globals.cpu_pass_x10));
                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

            // Line 4: GPU_PASS
            {
                let y = box_y0 + 8.0 + line_step * 3.0;
                var line_a: f32 = 0.0;
                var x = text_x;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 71u)); x = x + adv; // G
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 80u)); x = x + adv; // P
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 85u)); x = x + adv; // U
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 95u)); x = x + adv; // _
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 80u)); x = x + adv; // P
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv; // A
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv; // S
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); // S
                line_a = max(line_a, decimal_u32_x10_alpha_right(px, value_right_x, y, text_h, adv, globals.gpu_pass_x10));
                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

        }
    }

    // --- Top-right status box ---
    {
        let margin = 8.0;
        let box_h = 48.0;
        let text_h = 14.0;
        let adv = (text_h / 7.0) * 6.0;
        let side_padding = 8.0;
        let label_chars = 12.0;
        let value_chars = 10.0;
        let column_gap_chars = 1.0;
        let box_w = side_padding * 2.0 + adv * (label_chars + column_gap_chars + value_chars);
        let box_x1 = res.x - margin;
        let box_x0 = box_x1 - box_w;
        let box_y0 = top_bar0_y1 + margin;
        let box_y1 = box_y0 + box_h;

        if (px.x >= box_x0 && px.x <= box_x1 && px.y >= box_y0 && px.y <= box_y1) {
            let border = 1.0;
            let on_border =
                px.x <= box_x0 + border ||
                px.x >= box_x1 - border ||
                px.y <= box_y0 + border ||
                px.y >= box_y1 - border;

            let bg = vec4<f32>(vec3<f32>(0.0), 0.6);
            let border_col = vec4<f32>(vec3<f32>(1.0), 0.9);
            let panel = select(bg, border_col, on_border);
            let panel_blend = over_pm(out_pm, out_a, panel);
            out_pm = panel_blend.rgb;
            out_a = panel_blend.a;

            let text_x = box_x0 + 8.0;
            let value_right_x = box_x1 - 8.0;
            let line_step = text_h + 4.0;
            let text_color = vec4<f32>(vec3<f32>(1.0), 0.95);
            let time_elapsed_total_ms = u32(max(globals.time_elapsed_ms, 0.0));

            // Line 1: TIME_ELAPSED
            {
                let y = box_y0 + 8.0 + line_step * 0.0;
                var line_a: f32 = 0.0;
                var x = text_x;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 84u)); x = x + adv; // T
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 73u)); x = x + adv; // I
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 77u)); x = x + adv; // M
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u)); x = x + adv; // E
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 95u)); x = x + adv; // _
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u)); x = x + adv; // E
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv; // L
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv; // A
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 80u)); x = x + adv; // P
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv; // S
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u)); x = x + adv; // E
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 68u)); // D

                let elapsed_minutes = time_elapsed_total_ms / 60000u;
                let elapsed_seconds = (time_elapsed_total_ms / 1000u) % 60u;

                var minute_digits_rev: array<u32, 10>;
                var minute_count: u32 = 1u;
                var minute_n = elapsed_minutes;
                if (minute_n >= 10u) {
                    minute_count = 0u;
                    loop {
                        minute_digits_rev[minute_count] = minute_n % 10u;
                        minute_count = minute_count + 1u;
                        if (minute_n < 10u || minute_count >= 10u) {
                            break;
                        }
                        minute_n = minute_n / 10u;
                    }
                } else {
                    minute_digits_rev[0] = minute_n;
                }

                let chars = minute_count + 3u;
                let start_x = value_right_x - adv * f32(chars);
                var value_x = start_x;
                for (var i: u32 = 0u; i < minute_count; i = i + 1u) {
                    let d = minute_digits_rev[minute_count - 1u - i];
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(value_x, y), text_h, 48u + d));
                    value_x = value_x + adv;
                }
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(value_x, y), text_h, 58u)); value_x = value_x + adv; // :
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(value_x, y), text_h, 48u + ((elapsed_seconds / 10u) % 10u))); value_x = value_x + adv;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(value_x, y), text_h, 48u + (elapsed_seconds % 10u)));
                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

            // Line 2: UNDOS_AVAIL
            {
                let y = box_y0 + 8.0 + line_step * 1.0;
                var line_a: f32 = 0.0;
                var x = text_x;
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 85u)); x = x + adv; // U
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 78u)); x = x + adv; // N
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 68u)); x = x + adv; // D
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv; // O
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv; // S
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 95u)); x = x + adv; // _
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv; // A
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 86u)); x = x + adv; // V
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv; // A
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 73u)); x = x + adv; // I
                line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); // L

                var digits: array<u32, 10>;
                var count: u32 = 1u;
                var n = globals.undo_count;
                if (n >= 10u) {
                    count = 0u;
                    loop {
                        digits[count] = n % 10u;
                        count = count + 1u;
                        if (n < 10u || count >= 10u) {
                            break;
                        }
                        n = n / 10u;
                    }
                } else {
                    digits[0] = n;
                }

                let start_x = value_right_x - adv * f32(count);
                for (var i: u32 = 0u; i < count; i = i + 1u) {
                    let d = digits[count - 1u - i];
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(start_x + f32(i) * adv, y), text_h, 48u + d));
                }
                if (line_a > 0.0) {
                    let tmp = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }
        }
    }

    // --- Top-right undo/current/redo buttons (below status) ---
    {
        let margin = 8.0;
        let outer_gap = 8.0;
        let prev_box_h = 48.0;
        let text_h = 14.0;
        let adv = (text_h / 7.0) * 6.0;
        let button_h = 30.0;
        let button_gap = 8.0;

        let next_count_clamped = min(globals.undo_redo_info.z, 8u);

        let side_padding = 8.0;
        let label_chars = 12.0;
        let value_chars = 10.0;
        let column_gap_chars = 1.0;
        let box_w = side_padding * 2.0 + adv * (label_chars + column_gap_chars + value_chars);
        let box_x1 = res.x - margin;
        let box_x0 = box_x1 - box_w;
        let button_x0 = box_x0;
        let button_x1 = box_x1;
        let top_y0 = top_bar0_y1 + margin + prev_box_h + outer_gap;

        // Undo button (clickable)
        {
            let row_y0 = top_y0;
            let row_y1 = row_y0 + button_h;
            let undo_available = globals.undo_prev_state_info.w != 0u;
            let in_row = px.x >= button_x0 && px.x <= button_x1 && px.y >= row_y0 && px.y <= row_y1;
            if (in_row) {
                let hovered = undo_available && globals.undo_button_meta.x != 0u;
                let clicked = undo_available && globals.undo_button_meta.y != 0u;
                let on_row_border =
                    px.x <= button_x0 + 1.0 ||
                    px.x >= button_x1 - 1.0 ||
                    px.y <= row_y0 + 1.0 ||
                    px.y >= row_y1 - 1.0;
                var row_fill = vec4<f32>(vec3<f32>(0.0), 0.45);
                row_fill = select(row_fill, vec4<f32>(vec3<f32>(1.0), 0.28), hovered);
                row_fill = select(row_fill, vec4<f32>(vec3<f32>(1.0), 0.45), clicked);
                row_fill = select(row_fill, vec4<f32>(vec3<f32>(0.42), 0.35), !undo_available);
                var row_border = vec4<f32>(vec3<f32>(1.0), 0.8);
                row_border = select(row_border, vec4<f32>(vec3<f32>(1.0), 1.0), hovered || clicked);
                row_border = select(row_border, vec4<f32>(vec3<f32>(0.65), 0.85), !undo_available);
                let row_col = select(row_fill, row_border, on_row_border);
                let row_blend = over_pm(out_pm, out_a, row_col);
                out_pm = row_blend.rgb;
                out_a = row_blend.a;
            }

            let y = row_y0 + 7.0;
            var line_a: f32 = 0.0;

            let undo_label_x = (button_x0 + button_x1 - adv * 4.0) * 0.5;
            var undo_x = undo_label_x;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(undo_x, y), text_h, 85u)); undo_x = undo_x + adv;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(undo_x, y), text_h, 78u)); undo_x = undo_x + adv;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(undo_x, y), text_h, 68u)); undo_x = undo_x + adv;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(undo_x, y), text_h, 79u));

            if (globals.undo_prev_state_info.w != 0u) {
                let max_chars = u32(max(floor((button_x1 - button_x0 - 16.0) / adv), 0.0));
                let age_chars = u32_char_count(globals.undo_prev_state_info.y) + 5u;
                let reserved_chars = age_chars + 1u;
                let name_max_chars = select(0u, max_chars - reserved_chars, max_chars > reserved_chars);
                let name_len = globals.undo_prev_state_name_meta.x;
                if (name_len > 0u) {
                    line_a = max(
                        line_a,
                        packed_name_alpha(px, button_x0 + 8.0, y, text_h, adv, name_max_chars, name_len, 0u, 0u),
                    );
                } else {
                    line_a = max(line_a, uint_u32_alpha(px, button_x0 + 8.0, y, text_h, adv, globals.undo_prev_state_info.x));
                }
                line_a = max(line_a, age_ago_alpha_right(px, button_x1 - 8.0, y, text_h, adv, globals.undo_prev_state_info.y, globals.undo_prev_state_info.z != 0u));
            }

            if (line_a > 0.0) {
                let text_alpha = select(0.95, 0.65, !undo_available);
                let text_rgb = select(vec3<f32>(1.0), vec3<f32>(0.78), !undo_available);
                let tmp = over_pm(out_pm, out_a, vec4<f32>(text_rgb, text_alpha * line_a));
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }

        // Current state button (clickable; shows display name)
        {
            let row_y0 = top_y0 + (button_h + button_gap);
            let row_y1 = row_y0 + button_h;
            let in_row = px.x >= button_x0 && px.x <= button_x1 && px.y >= row_y0 && px.y <= row_y1;
            if (in_row) {
                let hovered = globals.current_state_button_meta.x != 0u;
                let clicked = globals.current_state_button_meta.y != 0u;
                let on_row_border =
                    px.x <= button_x0 + 1.0 ||
                    px.x >= button_x1 - 1.0 ||
                    px.y <= row_y0 + 1.0 ||
                    px.y >= row_y1 - 1.0;
                var row_fill = select(
                    vec4<f32>(vec3<f32>(0.0), 0.45),
                    vec4<f32>(vec3<f32>(1.0), 0.28),
                    hovered,
                );
                row_fill = select(row_fill, vec4<f32>(vec3<f32>(1.0), 0.45), clicked);
                let row_border = select(
                    vec4<f32>(vec3<f32>(1.0), 0.8),
                    vec4<f32>(vec3<f32>(1.0), 1.0),
                    hovered || clicked,
                );
                let row_col = select(row_fill, row_border, on_row_border);
                let row_blend = over_pm(out_pm, out_a, row_col);
                out_pm = row_blend.rgb;
                out_a = row_blend.a;
            }

            let y = row_y0 + 7.0;
            var line_a: f32 = 0.0;
            let max_chars = u32(max(floor((button_x1 - button_x0 - 16.0) / adv), 0.0));
            let age_chars = u32_char_count(globals.undo_current_state_info.y) + 5u;
            let reserved_chars = age_chars + 1u;
            let name_max_chars = select(0u, max_chars - reserved_chars, max_chars > reserved_chars);
            line_a = max(
                line_a,
                current_state_name_alpha(px, button_x0 + 8.0, y, text_h, adv, name_max_chars),
            );

            let text_mode = globals.current_state_name_meta.y != 0u;
            if (!text_mode) {
                if (globals.current_state_name_meta.x == 0u) {
                    line_a = max(
                        line_a,
                        uint_u32_alpha(px, button_x0 + 8.0, y, text_h, adv, globals.undo_current_state_info.x),
                    );
                }
                line_a = max(
                    line_a,
                    age_ago_alpha_right(
                        px,
                        button_x1 - 8.0,
                        y,
                        text_h,
                        adv,
                        globals.undo_current_state_info.y,
                        globals.undo_current_state_info.z != 0u,
                    ),
                );
            }

            if (line_a > 0.0) {
                let tmp = over_pm(out_pm, out_a, vec4<f32>(vec3<f32>(1.0), 0.95 * line_a));
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }

        // Redo selectable rows (hidden when empty)
        for (var i: u32 = 0u; i < next_count_clamped; i = i + 1u) {
            let row_y0 = top_y0 + (button_h + button_gap) * 2.0 + f32(i) * (button_h + button_gap);
            let row_y1 = row_y0 + button_h;
            let in_row = px.x >= button_x0 && px.x <= button_x1 && px.y >= row_y0 && px.y <= row_y1;
            if (in_row) {
                let hovered = globals.redo_buttons_meta.y != 0u && globals.redo_buttons_meta.x == i;
                let clicked = globals.redo_buttons_meta.w != 0u && globals.redo_buttons_meta.z == i;
                let on_row_border =
                    px.x <= button_x0 + 1.0 ||
                    px.x >= button_x1 - 1.0 ||
                    px.y <= row_y0 + 1.0 ||
                    px.y >= row_y1 - 1.0;
                var row_fill = select(
                    vec4<f32>(vec3<f32>(0.0), 0.45),
                    vec4<f32>(vec3<f32>(1.0), 0.28),
                    hovered,
                );
                row_fill = select(row_fill, vec4<f32>(vec3<f32>(1.0), 0.45), clicked);
                let row_border = select(
                    vec4<f32>(vec3<f32>(1.0), 0.8),
                    vec4<f32>(vec3<f32>(1.0), 1.0),
                    hovered || clicked,
                );
                let row_col = select(row_fill, row_border, on_row_border);
                let row_blend = over_pm(out_pm, out_a, row_col);
                out_pm = row_blend.rgb;
                out_a = row_blend.a;
            }

            let y = row_y0 + 7.0;
            var line_a: f32 = 0.0;

            let redo_label_x = (button_x0 + button_x1 - adv * 4.0) * 0.5;
            var redo_x = redo_label_x;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(redo_x, y), text_h, 82u)); redo_x = redo_x + adv;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(redo_x, y), text_h, 69u)); redo_x = redo_x + adv;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(redo_x, y), text_h, 68u)); redo_x = redo_x + adv;
            line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(redo_x, y), text_h, 79u));

            var uuid: u32 = 0u;
            var age_value: u32 = 0u;
            var age_unit: u32 = 0u;
            if (i < 4u) {
                uuid = globals.undo_next_states_uuid_0[i];
                age_value = globals.undo_next_states_age_0[i];
                age_unit = globals.undo_next_states_age_unit_0[i];
            } else {
                uuid = globals.undo_next_states_uuid_1[i - 4u];
                age_value = globals.undo_next_states_age_1[i - 4u];
                age_unit = globals.undo_next_states_age_unit_1[i - 4u];
            }
            let max_chars = u32(max(floor((button_x1 - button_x0 - 16.0) / adv), 0.0));
            let age_chars = u32_char_count(age_value) + 5u;
            let reserved_chars = age_chars + 1u;
            let name_max_chars = select(0u, max_chars - reserved_chars, max_chars > reserved_chars);
            let name_len = undo_next_state_name_len(i);
            if (name_len > 0u) {
                line_a = max(
                    line_a,
                    packed_name_alpha(px, button_x0 + 8.0, y, text_h, adv, name_max_chars, name_len, 1u, i),
                );
            } else {
                line_a = max(line_a, uint_u32_alpha(px, button_x0 + 8.0, y, text_h, adv, uuid));
            }
            line_a = max(line_a, age_ago_alpha_right(px, button_x1 - 8.0, y, text_h, adv, age_value, age_unit != 0u));

            if (line_a > 0.0) {
                let tmp = over_pm(out_pm, out_a, vec4<f32>(vec3<f32>(1.0), 0.95 * line_a));
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }
    }

    // --- Selection details boxes below top-left stats box ---
    {
        let parent = globals.stats_box_rect;
        let gap = 8.0;
        let box_h = 222.0;
        let line_step = 16.0;
        let text_h = 14.0;
        let adv = (text_h / 7.0) * 6.0;
        let box_w = (parent.z - parent.x) - adv * 2.0;

        for (var side: u32 = 0u; side < 2u; side = side + 1u) {
            let exists = select(globals.selection_exists_meta.x != 0u, globals.selection_exists_meta.y != 0u, side == 1u);
            if (!exists) {
                continue;
            }

            let box_x0 = parent.x;
            let box_y0 = parent.w + gap + f32(side) * (box_h + gap);
            let box_x1 = box_x0 + box_w;
            let box_y1 = box_y0 + box_h;

            let color = side_selection_color(side, SC_SELECTION_BORDER);
            let count = select(globals.selection_meta.z, globals.selection_meta.w, side == 1u);
            let scale_v = max(select(globals.selection_left_scale, globals.selection_right_scale, side == 1u), 0.0);
            let rot_v = select(globals.selection_left_rotation_degrees, globals.selection_right_rotation_degrees, side == 1u);
            let origin_pf = select(globals.selection_origin_left_playfield, globals.selection_origin_right_playfield, side == 1u);
            let moved_pf = select(globals.selection_moved_left_playfield, globals.selection_moved_right_playfield, side == 1u);
            let pos_locked = select(globals.selection_lock_meta.x != 0u, globals.selection_lock_meta.y != 0u, side == 1u);
            let scale_locked = select(globals.selection_lock_meta.z != 0u, globals.selection_lock_meta.w != 0u, side == 1u);

            if (px.x >= box_x0 && px.x <= box_x1 && px.y >= box_y0 && px.y <= box_y1) {
                let border = 1.0;
                let on_border =
                    px.x <= box_x0 + border ||
                    px.x >= box_x1 - border ||
                    px.y <= box_y0 + border ||
                    px.y >= box_y1 - border;
                let panel_fill = vec4<f32>(vec3<f32>(0.0), 0.55);
                let panel_border = vec4<f32>(color.rgb, max(color.a, 0.95));
                let panel = select(panel_fill, panel_border, on_border);
                let blended = over_pm(out_pm, out_a, panel);
                out_pm = blended.rgb;
                out_a = blended.a;

                let text_x = box_x0 + 8.0;
                let longest_label_chars = 10.0; // LOCK_SCALE
                let value_x = text_x + adv * (longest_label_chars + 1.0);
                let signed_value_x = value_x - adv;
                let text_y0 = box_y0 + 8.0;
                let text_color = vec4<f32>(vec3<f32>(1.0), 0.95);

                // OBJECTS
                {
                    let y = text_y0 + line_step * 0.0;
                    var line_a: f32 = 0.0;
                    var x = text_x;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv; // O
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 66u)); x = x + adv; // B
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 74u)); x = x + adv; // J
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u)); x = x + adv; // E
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 67u)); x = x + adv; // C
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 84u)); x = x + adv; // T
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); // S

                    x = value_x;
                    var rev_digits: array<u32, 10>;
                    var rev_count: u32 = 0u;
                    var n = count;
                    loop {
                        rev_digits[rev_count] = n % 10u;
                        rev_count = rev_count + 1u;
                        if (n < 10u || rev_count >= 10u) {
                            break;
                        }
                        n = n / 10u;
                    }
                    for (var i: u32 = 0u; i < rev_count; i = i + 1u) {
                        let d = rev_digits[rev_count - 1u - i];
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 48u + d));
                        x = x + adv;
                    }

                    if (line_a > 0.0) {
                        let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                        out_pm = t.rgb;
                        out_a = t.a;
                    }
                }

                // SCALE
                {
                    let y = text_y0 + line_step * 2.0;
                    var line_a: f32 = 0.0;
                    var x = text_x;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 67u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u));

                    let v100 = u32(round(scale_v * 100.0));
                    line_a = max(line_a, decimal_u32_x100_alpha(px, value_x, y, text_h, adv, v100));

                    if (line_a > 0.0) {
                        let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                        out_pm = t.rgb;
                        out_a = t.a;
                    }
                }

                // ROT
                {
                    let y = text_y0 + line_step * 3.0;
                    var line_a: f32 = 0.0;
                    var x = text_x;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 82u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 84u));

                    let v10 = i32(round(rot_v * 10.0));
                    line_a = max(line_a, decimal_i32_x10_alpha(px, signed_value_x, y, text_h, adv, v10));

                    if (line_a > 0.0) {
                        let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                        out_pm = t.rgb;
                        out_a = t.a;
                    }
                }

                // X
                {
                    let y = text_y0 + line_step * 5.0;
                    var line_a: f32 = 0.0;
                    var x = text_x;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 88u));

                    let x10 = i32(round(origin_pf.x * 10.0));
                    line_a = max(line_a, decimal_i32_x10_alpha(px, signed_value_x, y, text_h, adv, x10));

                    if (line_a > 0.0) {
                        let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                        out_pm = t.rgb;
                        out_a = t.a;
                    }
                }

                // Y
                {
                    let y = text_y0 + line_step * 6.0;
                    var line_a: f32 = 0.0;
                    var x = text_x;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 89u));

                    let y10 = i32(round(origin_pf.y * 10.0));
                    line_a = max(line_a, decimal_i32_x10_alpha(px, signed_value_x, y, text_h, adv, y10));

                    if (line_a > 0.0) {
                        let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                        out_pm = t.rgb;
                        out_a = t.a;
                    }
                }

                // DX
                {
                    let y = text_y0 + line_step * 8.0;
                    var line_a: f32 = 0.0;
                    var x = text_x;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 68u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 88u));

                    let dx10 = i32(round(moved_pf.x * 10.0));
                    line_a = max(line_a, decimal_i32_x10_alpha(px, signed_value_x, y, text_h, adv, dx10));

                    if (line_a > 0.0) {
                        let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                        out_pm = t.rgb;
                        out_a = t.a;
                    }
                }

                // DY
                {
                    let y = text_y0 + line_step * 9.0;
                    var line_a: f32 = 0.0;
                    var x = text_x;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 68u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 89u));

                    let dy10 = i32(round(moved_pf.y * 10.0));
                    line_a = max(line_a, decimal_i32_x10_alpha(px, signed_value_x, y, text_h, adv, dy10));

                    if (line_a > 0.0) {
                        let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                        out_pm = t.rgb;
                        out_a = t.a;
                    }
                }

                // LOCK_POS
                {
                    let y = text_y0 + line_step * 11.0;
                    var line_a: f32 = 0.0;
                    var x = text_x;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 67u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 75u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 95u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 80u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u));
                    

                    x = value_x;
                    if (pos_locked) {
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 84u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 82u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 85u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u));
                    } else {
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 70u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u));
                    }

                    if (line_a > 0.0) {
                        let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                        out_pm = t.rgb;
                        out_a = t.a;
                    }
                }

                // LOCK_SCALE
                {
                    let y = text_y0 + line_step * 12.0;
                    var line_a: f32 = 0.0;
                    var x = text_x;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 79u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 67u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 75u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 95u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 67u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv;
                    line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u));

                    x = value_x;
                    if (scale_locked) {
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 84u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 82u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 85u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u));
                    } else {
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 70u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 65u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 76u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 83u)); x = x + adv;
                        line_a = max(line_a, glyph5x7_alpha(px, vec2<f32>(x, y), text_h, 69u));
                    }

                    if (line_a > 0.0) {
                        let t = over_pm(out_pm, out_a, vec4<f32>(text_color.rgb, text_color.a * line_a));
                        out_pm = t.rgb;
                        out_a = t.a;
                    }
                }
            }
        }
    }

    // --- Top timeline placeholder boxes ---
    {
        let cursor = globals.cursor_pos;
        // Box 1
        if (px.y >= top_bar0_y0 && px.y <= top_bar0_y1 && px.x >= top_bar0_x0 && px.x <= top_bar0_x1 && !px_in_perf_box) {
            let on_border =
                px.x <= top_bar0_x0 + 1.0 ||
                px.x >= top_bar0_x1 - 1.0 ||
                px.y <= top_bar0_y0 + 1.0 ||
                px.y >= top_bar0_y1 - 1.0;
            let panel_bg = vec4<f32>(vec3<f32>(0.0), 0.55);
            let panel_border = vec4<f32>(vec3<f32>(1.0), 0.9);
            let panel = select(panel_bg, panel_border, on_border);
            let panel_blend = over_pm(out_pm, out_a, panel);
            out_pm = panel_blend.rgb;
            out_a = panel_blend.a;

            let is_past_side = px.x <= globals.timeline_current_x;
            if (is_past_side) {
                let under_tint_alpha = clamp(globals.timeline_past_tint_rgba.a, 0.0, 1.0);
                if (under_tint_alpha > 0.0) {
                    let under_tint = vec4<f32>(globals.timeline_past_tint_rgba.rgb, under_tint_alpha);
                    let t = over_pm(out_pm, out_a, under_tint);
                    out_pm = t.rgb;
                    out_a = t.a;
                }
            }

        }

        // Box 2
        if (px.y >= top_bar1_y0 && px.y <= top_bar1_y1 && px.x >= top_bar1_x0 && px.x <= top_bar1_x1 && !px_in_perf_box) {
            let on_border =
                px.x <= top_bar1_x0 + 1.0 ||
                px.x >= top_bar1_x1 - 1.0 ||
                px.y <= top_bar1_y0 + 1.0 ||
                px.y >= top_bar1_y1 - 1.0;
            let panel_bg = vec4<f32>(vec3<f32>(0.0), 0.55);
            let panel_border = vec4<f32>(vec3<f32>(1.0), 0.9);
            let panel = select(panel_bg, panel_border, on_border);
            let panel_blend = over_pm(out_pm, out_a, panel);
            out_pm = panel_blend.rgb;
            out_a = panel_blend.a;

        }

        // Box 3
        if (px.y >= top_bar2_y0 && px.y <= top_bar2_y1 && px.x >= top_bar2_x0 && px.x <= top_bar2_x1 && !px_in_perf_box) {
            let on_border =
                px.x <= top_bar2_x0 + 1.0 ||
                px.x >= top_bar2_x1 - 1.0 ||
                px.y <= top_bar2_y0 + 1.0 ||
                px.y >= top_bar2_y1 - 1.0;
            let panel_bg = vec4<f32>(vec3<f32>(0.0), 0.55);
            let panel_border = vec4<f32>(vec3<f32>(1.0), 0.9);
            let panel = select(panel_bg, panel_border, on_border);
            let panel_blend = over_pm(out_pm, out_a, panel);
            out_pm = panel_blend.rgb;
            out_a = panel_blend.a;

        }
    }

    // --- Timeline hitbox tint ---
    if (px.y >= hitbox_y0 && px.y <= hitbox_y1 && px.x >= hitbox_x0 && px.x <= hitbox_x1 && !px_in_perf_box) {
        let tint = vec4<f32>(timeline_rgb, 0.125);
        let tmp = over_pm(out_pm, out_a, tint);
        out_pm = tmp.rgb;
        out_a = tmp.a;

        let top_y1 = hitbox_y0 + (hitbox_y1 - hitbox_y0) * 0.5;
        if (px.y <= top_y1 && px.x <= fill_x) {
            let top_progress = vec4<f32>(timeline_rgb, 0.25);
            let tmp2 = over_pm(out_pm, out_a, top_progress);
            out_pm = tmp2.rgb;
            out_a = tmp2.a;
        }
    }

    // --- Progress bar ---
    if (px.y >= bar_y0 && px.y <= bar_y1 && px.x >= bar_x0 && px.x <= bar_x1 && !px_in_perf_box) {
        // Background track.
        let bg = vec4<f32>(timeline_rgb, 0.5);
        let tmp = over_pm(out_pm, out_a, bg);
        out_pm = tmp.rgb;
        out_a = tmp.a;

        // Filled section.
        if (px.x <= fill_x) {
            let fg = vec4<f32>(timeline_rgb, 1.0);
            let tmp2 = over_pm(out_pm, out_a, fg);
            out_pm = tmp2.rgb;
            out_a = tmp2.a;
        }
    }

    // --- Editor drag rectangles + selection rectangles ---
    {
        let selection_border_px = 10.0;
        let selection_border_aa = 1.0;
        let drag_border_px = 2.0;

        if (globals.selection_meta.y != 0u) {
            let q = selection_quad_right();
            let aabb = quad_aabb(q);
            let x0 = aabb.x;
            let y0 = aabb.y;
            let x1 = aabb.z;
            let y1 = aabb.w;
            if (px.x >= x0 - selection_border_px && px.x <= x1 + selection_border_px &&
                px.y >= y0 - selection_border_px && px.y <= y1 + selection_border_px) {
                let inside = point_in_quad(px, q);
                let border_alpha = quad_outside_border_alpha(px, q, selection_border_px, selection_border_aa);
                let hovered = globals.overlay_meta.w != 0u;
                let dragging = globals.selection_box_dragging_meta.y != 0u;
                let border_color = select(
                    side_selection_color(1u, SC_SELECTION_BORDER),
                    select(
                        side_selection_color(1u, SC_SELECTION_BORDER_HOVERED),
                        side_selection_color(1u, SC_SELECTION_BORDER_DRAGGING),
                        dragging,
                    ),
                    hovered || dragging,
                );
                let tint_color = select(
                    side_selection_color(1u, SC_SELECTION_TINT),
                    select(
                        side_selection_color(1u, SC_SELECTION_TINT_HOVERED),
                        side_selection_color(1u, SC_SELECTION_TINT_DRAGGING),
                        dragging,
                    ),
                    hovered || dragging,
                );
                let fill = vec4<f32>(tint_color.rgb, clamp(tint_color.a, 0.0, 1.0));
                let border_col = vec4<f32>(border_color.rgb, clamp(border_color.a, 0.0, 1.0));
                if (inside) {
                    let tmp = over_pm(out_pm, out_a, fill);
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
                if (border_alpha > 1e-4) {
                    let aa_border = vec4<f32>(border_col.rgb, border_col.a * border_alpha);
                    let tmp = over_pm(out_pm, out_a, aa_border);
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

            let origin = globals.selection_origin_right.xy;
            let guide_width = 1.0;
            let guide_aa = 1.0;
            var guide_a = 0.0;
            guide_a = max(guide_a, segment_line_alpha(px, origin, q[0], guide_width, guide_aa));
            guide_a = max(guide_a, segment_line_alpha(px, origin, q[1], guide_width, guide_aa));
            guide_a = max(guide_a, segment_line_alpha(px, origin, q[2], guide_width, guide_aa));
            guide_a = max(guide_a, segment_line_alpha(px, origin, q[3], guide_width, guide_aa));
            if (guide_a > 1e-4) {
                let hovered = globals.overlay_meta.w != 0u;
                let dragging = globals.selection_origin_right.w > 0.5;
                let c = select(
                    select(
                        side_selection_color(1u, SC_SELECTION_BORDER),
                        side_selection_color(1u, SC_SELECTION_BORDER_HOVERED),
                        hovered,
                    ),
                    side_selection_color(1u, SC_SELECTION_BORDER_DRAGGING),
                    dragging,
                );
                let guide = vec4<f32>(mix(c.rgb, vec3<f32>(1.0), 0.25), max(c.a, 0.9) * 0.75 * guide_a);
                let tmp = over_pm(out_pm, out_a, guide);
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }

        if (globals.selection_meta.x != 0u) {
            let q = selection_quad_left();
            let aabb = quad_aabb(q);
            let x0 = aabb.x;
            let y0 = aabb.y;
            let x1 = aabb.z;
            let y1 = aabb.w;
            if (px.x >= x0 - selection_border_px && px.x <= x1 + selection_border_px &&
                px.y >= y0 - selection_border_px && px.y <= y1 + selection_border_px) {
                let inside = point_in_quad(px, q);
                let border_alpha = quad_outside_border_alpha(px, q, selection_border_px, selection_border_aa);
                let hovered = globals.overlay_meta.z != 0u;
                let dragging = globals.selection_box_dragging_meta.x != 0u;
                let border_color = select(
                    side_selection_color(0u, SC_SELECTION_BORDER),
                    select(
                        side_selection_color(0u, SC_SELECTION_BORDER_HOVERED),
                        side_selection_color(0u, SC_SELECTION_BORDER_DRAGGING),
                        dragging,
                    ),
                    hovered || dragging,
                );
                let tint_color = select(
                    side_selection_color(0u, SC_SELECTION_TINT),
                    select(
                        side_selection_color(0u, SC_SELECTION_TINT_HOVERED),
                        side_selection_color(0u, SC_SELECTION_TINT_DRAGGING),
                        dragging,
                    ),
                    hovered || dragging,
                );
                let fill = vec4<f32>(tint_color.rgb, clamp(tint_color.a, 0.0, 1.0));
                let border_col = vec4<f32>(border_color.rgb, clamp(border_color.a, 0.0, 1.0));
                if (inside) {
                    let tmp = over_pm(out_pm, out_a, fill);
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
                if (border_alpha > 1e-4) {
                    let aa_border = vec4<f32>(border_col.rgb, border_col.a * border_alpha);
                    let tmp = over_pm(out_pm, out_a, aa_border);
                    out_pm = tmp.rgb;
                    out_a = tmp.a;
                }
            }

            let origin = globals.selection_origin_left.xy;
            let guide_width = 1.0;
            let guide_aa = 1.0;
            var guide_a = 0.0;
            guide_a = max(guide_a, segment_line_alpha(px, origin, q[0], guide_width, guide_aa));
            guide_a = max(guide_a, segment_line_alpha(px, origin, q[1], guide_width, guide_aa));
            guide_a = max(guide_a, segment_line_alpha(px, origin, q[2], guide_width, guide_aa));
            guide_a = max(guide_a, segment_line_alpha(px, origin, q[3], guide_width, guide_aa));
            if (guide_a > 1e-4) {
                let hovered = globals.overlay_meta.z != 0u;
                let dragging = globals.selection_origin_left.w > 0.5;
                let c = select(
                    select(
                        side_selection_color(0u, SC_SELECTION_BORDER),
                        side_selection_color(0u, SC_SELECTION_BORDER_HOVERED),
                        hovered,
                    ),
                    side_selection_color(0u, SC_SELECTION_BORDER_DRAGGING),
                    dragging,
                );
                let guide = vec4<f32>(mix(c.rgb, vec3<f32>(1.0), 0.25), max(c.a, 0.9) * 0.75 * guide_a);
                let tmp = over_pm(out_pm, out_a, guide);
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }

        if (globals.selection_meta.y != 0u && globals.selection_origin_right.w <= 0.5) {
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

        if (globals.selection_meta.x != 0u && globals.selection_origin_left.w <= 0.5) {
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

        if (globals.overlay_meta.x != 0u) {
            let r = globals.overlay_rect_left;
            let x0 = min(r.x, r.z);
            let y0 = min(r.y, r.w);
            let x1 = max(r.x, r.z);
            let y1 = max(r.y, r.w);
            if (px.x >= x0 && px.x <= x1 && px.y >= y0 && px.y <= y1) {
                let border =
                    px.x <= x0 + drag_border_px ||
                    px.x >= x1 - drag_border_px ||
                    px.y <= y0 + drag_border_px ||
                    px.y >= y1 - drag_border_px;
                let color = side_selection_color(0u, SC_DRAG_RECT);
                let fill = vec4<f32>(color.rgb, color.a * 0.5);
                let border_col = vec4<f32>(color.rgb, clamp(color.a + 0.2, 0.0, 1.0));
                let tmp = over_pm(out_pm, out_a, select(fill, border_col, border));
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }

        if (globals.overlay_meta.y != 0u) {
            let r = globals.overlay_rect_right;
            let x0 = min(r.x, r.z);
            let y0 = min(r.y, r.w);
            let x1 = max(r.x, r.z);
            let y1 = max(r.y, r.w);
            if (px.x >= x0 && px.x <= x1 && px.y >= y0 && px.y <= y1) {
                let border =
                    px.x <= x0 + drag_border_px ||
                    px.x >= x1 - drag_border_px ||
                    px.y <= y0 + drag_border_px ||
                    px.y >= y1 - drag_border_px;
                let color = side_selection_color(1u, SC_DRAG_RECT);
                let fill = vec4<f32>(color.rgb, color.a * 0.5);
                let border_col = vec4<f32>(color.rgb, clamp(color.a + 0.2, 0.0, 1.0));
                let tmp = over_pm(out_pm, out_a, select(fill, border_col, border));
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }
    }

    // --- Play/pause indicator ---
    {
        let color = vec4<f32>(vec3<f32>(1.0), 0.9);
        let icon_x = play_pause_x0;
        let icon_y = play_pause_y0;
        let icon_w = max(1.0, play_pause_x1 - play_pause_x0);
        let icon_h = max(1.0, play_pause_y1 - play_pause_y0);

        if (px.x >= play_pause_x0 && px.x <= play_pause_x1
            && px.y >= play_pause_y0 && px.y <= play_pause_y1) {
            if (globals.play_pause_button_meta.y != 0u) {
                let click_tint = vec4<f32>(vec3<f32>(1.0), 0.45);
                let ctmp = over_pm(out_pm, out_a, click_tint);
                out_pm = ctmp.rgb;
                out_a = ctmp.a;
            } else if (globals.play_pause_button_meta.x != 0u) {
                let hover_tint = vec4<f32>(vec3<f32>(1.0), 0.2);
                let htmp = over_pm(out_pm, out_a, hover_tint);
                out_pm = htmp.rgb;
                out_a = htmp.a;
            }
        }

        let is_playing = globals.is_playing != 0;
        let ia = select(
            play_icon_alpha(px, icon_x, icon_y, icon_w, icon_h),
            pause_icon_alpha(px, icon_x, icon_y, icon_w, icon_h),
            is_playing,
        );
        if (ia > 0.0) {
            let tmp = over_pm(out_pm, out_a, vec4<f32>(color.rgb, color.a * ia));
            out_pm = tmp.rgb;
            out_a = tmp.a;
        }
    }

    // --- Loading spinner (center screen) ---
    if (globals.loading != 0u) {
        let center = vec2<f32>(res.x * 0.5, res.y * 0.5);
        let tex_dim = vec2<f32>(textureDimensions(loading_tex));
        let max_dim = max(tex_dim.x, tex_dim.y);
        let size = min(res.x, res.y) * 0.5;
        let scale = size / max(max_dim, 1.0);
        let half_size = vec2<f32>(tex_dim.x, tex_dim.y) * scale * 0.5;
        let angle = globals.time_elapsed_ms * 0.001 * 9.0;

        let local = rotate2(px - center, -angle);
        let uv = (local / (half_size * 2.0)) + vec2<f32>(0.5, 0.5);

        if (all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0))) {
            let tex = textureSample(loading_tex, skin_samp, uv);
            if (tex.a > 1e-6) {
                let tmp = over_pm(out_pm, out_a, tex);
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }
    }

    // --- Break texture (center gameplay area) ---
    if (globals.is_break_time != 0u) {
        let os = globals.osu_rect;
        let center = vec2<f32>((os.x + os.z) * 0.5, (os.y + os.w) * 0.5);
        let gameplay_size = vec2<f32>(max(os.z - os.x, 1.0), max(os.w - os.y, 1.0));
        let tex_dim = vec2<f32>(textureDimensions(break_tex));
        let max_dim = max(tex_dim.x, tex_dim.y);
        let size = min(gameplay_size.x, gameplay_size.y) * 0.6;
        let scale = size / max(max_dim, 1.0);
        let half_size = vec2<f32>(tex_dim.x, tex_dim.y) * scale * 0.5;
        let angle = globals.time_elapsed_ms * 0.001 * 0.9;

        let local = rotate2(px - center, angle);
        let uv = (local / (half_size * 2.0)) + vec2<f32>(0.5, 0.5);

        if (all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0))) {
            let tex = textureSample(break_tex, skin_samp, uv);
            if (tex.a > 1e-6) {
                let fade = break_spinner_alpha(globals.time_ms, globals.break_time);
                let tmp = over_pm(out_pm, out_a, vec4<f32>(tex.rgb, tex.a * fade));
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }
    }

    // --- Spinner circle (center screen) ---spi
    if (globals.spinner_state != 0u) {
        let os = globals.osu_rect;
        let center = vec2<f32>((os.x + os.z) * 0.5, (os.y + os.w) * 0.5);
        let gameplay_size = vec2<f32>(max(os.z - os.x, 1.0), max(os.w - os.y, 1.0));
        let tex_dim = vec2<f32>(textureDimensions(spinner_tex));
        let max_dim = max(tex_dim.x, tex_dim.y);
        let size = min(gameplay_size.x, gameplay_size.y) * 0.6;
        let scale = size / max(max_dim, 1.0);
        let half_size = vec2<f32>(tex_dim.x, tex_dim.y) * scale * 0.5;

        let spin_rate = 9.0;
        let spinning = globals.spinner_state == 1u;
        let angle = select(
            globals.spinner_time.y * 0.001 * spin_rate,
            globals.time_ms * 0.001 * spin_rate,
            spinning,
        );

        let local = rotate2(px - center, angle);
        let uv = (local / (half_size * 2.0)) + vec2<f32>(0.5, 0.5);

        if (all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0))) {
            let tex = textureSample(spinner_tex, skin_samp, uv);
            if (tex.a > 1e-6) {
                let post_fade_ms = 500.0;
                let idle_fade = 1.0 - clamp((globals.time_ms - globals.spinner_time.y) / post_fade_ms, 0.0, 1.0);
                let fade = select(idle_fade, 1.0, spinning);

                let selected = globals.spinner_selection_meta.x != 0u;
                let tint = select(
                    side_selection_color(0u, SC_SELECTION_TINT),
                    side_selection_color(1u, SC_SELECTION_TINT),
                    globals.spinner_selection_meta.x == 2u,
                );
                let tint_mix = clamp(tint.a, 0.0, 1.0);
                let spinner_rgb = select(tex.rgb, mix(tex.rgb, tint.rgb, tint_mix), selected);

                let tmp = over_pm(out_pm, out_a, vec4<f32>(spinner_rgb, tex.a * fade));
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }

        if (globals.spinner_selection_meta.x != 0u) {
            let dist = length(px - center);
            let ring_radius = min(gameplay_size.x, gameplay_size.y) * (0.05 / 3.0);
            let ring_thickness = 12.0;
            let ring_sd = abs(dist - ring_radius) - ring_thickness * 0.5;
            let aa = max(fwidth(dist), 1.0);
            let ring_alpha = 1.0 - smoothstep(-aa, aa, ring_sd);
            if (ring_alpha > 1e-4) {
                let tint = select(
                    side_selection_color(0u, SC_SELECTION_TINT),
                    side_selection_color(1u, SC_SELECTION_TINT),
                    globals.spinner_selection_meta.x == 2u,
                );
                let ring_col = vec4<f32>(tint.rgb, 0.95 * ring_alpha);
                let tmp = over_pm(out_pm, out_a, ring_col);
                out_pm = tmp.rgb;
                out_a = tmp.a;
            }
        }
    }

    out_pm = out_pm * opacity;
    out_a = out_a * opacity;

    if (out_a <= 1e-6) {
        discard;
    }
    return vec4<f32>(out_pm, out_a);
}

@fragment
fn fs_timeline_kiai(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let res = globals.screen_size;
    let px = uv * res;
    let opacity = clamp(globals.hud_opacity, 0.0, 1.0);
    if (opacity <= 1e-6) {
        discard;
    }

    let bar_x0 = globals.timeline_rect.x;
    let bar_y0 = globals.timeline_rect.y;
    let bar_x1 = globals.timeline_rect.z;
    let bar_y1 = globals.timeline_rect.w;
    let hitbox_x0 = globals.timeline_hitbox_rect.x;
    let hitbox_y0 = globals.timeline_hitbox_rect.y;
    let hitbox_x1 = globals.timeline_hitbox_rect.z;
    let hitbox_y1 = globals.timeline_hitbox_rect.w;
    let total = max(globals.song_total_ms, 0.0);
    let count = min(globals.kiai_interval_count, 128u);

    let in_interval = px_in_timeline_interval(px.x, total, bar_x0, bar_x1, count);
    if (!in_interval) {
        discard;
    }

    let fill_x = timeline_fill_x(total, bar_x0, bar_x1);
    let color = vec3<f32>(1.0, 0.55, 0.0);
    var out_pm = vec3<f32>(0.0);
    var out_a: f32 = 0.0;

    if (px.y >= hitbox_y0 && px.y <= hitbox_y1 && px.x >= hitbox_x0 && px.x <= hitbox_x1) {
        let tint = vec4<f32>(color, 0.125);
        let tmp = over_pm(out_pm, out_a, tint);
        out_pm = tmp.rgb;
        out_a = tmp.a;

        let top_y1 = hitbox_y0 + (hitbox_y1 - hitbox_y0) * 0.5;
        if (px.y <= top_y1 && px.x <= fill_x) {
            let top_progress = vec4<f32>(color, 0.25);
            let tmp2 = over_pm(out_pm, out_a, top_progress);
            out_pm = tmp2.rgb;
            out_a = tmp2.a;
        }
    }

    if (px.y >= bar_y0 && px.y <= bar_y1 && px.x >= bar_x0 && px.x <= bar_x1) {
        let bg = vec4<f32>(color, 0.5);
        let tmp = over_pm(out_pm, out_a, bg);
        out_pm = tmp.rgb;
        out_a = tmp.a;

        if (px.x <= fill_x) {
            let fg = vec4<f32>(color, 1.0);
            let tmp2 = over_pm(out_pm, out_a, fg);
            out_pm = tmp2.rgb;
            out_a = tmp2.a;
        }
    }

    out_pm = out_pm * opacity;
    out_a = out_a * opacity;
    if (out_a <= 1e-6) {
        discard;
    }
    return vec4<f32>(out_pm, out_a);
}

@fragment
fn fs_timeline_break(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let res = globals.screen_size;
    let px = uv * res;
    let opacity = clamp(globals.hud_opacity, 0.0, 1.0);
    if (opacity <= 1e-6) {
        discard;
    }

    let bar_x0 = globals.timeline_rect.x;
    let bar_y0 = globals.timeline_rect.y;
    let bar_x1 = globals.timeline_rect.z;
    let bar_y1 = globals.timeline_rect.w;
    let hitbox_x0 = globals.timeline_hitbox_rect.x;
    let hitbox_y0 = globals.timeline_hitbox_rect.y;
    let hitbox_x1 = globals.timeline_hitbox_rect.z;
    let hitbox_y1 = globals.timeline_hitbox_rect.w;
    let total = max(globals.song_total_ms, 0.0);
    let count = min(globals.break_interval_count, 128u);

    let in_interval = px_in_timeline_interval(px.x, total, bar_x0, bar_x1, count);
    if (!in_interval) {
        discard;
    }

    let fill_x = timeline_fill_x(total, bar_x0, bar_x1);
    let color = vec3<f32>(0.35, 0.35, 0.35);
    var out_pm = vec3<f32>(0.0);
    var out_a: f32 = 0.0;

    if (px.y >= hitbox_y0 && px.y <= hitbox_y1 && px.x >= hitbox_x0 && px.x <= hitbox_x1) {
        let tint = vec4<f32>(color, 0.125);
        let tmp = over_pm(out_pm, out_a, tint);
        out_pm = tmp.rgb;
        out_a = tmp.a;

        let top_y1 = hitbox_y0 + (hitbox_y1 - hitbox_y0) * 0.5;
        if (px.y <= top_y1 && px.x <= fill_x) {
            let top_progress = vec4<f32>(color, 0.25);
            let tmp2 = over_pm(out_pm, out_a, top_progress);
            out_pm = tmp2.rgb;
            out_a = tmp2.a;
        }
    }

    if (px.y >= bar_y0 && px.y <= bar_y1 && px.x >= bar_x0 && px.x <= bar_x1) {
        let bg = vec4<f32>(color, 0.5);
        let tmp = over_pm(out_pm, out_a, bg);
        out_pm = tmp.rgb;
        out_a = tmp.a;

        if (px.x <= fill_x) {
            let fg = vec4<f32>(color, 1.0);
            let tmp2 = over_pm(out_pm, out_a, fg);
            out_pm = tmp2.rgb;
            out_a = tmp2.a;
        }
    }

    out_pm = out_pm * opacity;
    out_a = out_a * opacity;
    if (out_a <= 1e-6) {
        discard;
    }
    return vec4<f32>(out_pm, out_a);
}

@fragment
fn fs_timeline_bookmarks(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let res = globals.screen_size;
    let px = uv * res;
    let opacity = clamp(globals.hud_opacity, 0.0, 1.0);
    if (opacity <= 1e-6) {
        discard;
    }

    let bar_x0 = globals.timeline_rect.x;
    let hitbox_y0 = globals.timeline_hitbox_rect.y;
    let bar_x1 = globals.timeline_rect.z;
    let hitbox_y1 = globals.timeline_hitbox_rect.w;
    let total = max(globals.song_total_ms, 0.0);
    let bookmark_count = min(globals.bookmark_count, 256u);
    let red_line_count = min(globals.red_line_count, 256u);
    if (total <= 0.0 || (bookmark_count + red_line_count) == 0u) {
        discard;
    }
    if (!(px.y >= hitbox_y0 && px.y <= hitbox_y1 && px.x >= bar_x0 && px.x <= bar_x1)) {
        discard;
    }

    let hitbox_mid_y = hitbox_y0 + (hitbox_y1 - hitbox_y0) * 0.5;

    if (px.y <= hitbox_mid_y) {
        for (var i: u32 = 0u; i < bookmark_count; i = i + 1u) {
            let bookmark_ms = timeline_marks[i].x;
            let bookmark_frac = clamp(bookmark_ms / max(total, 1.0), 0.0, 1.0);
            let bx = mix(bar_x0, bar_x1, bookmark_frac);
            if (px.x >= bx && px.x < bx + 1.0) {
                let marker = vec4<f32>(vec3<f32>(0.2, 0.45, 1.0), opacity);
                return marker;
            }
        }
        discard;
    }

    for (var i: u32 = 0u; i < red_line_count; i = i + 1u) {
        let idx = bookmark_count + i;
        let red_line_ms = timeline_marks[idx].x;
        let red_line_frac = clamp(red_line_ms / max(total, 1.0), 0.0, 1.0);
        let bx = mix(bar_x0, bar_x1, red_line_frac);
        if (px.x >= bx && px.x < bx + 1.0) {
            let marker = vec4<f32>(vec3<f32>(1.0, 0.2, 0.2), opacity);
            return marker;
        }
    }

    discard;
}

@fragment
fn fs_bg(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let uvc = cover_uv(uv);
    let bg = textureSample(background_tex, skin_samp, uvc);

    // Output premultiplied RGB.
    var out_pm = bg.rgb * bg.a;
    var out_a = bg.a;

    let break_mul = break_alpha_multiplier(globals.time_ms, globals.break_time, globals.break_time_lightness);

    // 512x384 playfield outline overlay.
    let res = globals.screen_size;
    let px = uv * res;
    let pf = globals.playfield_rect;
    let pf_x0 = pf.x;
    let pf_y0 = pf.y;
    let pf_x1 = pf.z;
    let pf_y1 = pf.w;

    let inside_pf = px.x >= pf_x0 && px.x <= pf_x1 && px.y >= pf_y0 && px.y <= pf_y1;
    // 640x480 osu! coordinate system outline overlay (center shifted down by 8 game px).
    let os = globals.osu_rect;
    let os_x0 = os.x;
    let os_y0 = os.y;
    let os_x1 = os.z;
    let os_y1 = os.w;

    let inside_osu = px.x >= os_x0 && px.x <= os_x1 && px.y >= os_y0 && px.y <= os_y1;

    // Dim/tint layers via RGBA controls.
    // Outside 640x480 (outer)
    if (!inside_osu) {
        let o = globals.outer_rgba;
        let o_a = clamp(o.a * break_mul, 0.0, 1.0);
        let o_pm = o.rgb * o_a;
        out_pm = out_pm * (1.0 - o_a) + o_pm;
        out_a = out_a + o_a * (1.0 - out_a);
    }

    // Between 640x480 and 512x384 (gameplay)
    if (inside_osu && !inside_pf) {
        let g = globals.gameplay_rgba;
        let g_a = clamp(g.a * break_mul, 0.0, 1.0);
        let g_pm = g.rgb * g_a;
        out_pm = out_pm * (1.0 - g_a) + g_pm;
        out_a = out_a + g_a * (1.0 - out_a);
    }

    // Inside 512x384 playfield
    if (inside_pf) {
        let p = globals.playfield_rgba;
        let p_a = clamp(p.a * break_mul, 0.0, 1.0);
        let p_pm = p.rgb * p_a;
        out_pm = out_pm * (1.0 - p_a) + p_pm;
        out_a = out_a + p_a * (1.0 - out_a);
    }

    let border_aa = 1.0;

    let os_outline_alpha = rect_outline_alpha(px, os_x0, os_y0, os_x1, os_y1, 1.5, border_aa);
    if (os_outline_alpha > 1e-4) {
        let o = globals.gameplay_border_rgba;
        let o_a = clamp(o.a * break_mul * os_outline_alpha, 0.0, 1.0);
        let o_pm = o.rgb * o_a;
        out_pm = out_pm * (1.0 - o_a) + o_pm;
        out_a = out_a + o_a * (1.0 - out_a);
    }

    let pf_outline_alpha = rect_outline_alpha(px, pf_x0, pf_y0, pf_x1, pf_y1, 2.0, border_aa);
    if (pf_outline_alpha > 1e-4) {
        let o = globals.playfield_border_rgba;
        let o_a = clamp(o.a * break_mul * pf_outline_alpha, 0.0, 1.0);
        let o_pm = o.rgb * o_a;
        out_pm = out_pm * (1.0 - o_a) + o_pm;
        out_a = out_a + o_a * (1.0 - out_a);
    }

    return vec4<f32>(out_pm, out_a);
}
