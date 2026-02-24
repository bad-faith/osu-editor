pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect{
    pub fn to_f32_array(&self) -> [f32; 4] {
        [
            self.x0 as f32,
            self.y0 as f32,
            self.x1 as f32,
            self.y1 as f32,
        ]
    }
}

pub struct Layout {
    pub top_timeline_rect: Rect,
    pub top_timeline_hitbox_rect: Rect,
    pub top_timeline_second_rect: Rect,
    pub top_timeline_second_hitbox_rect: Rect,
    pub top_timeline_third_rect: Rect,
    pub top_timeline_third_hitbox_rect: Rect,
    pub timeline_rect: Rect,
    pub timeline_hitbox_rect: Rect,
    pub play_pause_button_rect: Rect,
    pub stats_box_rect: Rect,
    pub audio_volume_box_rect: Rect,
    pub hitsound_volume_box_rect: Rect,
    pub playfield_scale_box_rect: Rect,
    pub left_hitbox_rect: Rect,
    pub right_hitbox_rect: Rect,
    pub playfield_rect: Rect,
    pub gameplay_rect: Rect,
}

pub fn compute_layout(
    screen_w: f64,
    screen_h: f64,
    playfield_scale: f64,
    timeline_height_percent: f64,
    timeline_second_box_width_percent: f64,
    timeline_third_box_width_percent: f64,
) -> Layout {
    let top_timeline_height_px =
        (screen_h * timeline_height_percent.clamp(0.0, 1.0)).max(0.0);
    let (
        top_timeline_rect,
        top_timeline_hitbox_rect,
        top_timeline_second_rect,
        top_timeline_second_hitbox_rect,
        top_timeline_third_rect,
        top_timeline_third_hitbox_rect,
    ) = compute_top_timeline_rects(
        screen_w,
        top_timeline_height_px,
        timeline_second_box_width_percent,
        timeline_third_box_width_percent,
    );
    let timeline_rect = compute_timeline_rect(screen_w, screen_h);
    let timeline_hitbox_rect = compute_timeline_hitbox_rect(screen_w, screen_h);
    let play_pause_button_rect = compute_play_pause_button_rect(screen_h);
    let stats_box_rect = compute_stats_box_rect(top_timeline_height_px);
    let (audio_volume_box_rect, hitsound_volume_box_rect, playfield_scale_box_rect) =
        compute_volume_box_rects(&stats_box_rect);
    let (playfield_rect, gameplay_rect) = compute_playfield_and_gameplay_rects(screen_w, screen_h, playfield_scale);
    let (left_hitbox_rect, right_hitbox_rect) = compute_left_right_hitbox_rects(screen_w, screen_h);

    Layout {
        top_timeline_rect,
        top_timeline_hitbox_rect,
        top_timeline_second_rect,
        top_timeline_second_hitbox_rect,
        top_timeline_third_rect,
        top_timeline_third_hitbox_rect,
        timeline_rect,
        timeline_hitbox_rect,
        play_pause_button_rect,
        stats_box_rect,
        audio_volume_box_rect,
        hitsound_volume_box_rect,
        playfield_scale_box_rect,
        left_hitbox_rect,
        right_hitbox_rect,
        playfield_rect,
        gameplay_rect,
    }
}


fn compute_top_timeline_rects(
    screen_w: f64,
    timeline_height_px: f64,
    timeline_second_box_width_percent: f64,
    timeline_third_box_width_percent: f64,
) -> (Rect, Rect, Rect, Rect, Rect, Rect) {
    let margin = 8.0;
    let gap = margin;
    let y0 = 0.0;
    let y1 = y0 + timeline_height_px.max(0.0);

    let second_w = (screen_w * timeline_second_box_width_percent.clamp(0.0, 1.0)).max(0.0);
    let third_w = (screen_w * timeline_third_box_width_percent.clamp(0.0, 1.0)).max(0.0);
    let available_w = (screen_w - margin * 2.0 - gap * 2.0 - second_w - third_w).max(0.0);

    let first_x0 = margin;
    let first_x1 = first_x0 + available_w;

    let second_x0 = first_x1 + gap;
    let second_x1 = second_x0 + second_w;

    let third_x0 = second_x1 + gap;
    let third_x1 = third_x0 + third_w;

    let first = Rect {
        x0: first_x0,
        y0,
        x1: first_x1,
        y1,
    };
    let second = Rect {
        x0: second_x0,
        y0,
        x1: second_x1,
        y1,
    };
    let third = Rect {
        x0: third_x0,
        y0,
        x1: third_x1,
        y1,
    };

    (
        first,
        Rect {
            x0: first_x0,
            y0,
            x1: first_x1,
            y1,
        },
        second,
        Rect {
            x0: second_x0,
            y0,
            x1: second_x1,
            y1,
        },
        third,
        Rect {
            x0: third_x0,
            y0,
            x1: third_x1,
            y1,
        },
    )
}

fn compute_timeline_rect(screen_w: f64, screen_h: f64) -> Rect {
    let bar_height = 32.0;
    let x0 = 0.0;
    let x1 = screen_w;
    let y1 = screen_h;
    let y0 = (y1 - bar_height).max(0.0);
    Rect { x0, y0, x1, y1 }
}

fn compute_timeline_hitbox_rect(screen_w: f64, screen_h: f64) -> Rect {
    let hitbox_height = 64.0;
    let x0 = 0.0;
    let x1 = screen_w;
    let y1 = screen_h;
    let y0 = (y1 - hitbox_height).max(0.0);
    Rect { x0, y0, x1, y1 }
}

fn compute_play_pause_button_rect(screen_h: f64) -> Rect {
    let bar_height = 32.0;
    let bar_y0 = (screen_h - bar_height).max(0.0);
    let button_size = 96.0;
    let gap_above_timeline = 4.0;
    let y0 = (bar_y0 - gap_above_timeline - button_size).max(0.0);
    let x0 = gap_above_timeline;
    Rect { x0, y0, x1: x0 + button_size, y1: y0 + button_size }
}

fn compute_stats_box_rect(timeline_height_px: f64) -> Rect {
    let margin = 8.0;
    let text_h = 14.0;
    let adv = (text_h / 7.0) * 6.0;
    let side_padding = 8.0;
    let label_chars = 9.0;
    let value_chars = 8.0;
    let column_gap_chars = 1.0;
    let width = side_padding * 2.0 + adv * (label_chars + column_gap_chars + value_chars) - 2.0;
    let height = 156.0;

    let x0 = margin;
    let y0 = timeline_height_px.max(0.0) + margin;
    let x1 = x0 + width;
    let y1 = y0 + height;
    Rect { x0, y0, x1, y1 }
}

fn compute_volume_box_rects(stats_box_rect: &Rect) -> (Rect, Rect, Rect) {
    let gap = 8.0;
    let box_h = 28.0;
    let box_w = 236.0;
    let x0 = stats_box_rect.x1 + gap;
    let x1 = x0 + box_w;
    let y0 = stats_box_rect.y0;
    let y1 = y0 + box_h;
    let audio = Rect { x0, y0, x1, y1 };
    let hitsounds = Rect {
        x0,
        y0: y1 + gap,
        x1,
        y1: y1 + gap + box_h,
    };
    let playfield = Rect {
        x0,
        y0: hitsounds.y1 + gap,
        x1,
        y1: hitsounds.y1 + gap + box_h,
    };
    (audio, hitsounds, playfield)
}

fn compute_left_right_hitbox_rects(screen_w: f64, screen_h: f64) -> (Rect, Rect) {
    let width = screen_w;
    let height = screen_h;
    let half = width * 0.5;
    (Rect { x0: 0.0, y0: 0.0, x1: half, y1: height }, Rect { x0: half, y0: 0.0, x1: width, y1: height })
}

fn compute_playfield_and_gameplay_rects(
    screen_w: f64,
    screen_h: f64,
    playfield_scale: f64,
) -> (Rect, Rect) {
    const OSU_PLAYFIELD_LEGACY_PADDING: f64 = 8.0;
    const OSU_W: f64 = 640.0;
    const OSU_H: f64 = 480.0;

    let osu_center_x = screen_w * 0.5;
    let osu_center_y = screen_h * 0.5;

    let max_fit = (screen_w / OSU_W).min(screen_h / OSU_H);
    let scale = max_fit * playfield_scale.clamp(0.01, 1.0);

    let playfield_rect = Rect {
        x0: osu_center_x + (-256.0) * scale,
        y0: osu_center_y + (-192.0 + OSU_PLAYFIELD_LEGACY_PADDING) * scale,
        x1: osu_center_x + (256.0) * scale,
        y1: osu_center_y + (192.0 + OSU_PLAYFIELD_LEGACY_PADDING) * scale,
    };

    let osu_rect = Rect {
        x0: osu_center_x - 320.0 * scale,
        y0: osu_center_y - 240.0 * scale,
        x1: osu_center_x + 320.0 * scale,
        y1: osu_center_y + 240.0 * scale,
    };

    return (playfield_rect, osu_rect);
}