use bytemuck::Zeroable;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::config::Config;
use crate::geometry::vec2::Vec2;
use crate::layout;
use crate::map_format::colors::Color;
use crate::skin::{Skin, Texture, load_texture};
use crate::state::Object;
use crate::treap::Treap;

use super::msaa;
use super::textures;
use super::timeline::calculate_timeline_points_and_boxes;
pub use super::types::ObjectInstance;
use super::types::{
    CircleGpu, DigitsMeta, Globals, INITIAL_SLIDER_BOXES_CAPACITY, INITIAL_SLIDER_SEGS_CAPACITY,
    MAX_BOOKMARKS, MAX_BREAK_INTERVALS, MAX_CIRCLES, MAX_KIAI_INTERVALS, MAX_RED_LINES,
    MAX_SNAP_MARKERS, MAX_TIMELINE_MARKS, MAX_TIMELINE_SNAKES, MAX_TIMELINE_X_BOXES, SkinMeta,
    SliderBoxGpu, SliderSegGpu, TimelinePointGpu, TimelineXBoxGpu,
};

pub struct GpuRenderer {
    _window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    circles_pipeline: wgpu::RenderPipeline,
    sliders_pipeline: wgpu::RenderPipeline,
    slider_caps_pipeline: wgpu::RenderPipeline,
    background_pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    hud_pipeline: wgpu::RenderPipeline,
    timeline_kiai_pipeline: wgpu::RenderPipeline,
    timeline_break_pipeline: wgpu::RenderPipeline,
    timeline_bookmark_pipeline: wgpu::RenderPipeline,
    timeline_slider_pipeline: wgpu::RenderPipeline,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    timeline_empty_bind_group: wgpu::BindGroup,
    _demo_texture: wgpu::Texture,
    _demo_texture_view: wgpu::TextureView,
    _demo_overlay_texture: wgpu::Texture,
    _demo_overlay_texture_view: wgpu::TextureView,
    _slider_texture: wgpu::Texture,
    _slider_texture_view: wgpu::TextureView,
    _slider_overlay_texture: wgpu::Texture,
    _slider_overlay_texture_view: wgpu::TextureView,
    _slider_end_texture: wgpu::Texture,
    _slider_end_texture_view: wgpu::TextureView,
    _slider_end_overlay_texture: wgpu::Texture,
    _slider_end_overlay_texture_view: wgpu::TextureView,
    _reverse_arrow_texture: wgpu::Texture,
    _reverse_arrow_texture_view: wgpu::TextureView,
    _slider_ball_texture: wgpu::Texture,
    _slider_ball_texture_view: wgpu::TextureView,
    _slider_follow_circle_texture: wgpu::Texture,
    _slider_follow_circle_texture_view: wgpu::TextureView,
    _approach_circle_texture: wgpu::Texture,
    _approach_circle_texture_view: wgpu::TextureView,
    _background_texture: wgpu::Texture,
    _background_texture_view: wgpu::TextureView,
    _loading_texture: wgpu::Texture,
    _loading_texture_view: wgpu::TextureView,
    _break_texture: wgpu::Texture,
    _break_texture_view: wgpu::TextureView,
    _spinner_texture: wgpu::Texture,
    _spinner_texture_view: wgpu::TextureView,
    _demo_sampler: wgpu::Sampler,
    _digits_texture: wgpu::Texture,
    _digits_texture_view: wgpu::TextureView,
    _digits_meta_buffer: wgpu::Buffer,
    _skin_meta_buffer: wgpu::Buffer,
    texture_bind_group: wgpu::BindGroup,
    msaa_samples: u32,
    msaa_color: Option<wgpu::Texture>,
    msaa_color_view: Option<wgpu::TextureView>,

    timeline_kiai_buffer: wgpu::Buffer,
    timeline_kiai_bind_group: wgpu::BindGroup,
    timeline_break_buffer: wgpu::Buffer,
    timeline_break_bind_group: wgpu::BindGroup,
    timeline_bookmark_buffer: wgpu::Buffer,
    timeline_bookmark_bind_group: wgpu::BindGroup,
    timeline_points_buffer: wgpu::Buffer,
    timeline_points_capacity: usize,
    timeline_points_bind_group: wgpu::BindGroup,
    timeline_x_boxes_buffer: wgpu::Buffer,
    timeline_x_boxes_capacity: usize,
    timeline_x_boxes_bind_group: wgpu::BindGroup,
    snap_markers_buffer: wgpu::Buffer,
    snap_markers_capacity: usize,
    snap_markers_bind_group: wgpu::BindGroup,

    slider_segs_buffer: wgpu::Buffer,
    slider_segs_capacity: usize,
    slider_boxes_buffer: wgpu::Buffer,
    slider_boxes_capacity: usize,
    slider_draw_indices_buffer: wgpu::Buffer,
    slider_draw_indices_capacity: usize,
    slider_bind_group: wgpu::BindGroup,
    objects_buffer: wgpu::Buffer,
    objects_bind_group: wgpu::BindGroup,
    objects_upload: Vec<CircleGpu>,
    size: PhysicalSize<u32>,
    cpu_pass_x10: u32,
    gpu_pass_x10: u32,
    cpu_pass_history: VecDeque<(Instant, u32)>,
    gpu_pass_history: VecDeque<(Instant, u32)>,
}

impl GpuRenderer {
    fn update_recent_peak(
        history: &mut VecDeque<(Instant, u32)>,
        now: Instant,
        value_x10: u32,
        window: Duration,
    ) -> u32 {
        history.push_back((now, value_x10));
        while let Some((ts, _)) = history.front() {
            if now.duration_since(*ts) > window {
                history.pop_front();
            } else {
                break;
            }
        }
        history.iter().map(|(_, value)| *value).max().unwrap_or(0)
    }

    fn upload_texture_2d_srgb(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &'static str,
        tex: &Texture,
        pad_to_nominal: bool,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        textures::upload_texture_2d_srgb(device, queue, label, tex, pad_to_nominal)
    }

    fn upload_texture_2d_array_srgb(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &'static str,
        frames: &[Texture],
        pad_to_nominal: bool,
    ) -> anyhow::Result<(wgpu::Texture, wgpu::TextureView)> {
        textures::upload_texture_2d_array_srgb(device, queue, label, frames, pad_to_nominal)
    }

    fn normalize_msaa_samples(samples: u32) -> u32 {
        msaa::normalize_msaa_samples(samples)
    }

    fn select_supported_msaa_samples(
        adapter: &wgpu::Adapter,
        format: wgpu::TextureFormat,
        requested: u32,
    ) -> u32 {
        msaa::select_supported_msaa_samples(adapter, format, requested)
    }

    fn create_msaa_target(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        samples: u32,
    ) -> (Option<wgpu::Texture>, Option<wgpu::TextureView>) {
        msaa::create_msaa_target(device, surface_config, samples)
    }

    pub fn new(
        window: Arc<Window>,
        editor_config: Config,
        skin: Skin,
        background: Texture,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();

        // SAFETY: wgpu requires the window handle outlive the surface.
        // We keep an `Arc<Window>` inside `GpuRenderer` to guarantee that.
        let surface = instance.create_surface(window.clone())?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .map_err(|e| anyhow::anyhow!("request_adapter failed: {e}"))?;

        let requested_msaa = Self::normalize_msaa_samples(editor_config.performance.msaa_samples);
        let wants_adapter_specific_msaa = requested_msaa != 1 && requested_msaa != 4;

        let mut required_features = wgpu::Features::empty();
        if wants_adapter_specific_msaa
            && adapter
                .features()
                .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
        {
            required_features |= wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        }

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features,
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::default(),
            }))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = if editor_config.performance.prefer_vrr {
            if surface_caps
                .present_modes
                .contains(&wgpu::PresentMode::Mailbox)
            {
                wgpu::PresentMode::Mailbox
            } else {
                // FIFO is always supported and VRR-friendly when available through compositor/driver.
                wgpu::PresentMode::Fifo
            }
        } else if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Immediate)
        {
            wgpu::PresentMode::Immediate
        } else if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Mailbox)
        {
            wgpu::PresentMode::Mailbox
        } else {
            // FIFO is always supported; slightly higher latency but consistent.
            wgpu::PresentMode::Fifo
        };

        let alpha_mode = surface_caps.alpha_modes[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let msaa_samples = Self::select_supported_msaa_samples(
            &adapter,
            config.format,
            editor_config.performance.msaa_samples,
        );
        let (msaa_color, msaa_color_view) =
            Self::create_msaa_target(&device, &config, msaa_samples);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene.wgsl"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("shaders/00_defs.wgsl"),
                    "\n",
                    include_str!("shaders/10_common.wgsl"),
                    "\n",
                    include_str!("shaders/20_bg_hud.wgsl"),
                    "\n",
                    include_str!("shaders/30_circles.wgsl"),
                    "\n",
                    include_str!("shaders/40_sliders.wgsl"),
                    "\n",
                    include_str!("shaders/50_overlay.wgsl"),
                )
                .into(),
            ),
        });

        let playfield_scale = editor_config.general.playfield_scale.clamp(0.0, 1.0);
        let initial_layout = layout::compute_layout(
            config.width as f64,
            config.height as f64,
            playfield_scale,
            editor_config.appearance.layout.timeline_height_percent,
            editor_config
                .appearance
                .layout
                .timeline_second_box_width_percent,
            editor_config
                .appearance
                .layout
                .timeline_third_box_width_percent,
        );

        let timeline_rect = initial_layout.timeline_rect.to_f32_array();
        let timeline_hitbox_rect = initial_layout.timeline_hitbox_rect.to_f32_array();
        let top_timeline_rect = initial_layout.top_timeline_rect.to_f32_array();
        let top_timeline_hitbox_rect = initial_layout.top_timeline_hitbox_rect.to_f32_array();
        let top_timeline_second_rect = initial_layout.top_timeline_second_rect.to_f32_array();
        let top_timeline_second_hitbox_rect = initial_layout
            .top_timeline_second_hitbox_rect
            .to_f32_array();
        let top_timeline_third_rect = initial_layout.top_timeline_third_rect.to_f32_array();
        let top_timeline_third_hitbox_rect =
            initial_layout.top_timeline_third_hitbox_rect.to_f32_array();
        let play_pause_button_rect = initial_layout.play_pause_button_rect.to_f32_array();
        let stats_box_rect = initial_layout.stats_box_rect.to_f32_array();

        let globals = Globals {
            screen_size: [config.width as f32, config.height as f32],
            time_ms: 0.0,
            slider_border_thickness: editor_config.appearance.layout.slider_border_thickness as f32,
            playfield_rect: [0.0, 0.0, 0.0, 0.0],
            osu_rect: [0.0, 0.0, 0.0, 0.0],
            slider_position: [-1000.0, -1000.0],

            playfield_rgba: [0.0, 0.0, 0.0, 0.0],

            gameplay_rgba: [0.0, 0.0, 0.0, 0.0],

            outer_rgba: [0.0, 0.0, 0.0, 0.0],

            playfield_border_rgba: [0.0, 0.0, 0.0, 0.0],
            gameplay_border_rgba: [0.0, 0.0, 0.0, 0.0],
            slider_ridge_rgba: [
                (editor_config.appearance.colors.slider_ridge_rgba[0] / 255.0) as f32,
                (editor_config.appearance.colors.slider_ridge_rgba[1] / 255.0) as f32,
                (editor_config.appearance.colors.slider_ridge_rgba[2] / 255.0) as f32,
                editor_config.appearance.colors.slider_ridge_rgba[3] as f32,
            ],
            slider_body_rgba: [
                (editor_config.appearance.colors.slider_body_rgba[0] / 255.0) as f32,
                (editor_config.appearance.colors.slider_body_rgba[1] / 255.0) as f32,
                (editor_config.appearance.colors.slider_body_rgba[2] / 255.0) as f32,
                editor_config.appearance.colors.slider_body_rgba[3] as f32,
            ],

            break_time_lightness: 0.0,
            is_kiai_time: 0,
            is_break_time: 0,

            slider_progress: 0.0,
            slider_follow_circle_scaling: 0.0,
            slider_ball_rotation_index: -1,
            slider_border_outer_thickness: editor_config.appearance.layout.slider_outer_thickness
                as f32,

            slider_radius: 0.0,
            slider_color: [1.0, 1.0, 1.0],
            slider_ball_direction: [1.0, 0.0],
            fps_x10: 0,
            _pad0: [0, 0, 0],
            fps_low_x10: 0,

            song_total_ms: 0.0,
            playback_rate: 0.0,
            audio_volume: 1.0,
            hitsound_volume: 1.0,
            hud_opacity: 1.0,
            is_playing: 0,
            time_elapsed_ms: 0.0,
            loading: 0,
            break_time: [0.0, 0.0],
            spinner_time: [0.0, 0.0],
            spinner_state: 0,
            undo_count: 0,
            undo_redo_info: [0, 0, 0, 0],
            undo_prev_state_info: [0, 0, 0, 0],
            undo_current_state_info: [0, 0, 0, 0],
            undo_next_states_uuid_0: [0, 0, 0, 0],
            undo_next_states_uuid_1: [0, 0, 0, 0],
            undo_next_states_age_0: [0, 0, 0, 0],
            undo_next_states_age_1: [0, 0, 0, 0],
            undo_next_states_age_unit_0: [0, 0, 0, 0],
            undo_next_states_age_unit_1: [0, 0, 0, 0],
            undo_prev_state_name_meta: [0, 0, 0, 0],
            undo_prev_state_name_packed: [0, 0, 0, 0],
            undo_next_states_name_len_0: [0, 0, 0, 0],
            undo_next_states_name_len_1: [0, 0, 0, 0],
            undo_next_states_name_packed: [[0, 0, 0, 0]; 8],
            undo_button_meta: [0, 0, 0, 0],
            current_state_button_meta: [0, 0, 0, 0],
            current_state_name_meta: [0, 0, 0, 0],
            current_state_name_text_0: [0, 0, 0, 0],
            current_state_name_text_1: [0, 0, 0, 0],
            current_state_name_text_2: [0, 0, 0, 0],
            current_state_name_text_3: [0, 0, 0, 0],
            current_state_name_text_4: [0, 0, 0, 0],
            current_state_name_text_5: [0, 0, 0, 0],
            current_state_name_text_6: [0, 0, 0, 0],
            current_state_name_text_7: [0, 0, 0, 0],
            redo_buttons_meta: [0, 0, 0, 0],
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
            play_pause_button_meta: [0, 0, 0, 0],
            overlay_rect_left: [0.0, 0.0, 0.0, 0.0],
            overlay_rect_right: [0.0, 0.0, 0.0, 0.0],
            selection_quad_left_01: [0.0, 0.0, 0.0, 0.0],
            selection_quad_left_23: [0.0, 0.0, 0.0, 0.0],
            selection_quad_right_01: [0.0, 0.0, 0.0, 0.0],
            selection_quad_right_23: [0.0, 0.0, 0.0, 0.0],
            selection_origin_left: [0.0, 0.0, 0.0, 0.0],
            selection_origin_right: [0.0, 0.0, 0.0, 0.0],
            selection_drag_pos_left: [0.0, 0.0, 0.0, 0.0],
            selection_drag_pos_right: [0.0, 0.0, 0.0, 0.0],
            left_selection_colors: [
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .drag_rectangle[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .drag_rectangle[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .drag_rectangle[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .drag_rectangle[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_hovered[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_hovered[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_hovered[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_hovered[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_dragging[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_dragging[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_dragging[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_dragging[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_hovered[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_hovered[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_hovered[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_hovered[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_dragging[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_dragging[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_dragging[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_dragging[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_hovered[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_hovered[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_hovered[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_hovered[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_clicked[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_clicked[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_clicked[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_clicked[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_locked[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_locked[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_locked[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_locked[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_combo_color[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_combo_color[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_combo_color[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_combo_color[3] as f32,
                ],
            ],
            right_selection_colors: [
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .drag_rectangle[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .drag_rectangle[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .drag_rectangle[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .drag_rectangle[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_hovered[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_hovered[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_hovered[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_hovered[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_dragging[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_dragging[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_dragging[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_dragging[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_hovered[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_hovered[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_hovered[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_hovered[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_dragging[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_dragging[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_dragging[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_dragging[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_hovered[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_hovered[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_hovered[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_hovered[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_clicked[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_clicked[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_clicked[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_clicked[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_locked[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_locked[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_locked[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_locked[3] as f32,
                ],
                [
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_combo_color[0]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_combo_color[1]
                        / 255.0) as f32,
                    (editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_combo_color[2]
                        / 255.0) as f32,
                    editor_config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_combo_color[3] as f32,
                ],
            ],
            overlay_meta: [0, 0, 0, 0],
            selection_meta: [0, 0, 0, 0],
            spinner_selection_meta: [0, 0, 0, 0],
            kiai_interval_count: 0,
            break_interval_count: 0,
            bookmark_count: 0,
            red_line_count: 0,
            cpu_pass_x10: 0,
            gpu_pass_x10: 0,
            cursor_pos: [0.0, 0.0],
            selected_fade_in_opacity_cap: editor_config
                .appearance
                .general
                .selected_fade_in_opacity_cap
                .clamp(0.0, 1.0) as f32,
            selected_fade_out_opacity_cap: editor_config
                .appearance
                .general
                .selected_fade_out_opacity_cap
                .clamp(0.0, 1.0) as f32,
            selection_color_mix_strength: editor_config
                .appearance
                .general
                .selection_color_mix_strength
                .clamp(0.0, 1.0) as f32,
            selection_left_scale: 1.0,
            selection_right_scale: 1.0,
            selection_left_rotation_degrees: 0.0,
            selection_right_rotation_degrees: 0.0,
            _pad4: [0, 0, 0],
            selection_exists_meta: [0, 0, 0, 0],
            selection_origin_left_playfield: [0.0, 0.0],
            selection_origin_right_playfield: [0.0, 0.0],
            selection_moved_left_playfield: [0.0, 0.0],
            selection_moved_right_playfield: [0.0, 0.0],
            selection_lock_meta: [0, 0, 0, 0],
            selection_box_dragging_meta: [0, 0, 0, 0],
            snap_marker_rgba: [
                (editor_config.appearance.colors.snap_marker_rgba[0] / 255.0) as f32,
                (editor_config.appearance.colors.snap_marker_rgba[1] / 255.0) as f32,
                (editor_config.appearance.colors.snap_marker_rgba[2] / 255.0) as f32,
                editor_config.appearance.colors.snap_marker_rgba[3] as f32,
            ],
            snap_marker_style: [
                editor_config.appearance.layout.snap_marker_radius_px as f32,
                0.0,
                0.0,
                0.0,
            ],
            movable_snap_marker_rgba: [
                (editor_config.appearance.colors.movable_snap_hitbox_rgba[0] / 255.0) as f32,
                (editor_config.appearance.colors.movable_snap_hitbox_rgba[1] / 255.0) as f32,
                (editor_config.appearance.colors.movable_snap_hitbox_rgba[2] / 255.0) as f32,
                editor_config.appearance.colors.movable_snap_hitbox_rgba[3] as f32,
            ],
            movable_snap_marker_style: [
                editor_config
                    .appearance
                    .layout
                    .movable_snap_hitbox_radius_px as f32,
                0.0,
                0.0,
                0.0,
            ],
            snap_meta: [0, 0, 0, 0],
            drag_state_marker_rgba: [
                (editor_config.appearance.colors.drag_state_marker_rgba[0] / 255.0) as f32,
                (editor_config.appearance.colors.drag_state_marker_rgba[1] / 255.0) as f32,
                (editor_config.appearance.colors.drag_state_marker_rgba[2] / 255.0) as f32,
                editor_config.appearance.colors.drag_state_marker_rgba[3] as f32,
            ],
            drag_state_marker_style: [
                editor_config
                    .appearance
                    .layout
                    .drag_state_marker_radius_px
                    .max(0.0) as f32,
                0.0,
                0.0,
                0.0,
            ],
            offscreen_playfield_tint_rgba: [
                (editor_config.appearance.colors.offscreen_playfield_tint_rgb[0] / 255.0) as f32,
                (editor_config.appearance.colors.offscreen_playfield_tint_rgb[1] / 255.0) as f32,
                (editor_config.appearance.colors.offscreen_playfield_tint_rgb[2] / 255.0) as f32,
                1.0,
            ],
            offscreen_osu_tint_rgba: [
                (editor_config.appearance.colors.offscreen_osu_tint_rgb[0] / 255.0) as f32,
                (editor_config.appearance.colors.offscreen_osu_tint_rgb[1] / 255.0) as f32,
                (editor_config.appearance.colors.offscreen_osu_tint_rgb[2] / 255.0) as f32,
                1.0,
            ],
            timeline_window_ms: [0.0, 0.0],
            timeline_current_x: 0.0,
            timeline_zoom: 1.0,
            timeline_object_meta: [0, 0, 0, 0],
            timeline_style: [0.0, 0.0, 0.0, 0.0],
            timeline_slider_outline_rgba: [0.0, 0.0, 0.0, 0.0],
            timeline_slider_head_body_rgba: [0.0, 0.0, 0.0, 0.0],
            timeline_slider_head_overlay_rgba: [0.0, 0.0, 0.0, 0.0],
            timeline_circle_head_body_rgba: [0.0, 0.0, 0.0, 0.0],
            timeline_circle_head_overlay_rgba: [0.0, 0.0, 0.0, 0.0],
            timeline_slider_head_point_rgba: [0.0, 0.0, 0.0, 0.0],
            timeline_slider_repeat_point_rgba: [0.0, 0.0, 0.0, 0.0],
            timeline_slider_end_point_rgba: [0.0, 0.0, 0.0, 0.0],
            timeline_past_grayscale_strength: 0.0,
            _timeline_past_pad: [0.0, 0.0, 0.0],
            timeline_past_tint_rgba: [0.0, 0.0, 0.0, 0.0],
            timeline_past_object_tint_rgba: [0.0, 0.0, 0.0, 0.0],
            _pad_end: [0.0, 0.0, 0.0, 0.0],
        };

        let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("globals"),
            contents: bytemuck::bytes_of(&globals),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("globals layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals bind group"),
            layout: &globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
        });

        let timeline_marks_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("timeline marks layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let snap_markers_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("snap markers layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let timeline_kiai_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timeline kiai buffer"),
            size: (MAX_KIAI_INTERVALS * std::mem::size_of::<[f32; 2]>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let timeline_break_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timeline break buffer"),
            size: (MAX_BREAK_INTERVALS * std::mem::size_of::<[f32; 2]>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let timeline_bookmark_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timeline bookmark buffer"),
            size: (MAX_TIMELINE_MARKS * std::mem::size_of::<[f32; 2]>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let snap_markers_capacity = MAX_SNAP_MARKERS.max(1);
        let snap_markers_init: Vec<[f32; 2]> = vec![[0.0, 0.0]; snap_markers_capacity];
        let snap_markers_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("snap markers buffer"),
            contents: bytemuck::cast_slice(snap_markers_init.as_slice()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let timeline_kiai_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("timeline kiai bind group"),
            layout: &timeline_marks_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 3,
                resource: timeline_kiai_buffer.as_entire_binding(),
            }],
        });
        let timeline_break_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("timeline break bind group"),
            layout: &timeline_marks_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 3,
                resource: timeline_break_buffer.as_entire_binding(),
            }],
        });
        let timeline_bookmark_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("timeline bookmark bind group"),
            layout: &timeline_marks_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 3,
                resource: timeline_bookmark_buffer.as_entire_binding(),
            }],
        });

        let timeline_points_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("timeline points layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let timeline_x_boxes_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("timeline x boxes layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let timeline_points_capacity = MAX_TIMELINE_SNAKES.max(1);
        let timeline_points_init: Vec<TimelinePointGpu> =
            vec![TimelinePointGpu::zeroed(); timeline_points_capacity];
        let timeline_points_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("timeline points buffer"),
            contents: bytemuck::cast_slice(timeline_points_init.as_slice()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let timeline_x_boxes_capacity = MAX_TIMELINE_X_BOXES.max(1);
        let timeline_x_boxes_init: Vec<TimelineXBoxGpu> =
            vec![TimelineXBoxGpu::zeroed(); timeline_x_boxes_capacity];
        let timeline_x_boxes_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("timeline x boxes buffer"),
                contents: bytemuck::cast_slice(timeline_x_boxes_init.as_slice()),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let timeline_points_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("timeline points bind group"),
            layout: &timeline_points_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 1,
                resource: timeline_points_buffer.as_entire_binding(),
            }],
        });

        let timeline_x_boxes_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("timeline x boxes bind group"),
            layout: &timeline_x_boxes_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 4,
                resource: timeline_x_boxes_buffer.as_entire_binding(),
            }],
        });

        let snap_markers_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("snap markers bind group"),
            layout: &snap_markers_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 4,
                resource: snap_markers_buffer.as_entire_binding(),
            }],
        });

        let hitcircle = skin.hit_circle;
        let hitcircleoverlay = skin.hit_circle_overlay;
        let slidercircle = skin.slider_start_circle;
        let slidercircleoverlay = skin.slider_start_circle_overlay;
        let sliderendcircle = skin.sliderend_circle;
        let sliderendcircleoverlay = skin.sliderend_circle_overlay;
        let reverse_arrow = skin.reverse_arrow;
        let slider_ball = skin.slider_ball;
        let slider_follow_circle = skin.slider_follow_circle;
        let approachcircle = skin.approach_circle;
        let skin_spinner_circle = skin.spinner_circle;
        let numbers = skin.numbers;

        let hitcircle_w = hitcircle.width;
        let hitcircle_h = hitcircle.height;

        // osu! effectively treats undersized skin sprites as if they were padded to the nominal
        // resolution (128px for 1x, 256px for @2x). That means they should *not* be scaled down;
        // instead they get transparent padding.
        let tex_scale = |tex: &Texture| -> f32 {
            let nominal_px: u32 = if tex.is_2x { 256 } else { 128 };
            let px = tex.width.max(tex.height).max(nominal_px).max(1) as f32;
            (px / nominal_px as f32).max(1.0)
        };
        let anim_tex_scale =
            |texs: &[Texture]| -> f32 { texs.iter().map(&tex_scale).fold(1.0, f32::max) };

        let skin_meta = SkinMeta {
            hitcircle_scale: tex_scale(&hitcircle),
            hitcircleoverlay_scale: tex_scale(&hitcircleoverlay),
            sliderstartcircle_scale: tex_scale(&slidercircle),
            sliderstartcircleoverlay_scale: tex_scale(&slidercircleoverlay),
            sliderendcircle_scale: tex_scale(&sliderendcircle),
            sliderendcircleoverlay_scale: tex_scale(&sliderendcircleoverlay),
            reversearrow_scale: tex_scale(&reverse_arrow),
            sliderball_scale: anim_tex_scale(&slider_ball),
            sliderfollowcircle_scale: tex_scale(&slider_follow_circle),
            _pad0: 0.0,
            _pad: [0.0, 0.0],
        };

        let (hitcircle_texture, demo_texture_view) =
            Self::upload_texture_2d_srgb(&device, &queue, "hitcircle texture", &hitcircle, true);
        let (hitcircleoverlay_texture, demo_overlay_texture_view) = Self::upload_texture_2d_srgb(
            &device,
            &queue,
            "hitcircle overlay texture",
            &hitcircleoverlay,
            true,
        );
        let (slidercircle_texture, slidercircle_texture_view) = Self::upload_texture_2d_srgb(
            &device,
            &queue,
            "slider start circle texture",
            &slidercircle,
            true,
        );
        let (slidercircleoverlay_texture, slidercircleoverlay_texture_view) =
            Self::upload_texture_2d_srgb(
                &device,
                &queue,
                "slider start circle overlay texture",
                &slidercircleoverlay,
                true,
            );
        let (sliderendcircle_texture, sliderendcircle_texture_view) = Self::upload_texture_2d_srgb(
            &device,
            &queue,
            "slider end circle texture",
            &sliderendcircle,
            true,
        );
        let (sliderendcircleoverlay_texture, sliderendcircleoverlay_texture_view) =
            Self::upload_texture_2d_srgb(
                &device,
                &queue,
                "slider end circle overlay texture",
                &sliderendcircleoverlay,
                true,
            );
        let (reverse_arrow_texture, reverse_arrow_texture_view) = Self::upload_texture_2d_srgb(
            &device,
            &queue,
            "reverse arrow texture",
            &reverse_arrow,
            true,
        );

        let (slider_ball_texture, slider_ball_texture_view) = Self::upload_texture_2d_array_srgb(
            &device,
            &queue,
            "sliderball texture array",
            slider_ball.as_slice(),
            true,
        )?;

        let (slider_follow_circle_texture, slider_follow_circle_texture_view) =
            Self::upload_texture_2d_array_srgb(
                &device,
                &queue,
                "sliderfollowcircle texture array",
                std::slice::from_ref(&slider_follow_circle),
                true,
            )?;
        let (approachcircle_texture, approachcircle_texture_view) = Self::upload_texture_2d_srgb(
            &device,
            &queue,
            "approach circle texture",
            &approachcircle,
            true,
        );
        let (background_texture, background_texture_view) =
            Self::upload_texture_2d_srgb(&device, &queue, "background texture", &background, false);

        let loading = match std::fs::read("assets/loading.png") {
            Ok(bytes) => load_texture(&bytes).unwrap_or(Texture {
                rgba: vec![0, 0, 0, 0],
                width: 1,
                height: 1,
                is_2x: false,
            }),
            Err(err) => {
                log!("Failed to read assets/loading.png: {err}");
                Texture {
                    rgba: vec![0, 0, 0, 0],
                    width: 1,
                    height: 1,
                    is_2x: false,
                }
            }
        };
        let (loading_texture, loading_texture_view) =
            Self::upload_texture_2d_srgb(&device, &queue, "loading texture", &loading, true);

        let break_tex = match std::fs::read("assets/break.png") {
            Ok(bytes) => load_texture(&bytes).unwrap_or(Texture {
                rgba: vec![0, 0, 0, 0],
                width: 1,
                height: 1,
                is_2x: false,
            }),
            Err(err) => {
                log!("Failed to read assets/break.png: {err}");
                Texture {
                    rgba: vec![0, 0, 0, 0],
                    width: 1,
                    height: 1,
                    is_2x: false,
                }
            }
        };
        let (break_texture, break_texture_view) =
            Self::upload_texture_2d_srgb(&device, &queue, "break texture", &break_tex, true);

        let spinner_tex = {
            if !skin_spinner_circle.rgba.is_empty() {
                skin_spinner_circle
            } else {
                let spinner_2x = "assets/spinner-circle@2x.png";
                let spinner_1x = "assets/spinner-circle.png";

                let try_load = |path: &str| -> Option<Texture> {
                    match std::fs::read(path) {
                        Ok(bytes) => load_texture(&bytes),
                        Err(_) => None,
                    }
                };

                if let Some(tex) = try_load(spinner_2x) {
                    tex
                } else if let Some(tex) = try_load(spinner_1x) {
                    tex
                } else {
                    log!(
                        "Failed to read spinner texture from skin and assets: tried {spinner_2x} and {spinner_1x}"
                    );
                    Texture {
                        rgba: vec![0, 0, 0, 0],
                        width: 1,
                        height: 1,
                        is_2x: false,
                    }
                }
            }
        };
        let (spinner_texture, spinner_texture_view) =
            Self::upload_texture_2d_srgb(&device, &queue, "spinner texture", &spinner_tex, true);
        let demo_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("demo sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // --- Digit textures (0-9) as a 2D array texture ---
        // Skins sometimes have different pixel sizes per digit (e.g. proportional fonts).
        // `texture_2d_array` requires a single (w,h) for all layers, so we pad each digit into
        // a max-sized layer and send UV transforms to sample the digit region without stretching.
        const DIGIT_CROP_MAX_PX: u32 = 128;
        struct DigitUpload {
            w: u32,
            h: u32,
            rgba: Vec<u8>,
        }
        let mut digits_max_w: u32 = 0;
        let mut digits_max_h: u32 = 0;
        let mut digit_uploads: Vec<DigitUpload> = Vec::with_capacity(numbers.len());
        for (i, tex) in numbers.iter().enumerate() {
            let w = tex.width;
            let h = tex.height;
            if w == 0 || h == 0 {
                anyhow::bail!("skin digit texture default-{i} has invalid size (0x0)");
            }

            let crop_w = w.min(DIGIT_CROP_MAX_PX);
            let crop_h = h.min(DIGIT_CROP_MAX_PX);
            let crop_x = (w - crop_w) / 2;
            let crop_y = (h - crop_h) / 2;

            let rgba = if crop_w == w && crop_h == h {
                tex.rgba.clone()
            } else {
                let mut cropped = vec![0u8; (crop_w * crop_h * 4) as usize];
                for y in 0..crop_h {
                    let src_y = y + crop_y;
                    let src_row = ((src_y * w + crop_x) * 4) as usize;
                    let dst_row = (y * crop_w * 4) as usize;
                    let row_len = (crop_w * 4) as usize;
                    cropped[dst_row..(dst_row + row_len)]
                        .copy_from_slice(&tex.rgba[src_row..(src_row + row_len)]);
                }
                cropped
            };

            digits_max_w = digits_max_w.max(crop_w);
            digits_max_h = digits_max_h.max(crop_h);
            digit_uploads.push(DigitUpload {
                w: crop_w,
                h: crop_h,
                rgba,
            });
        }

        // Osu expects numbers to be authored at the same resolution as hitcircle (128 or 256 for @2x).
        // If a skin's digit image is smaller, it is effectively padded; we emulate that by making the
        // array layer size at least the hitcircle resolution.
        let digits_layer_w = digits_max_w.max(hitcircle_w);
        let digits_layer_h = digits_max_h.max(hitcircle_h);

        let digits_tex_size = wgpu::Extent3d {
            width: digits_layer_w,
            height: digits_layer_h,
            depth_or_array_layers: 10,
        };

        let digits_tex_desc = wgpu::TextureDescriptor {
            label: Some("digits texture array"),
            size: digits_tex_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let digits_texture = device.create_texture(&digits_tex_desc);

        // Clear each layer to transparent (texture contents are otherwise undefined).
        let clear_rgba = vec![0u8; (digits_layer_w * digits_layer_h * 4) as usize];
        for (layer, tex) in digit_uploads.iter().enumerate() {
            let rgba = tex.rgba.as_slice();
            let w = tex.w;
            let h = tex.h;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &digits_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &clear_rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * digits_layer_w),
                    rows_per_image: Some(digits_layer_h),
                },
                wgpu::Extent3d {
                    width: digits_layer_w,
                    height: digits_layer_h,
                    depth_or_array_layers: 1,
                },
            );

            let x_off_px = (digits_layer_w - w) / 2;
            let y_off_px = (digits_layer_h - h) / 2;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &digits_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: x_off_px,
                        y: y_off_px,
                        z: layer as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * w),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
        }

        let digits_texture_view = digits_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("digits texture array view"),
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(10),
            ..Default::default()
        });

        // Per-digit UV transform for sampling: uv' = uv * scale + offset.
        // Stored as 10x vec4<f32> (scale.xy, offset.zw) to keep 16-byte alignment.
        let mut digits_uv_xform = [[0.0f32; 4]; 10];
        for (i, tex) in digit_uploads.iter().enumerate() {
            let w = tex.w;
            let h = tex.h;
            let scale_x = w as f32 / digits_layer_w as f32;
            let scale_y = h as f32 / digits_layer_h as f32;
            let off_x = ((digits_layer_w - w) / 2) as f32 / digits_layer_w as f32;
            let off_y = ((digits_layer_h - h) / 2) as f32 / digits_layer_h as f32;
            digits_uv_xform[i] = [scale_x, scale_y, off_x, off_y];
        }

        let digits_meta = DigitsMeta {
            uv_xform: digits_uv_xform,
            max_size_px: [digits_layer_w as f32, digits_layer_h as f32],
            _pad: [0.0, 0.0],
        };

        let digits_meta_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("digits meta"),
            contents: bytemuck::bytes_of(&digits_meta),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let skin_meta_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("skin meta"),
            contents: bytemuck::bytes_of(&skin_meta),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 8,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 9,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 10,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 11,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 12,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 13,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 14,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 15,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 16,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 17,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                ],
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture bind group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&demo_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&demo_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&demo_overlay_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&slidercircle_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&slidercircleoverlay_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&approachcircle_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&digits_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: digits_meta_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(&background_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: skin_meta_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::TextureView(&sliderendcircle_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 11,
                    resource: wgpu::BindingResource::TextureView(
                        &sliderendcircleoverlay_texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 12,
                    resource: wgpu::BindingResource::TextureView(&reverse_arrow_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 13,
                    resource: wgpu::BindingResource::TextureView(&slider_ball_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 14,
                    resource: wgpu::BindingResource::TextureView(
                        &slider_follow_circle_texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 15,
                    resource: wgpu::BindingResource::TextureView(&loading_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 16,
                    resource: wgpu::BindingResource::TextureView(&break_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 17,
                    resource: wgpu::BindingResource::TextureView(&spinner_texture_view),
                },
            ],
        });

        // --- Circles instance data (from Rust -> shader) ---
        let circles_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("circles layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let circles_init: Vec<CircleGpu> = vec![CircleGpu::zeroed(); MAX_CIRCLES];
        let circles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("circles buffer"),
            contents: bytemuck::cast_slice(circles_init.as_slice()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let circles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("circles bind group"),
            layout: &circles_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: circles_buffer.as_entire_binding(),
            }],
        });

        let background_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("background pipeline layout"),
                bind_group_layouts: &[&globals_bind_group_layout, &texture_bind_group_layout],
                immediate_size: 0,
            });

        let overlay_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("overlay pipeline layout"),
                bind_group_layouts: &[
                    &globals_bind_group_layout,
                    &texture_bind_group_layout,
                    &snap_markers_bind_group_layout,
                ],
                immediate_size: 0,
            });

        let circles_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("circles pipeline layout"),
                bind_group_layouts: &[
                    &globals_bind_group_layout,
                    &texture_bind_group_layout,
                    &circles_bind_group_layout,
                ],
                immediate_size: 0,
            });

        // --- Slider path data (from Rust -> shader) ---
        let slider_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("slider data layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let slider_segs_capacity = INITIAL_SLIDER_SEGS_CAPACITY.max(1);
        let slider_segs_init: Vec<SliderSegGpu> =
            vec![SliderSegGpu::zeroed(); slider_segs_capacity];
        let slider_segs_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slider segs buffer"),
            contents: bytemuck::cast_slice(slider_segs_init.as_slice()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let slider_boxes_capacity = INITIAL_SLIDER_BOXES_CAPACITY.max(1);
        let slider_boxes_init: Vec<SliderBoxGpu> =
            vec![SliderBoxGpu::zeroed(); slider_boxes_capacity];
        let slider_boxes_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slider boxes buffer"),
            contents: bytemuck::cast_slice(slider_boxes_init.as_slice()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let slider_draw_indices_capacity = MAX_CIRCLES.max(1);
        let slider_draw_indices_init: Vec<u32> = vec![0u32; slider_draw_indices_capacity];
        let slider_draw_indices_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("slider draw indices buffer"),
                contents: bytemuck::cast_slice(slider_draw_indices_init.as_slice()),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let slider_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("slider data bind group"),
            layout: &slider_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: slider_segs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: slider_boxes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: slider_draw_indices_buffer.as_entire_binding(),
                },
            ],
        });

        let sliders_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sliders pipeline layout"),
                bind_group_layouts: &[
                    &globals_bind_group_layout,
                    &texture_bind_group_layout,
                    &circles_bind_group_layout,
                    &slider_bind_group_layout,
                ],
                immediate_size: 0,
            });

        let empty_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("empty layout"),
                entries: &[],
            });

        let timeline_empty_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("timeline empty bind group"),
            layout: &empty_bind_group_layout,
            entries: &[],
        });

        let timeline_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("timeline pipeline layout"),
                bind_group_layouts: &[
                    &globals_bind_group_layout,
                    &empty_bind_group_layout,
                    &empty_bind_group_layout,
                    &timeline_marks_bind_group_layout,
                ],
                immediate_size: 0,
            });

        let timeline_slider_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("timeline slider pipeline layout"),
                bind_group_layouts: &[
                    &globals_bind_group_layout,
                    &empty_bind_group_layout,
                    &timeline_points_bind_group_layout,
                    &timeline_x_boxes_bind_group_layout,
                ],
                immediate_size: 0,
            });

        let background_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("background pipeline"),
            layout: Some(&background_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_bg"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_bg"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: msaa_samples,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let overlay_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay pipeline"),
            layout: Some(&overlay_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_overlay"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_overlay"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: msaa_samples,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud pipeline"),
            layout: Some(&overlay_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_hud"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_hud"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: msaa_samples,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let timeline_kiai_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("timeline kiai pipeline"),
                layout: Some(&timeline_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_hud"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_timeline_kiai"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: msaa_samples,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            });

        let timeline_break_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("timeline break pipeline"),
                layout: Some(&timeline_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_hud"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_timeline_break"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: msaa_samples,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            });

        let timeline_bookmark_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("timeline bookmark pipeline"),
                layout: Some(&timeline_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_hud"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_timeline_bookmarks"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: msaa_samples,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            });

        let timeline_slider_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("timeline slider pipeline"),
                layout: Some(&timeline_slider_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_timeline_slider_boxes"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_timeline_slider_boxes"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: msaa_samples,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            });

        let sliders_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sliders body pipeline"),
            layout: Some(&sliders_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_slider_box"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_slider_box"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: msaa_samples,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let slider_caps_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("slider caps pipeline"),
            layout: Some(&sliders_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_slider_caps"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_slider_caps"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: msaa_samples,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let circles_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("circles pipeline"),
            layout: Some(&circles_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: msaa_samples,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        Ok(Self {
            _window: window,
            surface,
            device,
            queue,
            config,
            circles_pipeline,
            sliders_pipeline,
            slider_caps_pipeline,
            background_pipeline,
            overlay_pipeline,
            hud_pipeline,
            timeline_kiai_pipeline,
            timeline_break_pipeline,
            timeline_bookmark_pipeline,
            timeline_slider_pipeline,
            globals_buffer,
            globals_bind_group,
            timeline_empty_bind_group,
            _demo_texture: hitcircle_texture,
            _demo_texture_view: demo_texture_view,
            _demo_overlay_texture: hitcircleoverlay_texture,
            _demo_overlay_texture_view: demo_overlay_texture_view,
            _slider_texture: slidercircle_texture,
            _slider_texture_view: slidercircle_texture_view,
            _slider_overlay_texture: slidercircleoverlay_texture,
            _slider_overlay_texture_view: slidercircleoverlay_texture_view,
            _slider_end_texture: sliderendcircle_texture,
            _slider_end_texture_view: sliderendcircle_texture_view,
            _slider_end_overlay_texture: sliderendcircleoverlay_texture,
            _slider_end_overlay_texture_view: sliderendcircleoverlay_texture_view,
            _reverse_arrow_texture: reverse_arrow_texture,
            _reverse_arrow_texture_view: reverse_arrow_texture_view,
            _slider_ball_texture: slider_ball_texture,
            _slider_ball_texture_view: slider_ball_texture_view,
            _slider_follow_circle_texture: slider_follow_circle_texture,
            _slider_follow_circle_texture_view: slider_follow_circle_texture_view,
            _approach_circle_texture: approachcircle_texture,
            _approach_circle_texture_view: approachcircle_texture_view,
            _background_texture: background_texture,
            _background_texture_view: background_texture_view,
            _loading_texture: loading_texture,
            _loading_texture_view: loading_texture_view,
            _break_texture: break_texture,
            _break_texture_view: break_texture_view,
            _spinner_texture: spinner_texture,
            _spinner_texture_view: spinner_texture_view,
            _demo_sampler: demo_sampler,
            _digits_texture: digits_texture,
            _digits_texture_view: digits_texture_view,
            _digits_meta_buffer: digits_meta_buffer,
            _skin_meta_buffer: skin_meta_buffer,
            texture_bind_group,
            msaa_samples,
            msaa_color,
            msaa_color_view,

            timeline_kiai_buffer,
            timeline_kiai_bind_group,
            timeline_break_buffer,
            timeline_break_bind_group,
            timeline_bookmark_buffer,
            timeline_bookmark_bind_group,
            timeline_points_buffer,
            timeline_points_capacity,
            timeline_points_bind_group,
            timeline_x_boxes_buffer,
            timeline_x_boxes_capacity,
            timeline_x_boxes_bind_group,
            snap_markers_buffer,
            snap_markers_capacity,
            snap_markers_bind_group,

            slider_segs_buffer,
            slider_segs_capacity,
            slider_boxes_buffer,
            slider_boxes_capacity,
            slider_draw_indices_buffer,
            slider_draw_indices_capacity,
            slider_bind_group,
            objects_buffer: circles_buffer,
            objects_bind_group: circles_bind_group,
            objects_upload: circles_init,
            size,
            cpu_pass_x10: 0,
            gpu_pass_x10: 0,
            cpu_pass_history: VecDeque::new(),
            gpu_pass_history: VecDeque::new(),
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            self.size = new_size;
            return;
        }

        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);

        let (msaa_color, msaa_color_view) =
            Self::create_msaa_target(&self.device, &self.config, self.msaa_samples);
        self.msaa_color = msaa_color;
        self.msaa_color_view = msaa_color_view;
    }

    pub fn render<'a>(
        &mut self,
        layout: &layout::Layout,
        objects: &Treap<Object>,
        combo_colors: &[Color],
        break_times: &Treap<(f64, f64)>,
        kiai_times: &Treap<(f64, f64)>,
        bookmarks: &Treap<f64>,
        red_lines: &Treap<f64>,
        left_selected_objects: &[usize],
        right_selected_objects: &[usize],
        time_ms: f64,
        song_total_ms: f64,
        time_elapsed_ms: f64,
        undo_count: usize,
        undo_prev_state: Option<(u32, u32, u32)>,
        undo_prev_state_display_name: Option<String>,
        undo_current_state: (u32, u32, u32),
        undo_current_state_display_name: Option<String>,
        current_state_rename_active: bool,
        current_state_rename_text: &str,
        undo_next_states: &[(u32, u32, u32)],
        undo_next_state_display_names: &[Option<String>],
        fps: f64,
        fps_low: f64,
        playback_rate: f64,
        audio_volume: f64,
        hitsound_volume: f64,
        config: &Config,
        is_playing: bool,
        is_loading: bool,
        overlay_rect_left: Option<[f32; 4]>,
        overlay_rect_right: Option<[f32; 4]>,
        selection_rect_left: Option<[[f32; 2]; 4]>,
        selection_rect_right: Option<[[f32; 2]; 4]>,
        selection_origin_left: Option<[f32; 2]>,
        selection_origin_right: Option<[f32; 2]>,
        selection_drag_pos_left: Option<[f32; 2]>,
        selection_drag_pos_right: Option<[f32; 2]>,
        selection_origin_left_playfield: Option<[f32; 2]>,
        selection_origin_right_playfield: Option<[f32; 2]>,
        selection_moved_left_playfield: [f32; 2],
        selection_moved_right_playfield: [f32; 2],
        selection_left_bbox_hovered: bool,
        selection_right_bbox_hovered: bool,
        selection_left_bbox_dragging: bool,
        selection_right_bbox_dragging: bool,
        selection_left_origin_hovered: bool,
        selection_right_origin_hovered: bool,
        selection_left_origin_dragging: bool,
        selection_right_origin_dragging: bool,
        cursor_pos: [f32; 2],
        play_pause_button_hovered: bool,
        play_pause_button_clicked: bool,
        undo_button_hovered: bool,
        undo_button_clicked: bool,
        current_state_button_hovered: bool,
        current_state_button_clicked: bool,
        redo_button_hovered_row: Option<u32>,
        redo_button_clicked_row: Option<u32>,
        left_selection_exists: bool,
        right_selection_exists: bool,
        left_selection_scale: f64,
        right_selection_scale: f64,
        left_selection_rotation_degrees: f64,
        right_selection_rotation_degrees: f64,
        left_selection_origin_locked: bool,
        right_selection_origin_locked: bool,
        left_selection_scale_locked: bool,
        right_selection_scale_locked: bool,
        snap_positions: &[Vec2],
        movable_snap_positions: &[Vec2],
        drag_happening: bool,
        timeline_zoom: f64,
    ) -> Result<(), wgpu::SurfaceError> {
        let frame_start = Instant::now();
        let output = self.surface.get_current_texture()?;
        let swapchain_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let playfield_rect = layout.playfield_rect.to_f32_array();
        let gameplay_rect = layout.gameplay_rect.to_f32_array();

        // --- Cull objects before uploading ---
        // Approach circles expand the shaded quad significantly (e.g. start scale=4x => 16x area).
        // Uploading/drawing circles that are not currently visible wastes fill-rate and texture
        // bandwidth. We cull by time window and a conservative on-screen bounds check.
        const IGNORE_CIRCLES_DELTA: f64 = 200.0;
        const FADE_OUT_MS: f64 = 250.0;

        let circles_to_upload = &mut self.objects_upload;
        let mut count: usize = 0;

        let mut slider_segs: Vec<SliderSegGpu> = Vec::new();
        let mut slider_boxes: Vec<SliderBoxGpu> = Vec::new();
        let mut slider_draw_indices: Vec<u32> = Vec::new();
        let mut slider_draw_lookup: Vec<i32> = Vec::new();

        let mut current_slider_progress = 0.0;
        let mut current_slider_position = Vec2 { x: 0.0, y: 0.0 };
        let mut current_slider_follow_circle_scaling = 0.0;
        let mut current_slider_radius = 0.0;
        let mut current_slider_ball_direction = Vec2 { x: 1.0, y: 0.0 };
        let mut current_slider_ball_rotation_index = -1;
        let mut current_slider_color = [0.0, 0.0, 0.0];

        let timeline_zoom = timeline_zoom.clamp(0.1, 10.0);
        let top_timeline_height_px =
            (layout.top_timeline_rect.y1 - layout.top_timeline_rect.y0).max(1.0);
        let top_timeline_width_px =
            (layout.top_timeline_rect.x1 - layout.top_timeline_rect.x0).max(1.0);
        let timeline_radius_px = (top_timeline_height_px
            * config
                .appearance
                .timeline
                .object_radius_height_percent
                .max(0.0)
                .clamp(0.0, 1.0))
        .max(1.0);
        let timeline_ms_per_radius = config
            .appearance
            .timeline
            .milliseconds_per_object_radius
            .max(1.0)
            / timeline_zoom;
        let timeline_window_span_ms =
            ((top_timeline_width_px / timeline_radius_px) * timeline_ms_per_radius).max(1.0);
        let timeline_ms_per_pixel = timeline_window_span_ms / top_timeline_width_px;
        let timeline_current_pos = config
            .appearance
            .timeline
            .current_timestamp_position_percent
            .clamp(0.0, 1.0);
        let timeline_window_start_ms = time_ms - timeline_window_span_ms * timeline_current_pos;
        let timeline_window_end_ms = timeline_window_start_ms + timeline_window_span_ms;
        let timeline_window_ms = [
            timeline_window_start_ms as f32,
            timeline_window_end_ms as f32,
        ];
        let timeline_current_x =
            layout.top_timeline_rect.x0 + top_timeline_width_px * timeline_current_pos;

        let left_selected_set: HashSet<usize> = left_selected_objects.iter().copied().collect();
        let right_selected_set: HashSet<usize> = right_selected_objects.iter().copied().collect();

        let outline_thickness_px =
            timeline_radius_px * config.appearance.timeline.slider_outline_thickness_percent;

        let (timeline_points_cpu, timeline_x_boxes_cpu) = calculate_timeline_points_and_boxes(
            objects,
            layout.top_timeline_rect.x0,
            timeline_current_x,
            layout.top_timeline_rect.x1,
            timeline_ms_per_pixel,
            time_ms,
            &left_selected_set,
            &right_selected_set,
            timeline_radius_px,
            outline_thickness_px,
            combo_colors,
        );
        let timeline_center_y =
            (layout.top_timeline_rect.y0 + layout.top_timeline_rect.y1) as f32 * 0.5;
        let timeline_points_gpu: Vec<TimelinePointGpu> = timeline_points_cpu
            .iter()
            .map(|p| TimelinePointGpu {
                x: p.x,
                center_y: timeline_center_y,
                radius_px: timeline_radius_px as f32,
                is_slide_start: p.is_object_start,
                is_slide_end: p.is_object_end,
                is_slide_repeat: p.is_slide_repeat,
                is_selected: p.is_selected,
                is_selected_by_left: if p.selection_side == 1 { 1 } else { 0 },
                is_slider_or_spinner: p.is_slider_or_spinner,
                _pad: [0, 0, 0],
                color: p.combo_color_and_opacity,
            })
            .collect();
        let timeline_x_boxes_gpu: Vec<TimelineXBoxGpu> = timeline_x_boxes_cpu
            .iter()
            .map(|b| TimelineXBoxGpu {
                x1: b.x0,
                x2: b.x1,
                segment_start: b.points_index,
                segment_count: b.point_count,
            })
            .collect();

        let left_selection_rgb_u8 = [
            config
                .appearance
                .colors
                .left_selection_colors
                .selection_border[0]
                .round()
                .clamp(0.0, 255.0) as u32,
            config
                .appearance
                .colors
                .left_selection_colors
                .selection_border[1]
                .round()
                .clamp(0.0, 255.0) as u32,
            config
                .appearance
                .colors
                .left_selection_colors
                .selection_border[2]
                .round()
                .clamp(0.0, 255.0) as u32,
        ];
        let right_selection_rgb_u8 = [
            config
                .appearance
                .colors
                .right_selection_colors
                .selection_border[0]
                .round()
                .clamp(0.0, 255.0) as u32,
            config
                .appearance
                .colors
                .right_selection_colors
                .selection_border[1]
                .round()
                .clamp(0.0, 255.0) as u32,
            config
                .appearance
                .colors
                .right_selection_colors
                .selection_border[2]
                .round()
                .clamp(0.0, 255.0) as u32,
        ];
        let left_selection_rgb = [
            left_selection_rgb_u8[0] as f64 / 255.0,
            left_selection_rgb_u8[1] as f64 / 255.0,
            left_selection_rgb_u8[2] as f64 / 255.0,
        ];
        let right_selection_rgb = [
            right_selection_rgb_u8[0] as f64 / 255.0,
            right_selection_rgb_u8[1] as f64 / 255.0,
            right_selection_rgb_u8[2] as f64 / 255.0,
        ];
        let to_u8_rgb = |rgb: [f64; 3]| -> [u32; 3] {
            [
                (rgb[0].clamp(0.0, 1.0) * 255.0).round() as u32,
                (rgb[1].clamp(0.0, 1.0) * 255.0).round() as u32,
                (rgb[2].clamp(0.0, 1.0) * 255.0).round() as u32,
            ]
        };
        let mut combo = 0u64;
        let mut combo_color_index = 0i64;
        let combo_colors_len = combo_colors.len() as i64;

        let kiai_time = {
            let mut is_kiai_time = false;
            let mut kiai_time = (0.0, 0.0);
            for (start, end) in kiai_times.iter() {
                if *start <= time_ms && time_ms <= *end {
                    is_kiai_time = true;
                    kiai_time = (*start, *end);
                    break;
                }
            }
            if is_kiai_time { Some(kiai_time) } else { None }
        };

        let break_time = {
            let mut is_break_time = false;
            let mut break_time = (0.0, 0.0);
            for (start, end) in break_times.iter() {
                if *start <= time_ms && time_ms <= *end {
                    is_break_time = true;
                    break_time = (*start, *end);
                    break;
                }
            }
            if is_break_time {
                Some(break_time)
            } else {
                None
            }
        };

        const SPINNER_POST_FADE_MS: f64 = 500.0;
        let mut spinner_state: u32 = 0;
        let mut spinner_time = (0.0, 0.0);
        for object in objects.iter() {
            let Some(object) = object.instance() else {
                continue;
            };
            if !object.is_spinner {
                continue;
            }
            let (start, end) = (object.time, object.slider_end_time_ms);
            if spinner_state == 0 && start <= time_ms && time_ms <= end {
                spinner_state = 1;
                spinner_time = (start, end);
                break;
            } else if spinner_state == 1 && end <= time_ms && time_ms <= end + SPINNER_POST_FADE_MS
            {
                spinner_state = 2;
                spinner_time = (start, end);
                break;
            }
        }

        let mut spinner_selection_side: u32 = 0;
        if spinner_state != 0 {
            for (object_idx, object) in objects.iter().enumerate() {
                let Some(object) = object.instance() else {
                    continue;
                };
                if !object.is_spinner {
                    continue;
                }

                let is_target_spinner = if spinner_state == 1 {
                    object.time <= time_ms && time_ms <= object.slider_end_time_ms
                } else {
                    object.slider_end_time_ms <= time_ms
                        && time_ms <= object.slider_end_time_ms + SPINNER_POST_FADE_MS
                };

                if !is_target_spinner {
                    continue;
                }

                spinner_selection_side = if left_selected_set.contains(&object_idx) {
                    1
                } else if right_selected_set.contains(&object_idx) {
                    2
                } else {
                    0
                };
                break;
            }
        }

        for (object_idx, circle) in objects.iter().enumerate() {
            let combo_info = circle.hit_object.combo_info().clone();
            let Some(circle) = circle.instance() else {
                continue;
            };

            let left_selected = left_selected_set.contains(&object_idx);
            let right_selected = right_selected_set.contains(&object_idx);
            let selected_side =
                (if left_selected { 1 } else { 0 }) | (if right_selected { 2 } else { 0 });

            if circle.is_new_combo {
                combo = 1;
                if !circle.is_spinner && combo_colors_len > 0 {
                    combo_color_index += 1 + combo_info.color_skip;
                    combo_color_index %= combo_colors_len;
                }
            } else {
                combo += 1;
            }

            let combo_color = if combo_colors_len > 0 {
                let combo_color = &combo_colors[combo_color_index as usize];
                [
                    combo_color.r / 255.0,
                    combo_color.g / 255.0,
                    combo_color.b / 255.0,
                ]
            } else {
                [1.0, 1.0, 1.0]
            };

            let slider_start_border_color = combo_color;
            let slider_end_border_color = if config.appearance.general.use_custom_slider_end_color {
                let slider_end_color_weight = config.appearance.colors.slider_end_rgba[3];
                [
                    config.appearance.colors.slider_end_rgba[0] / 255.0 * slider_end_color_weight
                        + combo_color[0] * (1.0 - slider_end_color_weight),
                    config.appearance.colors.slider_end_rgba[1] / 255.0 * slider_end_color_weight
                        + combo_color[1] * (1.0 - slider_end_color_weight),
                    config.appearance.colors.slider_end_rgba[2] / 255.0 * slider_end_color_weight
                        + combo_color[2] * (1.0 - slider_end_color_weight),
                ]
            } else {
                combo_color
            };

            let appear_ms = circle.time - circle.preempt - IGNORE_CIRCLES_DELTA;
            let end_ms = if circle.is_slider {
                circle.slider_end_time_ms
            } else {
                circle.time
            };
            let disappear_ms = end_ms + FADE_OUT_MS + IGNORE_CIRCLES_DELTA;

            if selected_side == 0 && (time_ms < appear_ms || time_ms > disappear_ms) {
                continue;
            }

            if circle.is_spinner {
                continue;
            }

            if circle.time <= time_ms && time_ms <= end_ms && circle.is_slider {
                // set active slider info
                (
                    current_slider_position,
                    current_slider_progress,
                    current_slider_ball_direction,
                ) = circle.sample_position_and_progress_and_direction(time_ms);
                let grow_out_duration = 100.0;
                current_slider_follow_circle_scaling = if time_ms > circle.time + grow_out_duration
                {
                    2.4
                } else {
                    1.0 + 1.4 * ((time_ms - circle.time) / grow_out_duration)
                };
                let animation_duration = 20.0;
                current_slider_ball_rotation_index =
                    ((time_ms - circle.time) / animation_duration).floor() as i32;
                current_slider_radius = circle.radius;
                current_slider_color = match selected_side {
                    1 => left_selection_rgb,
                    2 => right_selection_rgb,
                    3 => [
                        left_selection_rgb[0] * (1.0 - current_slider_progress)
                            + right_selection_rgb[0] * current_slider_progress,
                        left_selection_rgb[1] * (1.0 - current_slider_progress)
                            + right_selection_rgb[1] * current_slider_progress,
                        left_selection_rgb[2] * (1.0 - current_slider_progress)
                            + right_selection_rgb[2] * current_slider_progress,
                    ],
                    _ => [
                        slider_start_border_color[0] * (1.0 - current_slider_progress)
                            + slider_end_border_color[0] * current_slider_progress,
                        slider_start_border_color[1] * (1.0 - current_slider_progress)
                            + slider_end_border_color[1] * current_slider_progress,
                        slider_start_border_color[2] * (1.0 - current_slider_progress)
                            + slider_end_border_color[2] * current_slider_progress,
                    ],
                };
            }

            let mut circle_gpu = CircleGpu::from_instance(
                &circle,
                combo as u32,
                [
                    combo_color[0] as f32,
                    combo_color[1] as f32,
                    combo_color[2] as f32,
                ],
                to_u8_rgb(slider_start_border_color),
                to_u8_rgb(slider_end_border_color),
            );
            circle_gpu.selected_side = selected_side;
            match selected_side {
                1 => {
                    circle_gpu.slider_start_border_color = left_selection_rgb_u8;
                    circle_gpu.slider_end_border_color = left_selection_rgb_u8;
                }
                2 => {
                    circle_gpu.slider_start_border_color = right_selection_rgb_u8;
                    circle_gpu.slider_end_border_color = right_selection_rgb_u8;
                }
                3 => {
                    circle_gpu.slider_start_border_color = left_selection_rgb_u8;
                    circle_gpu.slider_end_border_color = right_selection_rgb_u8;
                }
                _ => {}
            }

            if let Some(curve) = circle.slider_path.as_ref() {
                let to4_ridge = |p: Vec2, progress: f64| -> [f32; 4] {
                    [p.x as f32, p.y as f32, progress as f32, 0.0]
                };

                let box_start = slider_boxes.len() as u32;
                let mut box_count: u32 = 0;
                for b in curve.boxes.iter() {
                    let seg_start = slider_segs.len() as u32;
                    for seg in b.segments.iter() {
                        let a = seg[0];
                        let c = seg[1];
                        slider_segs.push(SliderSegGpu {
                            ridge0: to4_ridge(a.point, a.progress),
                            ridge1: to4_ridge(c.point, c.progress),
                        });
                    }
                    let seg_count = (slider_segs.len() as u32) - seg_start;
                    if seg_count == 0 {
                        continue;
                    }
                    slider_boxes.push(SliderBoxGpu {
                        bbox_min: [b.bbox.x[0] as f32, b.bbox.y[0] as f32],
                        bbox_max: [b.bbox.x[1] as f32, b.bbox.y[1] as f32],
                        seg_start,
                        seg_count,
                        obj_iid: count as u32,
                        _pad: 0,
                    });
                    box_count += 1;
                }

                if box_count > 0 {
                    slider_draw_lookup.push(slider_draw_indices.len() as i32);
                    slider_draw_indices.push(count as u32);
                } else {
                    slider_draw_lookup.push(-1);
                }

                let _start_pt = curve.ridge.start_point();
                let end_pt = curve.ridge.end_point();

                let start_rotation = curve.ridge.start_rotation();
                let end_rotation = curve.ridge.end_rotation();
                circle_gpu.slider_head_rotation =
                    [start_rotation.x as f32, start_rotation.y as f32];
                circle_gpu.slider_end_rotation = [end_rotation.x as f32, end_rotation.y as f32];
                circle_gpu.slider_end_center_xy = [end_pt.x as f32, end_pt.y as f32];

                circle_gpu.slider_box_start = box_start;
                circle_gpu.slider_box_count = box_count;
            } else {
                slider_draw_lookup.push(-1);
            }

            circles_to_upload[count] = circle_gpu;
            count += 1;
            if count >= MAX_CIRCLES {
                break;
            }
        }

        let timeline_rect = layout.timeline_rect.to_f32_array();
        let timeline_hitbox_rect = layout.timeline_hitbox_rect.to_f32_array();
        let top_timeline_rect = layout.top_timeline_rect.to_f32_array();
        let top_timeline_hitbox_rect = layout.top_timeline_hitbox_rect.to_f32_array();
        let top_timeline_second_rect = layout.top_timeline_second_rect.to_f32_array();
        let top_timeline_second_hitbox_rect = layout.top_timeline_second_hitbox_rect.to_f32_array();
        let top_timeline_third_rect = layout.top_timeline_third_rect.to_f32_array();
        let top_timeline_third_hitbox_rect = layout.top_timeline_third_hitbox_rect.to_f32_array();
        let stats_box_rect = layout.stats_box_rect.to_f32_array();
        let play_pause_button_rect = layout.play_pause_button_rect.to_f32_array();

        let mut kiai_intervals: Vec<[f32; 2]> = Vec::with_capacity(MAX_KIAI_INTERVALS);
        for (start, end) in kiai_times.iter() {
            if end <= start {
                continue;
            }
            if kiai_intervals.len() >= MAX_KIAI_INTERVALS {
                break;
            }
            kiai_intervals.push([*start as f32, *end as f32]);
        }
        let mut break_intervals: Vec<[f32; 2]> = Vec::with_capacity(MAX_BREAK_INTERVALS);
        for (start, end) in break_times.iter() {
            if end <= start {
                continue;
            }
            if break_intervals.len() >= MAX_BREAK_INTERVALS {
                break;
            }
            break_intervals.push([*start as f32, *end as f32]);
        }
        let mut bookmark_times: Vec<f32> = Vec::with_capacity(MAX_BOOKMARKS);
        for bookmark in bookmarks.iter() {
            if bookmark_times.len() >= MAX_BOOKMARKS {
                break;
            }
            bookmark_times.push(*bookmark as f32);
        }

        let mut red_line_times: Vec<f32> = Vec::with_capacity(MAX_RED_LINES);
        for red_line in red_lines.iter() {
            if red_line_times.len() >= MAX_RED_LINES {
                break;
            }
            red_line_times.push(*red_line as f32);
        }

        let mut timeline_markers: Vec<[f32; 2]> =
            Vec::with_capacity(bookmark_times.len() + red_line_times.len());
        for bookmark in bookmark_times.iter() {
            timeline_markers.push([*bookmark, 0.0]);
        }
        for red_line in red_line_times.iter() {
            timeline_markers.push([*red_line, 0.0]);
        }

        if !kiai_intervals.is_empty() {
            self.queue.write_buffer(
                &self.timeline_kiai_buffer,
                0,
                bytemuck::cast_slice(kiai_intervals.as_slice()),
            );
        }
        if !break_intervals.is_empty() {
            self.queue.write_buffer(
                &self.timeline_break_buffer,
                0,
                bytemuck::cast_slice(break_intervals.as_slice()),
            );
        }
        if !timeline_markers.is_empty() {
            self.queue.write_buffer(
                &self.timeline_bookmark_buffer,
                0,
                bytemuck::cast_slice(timeline_markers.as_slice()),
            );
        }

        let static_snap_count = MAX_SNAP_MARKERS.min(snap_positions.len());
        let remaining_snap_capacity = MAX_SNAP_MARKERS.saturating_sub(static_snap_count);
        let movable_snap_count = remaining_snap_capacity.min(movable_snap_positions.len());
        let mut snap_markers_upload: Vec<[f32; 2]> =
            Vec::with_capacity(static_snap_count + movable_snap_count);
        for pos in snap_positions.iter().take(static_snap_count) {
            snap_markers_upload.push([pos.x as f32, pos.y as f32]);
        }
        for pos in movable_snap_positions.iter().take(movable_snap_count) {
            snap_markers_upload.push([pos.x as f32, pos.y as f32]);
        }

        let fps_clamped = fps.clamp(0.0, u32::MAX as f64 / 10.0);
        let fps_low_clamped = fps_low.clamp(0.0, u32::MAX as f64 / 10.0);
        let fps_x10 = (fps_clamped * 10.0).round() as u32;
        let fps_low_x10 = (fps_low_clamped * 10.0).round() as u32;
        let next_states_count = undo_next_states.len().min(8);
        let mut undo_next_states_uuid_0 = [0u32; 4];
        let mut undo_next_states_uuid_1 = [0u32; 4];
        let mut undo_next_states_age_0 = [0u32; 4];
        let mut undo_next_states_age_1 = [0u32; 4];
        let mut undo_next_states_age_unit_0 = [0u32; 4];
        let mut undo_next_states_age_unit_1 = [0u32; 4];
        for (idx, (uuid, age, age_unit)) in
            undo_next_states.iter().take(next_states_count).enumerate()
        {
            if idx < 4 {
                undo_next_states_uuid_0[idx] = *uuid;
                undo_next_states_age_0[idx] = *age;
                undo_next_states_age_unit_0[idx] = *age_unit;
            } else {
                undo_next_states_uuid_1[idx - 4] = *uuid;
                undo_next_states_age_1[idx - 4] = *age;
                undo_next_states_age_unit_1[idx - 4] = *age_unit;
            }
        }

        let mut undo_prev_state_name_chars = [0u32; 16];
        let mut undo_prev_state_name_len = 0usize;
        if let Some(name) = undo_prev_state_display_name.as_deref() {
            for ch in name.chars() {
                if undo_prev_state_name_len >= undo_prev_state_name_chars.len() {
                    break;
                }
                if ch.is_control() {
                    continue;
                }
                let code = if ch.is_ascii() { ch as u32 } else { '?' as u32 };
                undo_prev_state_name_chars[undo_prev_state_name_len] = code;
                undo_prev_state_name_len += 1;
            }
        }
        let mut undo_prev_state_name_packed = [0u32; 4];
        for (idx, code) in undo_prev_state_name_chars.iter().enumerate() {
            let word = idx / 4;
            let shift = (idx % 4) * 8;
            undo_prev_state_name_packed[word] |= (*code & 0xFF) << shift;
        }

        let mut undo_next_states_name_len_0 = [0u32; 4];
        let mut undo_next_states_name_len_1 = [0u32; 4];
        let mut undo_next_states_name_chars = [0u32; 128];
        for (row, maybe_name) in undo_next_state_display_names
            .iter()
            .take(next_states_count)
            .enumerate()
        {
            let mut char_count = 0usize;
            if let Some(name) = maybe_name.as_deref() {
                for ch in name.chars() {
                    if char_count >= 16 {
                        break;
                    }
                    if ch.is_control() {
                        continue;
                    }
                    let code = if ch.is_ascii() { ch as u32 } else { '?' as u32 };
                    undo_next_states_name_chars[row * 16 + char_count] = code;
                    char_count += 1;
                }
            }

            if row < 4 {
                undo_next_states_name_len_0[row] = char_count as u32;
            } else {
                undo_next_states_name_len_1[row - 4] = char_count as u32;
            }
        }
        let mut undo_next_states_name_packed = [[0u32; 4]; 8];
        for row in 0..8 {
            for char_idx in 0..16 {
                let code = undo_next_states_name_chars[row * 16 + char_idx] & 0xFF;
                let word = char_idx / 4;
                let shift = (char_idx % 4) * 8;
                undo_next_states_name_packed[row][word] |= code << shift;
            }
        }

        let current_state_name_source = if current_state_rename_active {
            current_state_rename_text
        } else {
            undo_current_state_display_name.as_deref().unwrap_or("")
        };

        let mut current_state_name_chars = [0u32; 32];
        let mut current_state_name_len = 0usize;
        for ch in current_state_name_source.chars() {
            if current_state_name_len >= current_state_name_chars.len() {
                break;
            }
            if ch.is_control() {
                continue;
            }
            let code = if ch.is_ascii() { ch as u32 } else { '?' as u32 };
            current_state_name_chars[current_state_name_len] = code;
            current_state_name_len += 1;
        }
        let mut current_state_name_text_0 = [0u32; 4];
        let mut current_state_name_text_1 = [0u32; 4];
        let mut current_state_name_text_2 = [0u32; 4];
        let mut current_state_name_text_3 = [0u32; 4];
        let mut current_state_name_text_4 = [0u32; 4];
        let mut current_state_name_text_5 = [0u32; 4];
        let mut current_state_name_text_6 = [0u32; 4];
        let mut current_state_name_text_7 = [0u32; 4];
        for (idx, code) in current_state_name_chars.iter().enumerate() {
            if idx < 4 {
                current_state_name_text_0[idx] = *code;
            } else if idx < 8 {
                current_state_name_text_1[idx - 4] = *code;
            } else if idx < 12 {
                current_state_name_text_2[idx - 8] = *code;
            } else if idx < 16 {
                current_state_name_text_3[idx - 12] = *code;
            } else if idx < 20 {
                current_state_name_text_4[idx - 16] = *code;
            } else if idx < 24 {
                current_state_name_text_5[idx - 20] = *code;
            } else if idx < 28 {
                current_state_name_text_6[idx - 24] = *code;
            } else {
                current_state_name_text_7[idx - 28] = *code;
            }
        }

        let globals = Globals {
            screen_size: [self.config.width as f32, self.config.height as f32],
            time_ms: time_ms as f32,
            slider_border_thickness: config.appearance.layout.slider_border_thickness as f32,
            playfield_rect,
            osu_rect: gameplay_rect,

            playfield_rgba: [
                (config.appearance.colors.playfield_rgba[0] / 255.0) as f32,
                (config.appearance.colors.playfield_rgba[1] / 255.0) as f32,
                (config.appearance.colors.playfield_rgba[2] / 255.0) as f32,
                config.appearance.colors.playfield_rgba[3] as f32,
            ],

            gameplay_rgba: [
                (config.appearance.colors.gameplay_rgba[0] / 255.0) as f32,
                (config.appearance.colors.gameplay_rgba[1] / 255.0) as f32,
                (config.appearance.colors.gameplay_rgba[2] / 255.0) as f32,
                config.appearance.colors.gameplay_rgba[3] as f32,
            ],

            outer_rgba: [
                (config.appearance.colors.outer_rgba[0] / 255.0) as f32,
                (config.appearance.colors.outer_rgba[1] / 255.0) as f32,
                (config.appearance.colors.outer_rgba[2] / 255.0) as f32,
                config.appearance.colors.outer_rgba[3] as f32,
            ],

            playfield_border_rgba: [
                (config.appearance.colors.playfield_border_rgba[0] / 255.0) as f32,
                (config.appearance.colors.playfield_border_rgba[1] / 255.0) as f32,
                (config.appearance.colors.playfield_border_rgba[2] / 255.0) as f32,
                config.appearance.colors.playfield_border_rgba[3] as f32,
            ],

            gameplay_border_rgba: [
                (config.appearance.colors.gameplay_border_rgba[0] / 255.0) as f32,
                (config.appearance.colors.gameplay_border_rgba[1] / 255.0) as f32,
                (config.appearance.colors.gameplay_border_rgba[2] / 255.0) as f32,
                config.appearance.colors.gameplay_border_rgba[3] as f32,
            ],

            slider_ridge_rgba: [
                (config.appearance.colors.slider_ridge_rgba[0] / 255.0) as f32,
                (config.appearance.colors.slider_ridge_rgba[1] / 255.0) as f32,
                (config.appearance.colors.slider_ridge_rgba[2] / 255.0) as f32,
                config.appearance.colors.slider_ridge_rgba[3] as f32,
            ],
            slider_body_rgba: [
                (config.appearance.colors.slider_body_rgba[0] / 255.0) as f32,
                (config.appearance.colors.slider_body_rgba[1] / 255.0) as f32,
                (config.appearance.colors.slider_body_rgba[2] / 255.0) as f32,
                config.appearance.colors.slider_body_rgba[3] as f32,
            ],

            break_time_lightness: config.appearance.general.break_time_lightness as f32,
            is_break_time: if break_time.is_some() { 1 } else { 0 },
            is_kiai_time: if kiai_time.is_some() { 1 } else { 0 },

            slider_progress: current_slider_progress as f32,
            slider_position: [
                current_slider_position.x as f32,
                current_slider_position.y as f32,
            ],
            slider_follow_circle_scaling: current_slider_follow_circle_scaling as f32,
            slider_ball_direction: [
                current_slider_ball_direction.x as f32,
                current_slider_ball_direction.y as f32,
            ],
            slider_ball_rotation_index: current_slider_ball_rotation_index,
            slider_radius: current_slider_radius as f32,
            slider_color: [
                current_slider_color[0] as f32,
                current_slider_color[1] as f32,
                current_slider_color[2] as f32,
            ],
            slider_border_outer_thickness: config.appearance.layout.slider_outer_thickness as f32,
            fps_x10,
            _pad0: [0, 0, 0],
            fps_low_x10,

            song_total_ms: song_total_ms.max(0.0) as f32,
            playback_rate: playback_rate as f32,
            audio_volume: audio_volume.clamp(0.0, 1.0) as f32,
            hitsound_volume: hitsound_volume.clamp(0.0, 1.0) as f32,
            cpu_pass_x10: self.cpu_pass_x10,
            gpu_pass_x10: self.gpu_pass_x10,
            hud_opacity: 1.0,
            is_playing: if is_playing { 1 } else { 0 },
            time_elapsed_ms: time_elapsed_ms as f32,
            loading: if is_loading { 1 } else { 0 },
            break_time: match break_time {
                Some((start, end)) => [start as f32, end as f32],
                None => [0.0, 0.0],
            },
            spinner_time: [spinner_time.0 as f32, spinner_time.1 as f32],
            spinner_state,
            undo_count: undo_count as u32,
            undo_redo_info: [
                undo_prev_state.map(|v| v.0).unwrap_or(0),
                undo_current_state.0,
                next_states_count as u32,
                if undo_prev_state.is_some() { 1 } else { 0 },
            ],
            undo_prev_state_info: [
                undo_prev_state.map(|v| v.0).unwrap_or(0),
                undo_prev_state.map(|v| v.1).unwrap_or(0),
                undo_prev_state.map(|v| v.2).unwrap_or(0),
                if undo_prev_state.is_some() { 1 } else { 0 },
            ],
            undo_current_state_info: [
                undo_current_state.0,
                undo_current_state.1,
                undo_current_state.2,
                1,
            ],
            undo_next_states_uuid_0,
            undo_next_states_uuid_1,
            undo_next_states_age_0,
            undo_next_states_age_1,
            undo_next_states_age_unit_0,
            undo_next_states_age_unit_1,
            undo_prev_state_name_meta: [undo_prev_state_name_len as u32, 0, 0, 0],
            undo_prev_state_name_packed,
            undo_next_states_name_len_0,
            undo_next_states_name_len_1,
            undo_next_states_name_packed,
            undo_button_meta: [
                if undo_button_hovered { 1 } else { 0 },
                if undo_button_clicked { 1 } else { 0 },
                0,
                0,
            ],
            current_state_button_meta: [
                if current_state_button_hovered { 1 } else { 0 },
                if current_state_button_clicked { 1 } else { 0 },
                0,
                0,
            ],
            current_state_name_meta: [
                current_state_name_len as u32,
                if current_state_rename_active { 1 } else { 0 },
                0,
                0,
            ],
            current_state_name_text_0,
            current_state_name_text_1,
            current_state_name_text_2,
            current_state_name_text_3,
            current_state_name_text_4,
            current_state_name_text_5,
            current_state_name_text_6,
            current_state_name_text_7,
            redo_buttons_meta: [
                redo_button_hovered_row.unwrap_or(0),
                if redo_button_hovered_row.is_some() {
                    1
                } else {
                    0
                },
                redo_button_clicked_row.unwrap_or(0),
                if redo_button_clicked_row.is_some() {
                    1
                } else {
                    0
                },
            ],
            top_timeline_rect,
            top_timeline_hitbox_rect,
            top_timeline_second_rect,
            top_timeline_second_hitbox_rect,
            top_timeline_third_rect,
            top_timeline_third_hitbox_rect,
            timeline_rect,
            timeline_hitbox_rect,
            play_pause_button_rect: play_pause_button_rect,
            stats_box_rect,
            play_pause_button_meta: [
                if play_pause_button_hovered { 1 } else { 0 },
                if play_pause_button_clicked { 1 } else { 0 },
                0,
                0,
            ],
            overlay_rect_left: overlay_rect_left.unwrap_or([0.0, 0.0, 0.0, 0.0]),
            overlay_rect_right: overlay_rect_right.unwrap_or([0.0, 0.0, 0.0, 0.0]),
            selection_quad_left_01: selection_rect_left
                .map(|q| [q[0][0], q[0][1], q[1][0], q[1][1]])
                .unwrap_or([0.0, 0.0, 0.0, 0.0]),
            selection_quad_left_23: selection_rect_left
                .map(|q| [q[2][0], q[2][1], q[3][0], q[3][1]])
                .unwrap_or([0.0, 0.0, 0.0, 0.0]),
            selection_quad_right_01: selection_rect_right
                .map(|q| [q[0][0], q[0][1], q[1][0], q[1][1]])
                .unwrap_or([0.0, 0.0, 0.0, 0.0]),
            selection_quad_right_23: selection_rect_right
                .map(|q| [q[2][0], q[2][1], q[3][0], q[3][1]])
                .unwrap_or([0.0, 0.0, 0.0, 0.0]),
            selection_origin_left: selection_origin_left
                .map(|p| {
                    [
                        p[0],
                        p[1],
                        if selection_left_origin_hovered {
                            1.0
                        } else {
                            0.0
                        },
                        if selection_left_origin_dragging {
                            1.0
                        } else {
                            0.0
                        },
                    ]
                })
                .unwrap_or([0.0, 0.0, 0.0, 0.0]),
            selection_origin_right: selection_origin_right
                .map(|p| {
                    [
                        p[0],
                        p[1],
                        if selection_right_origin_hovered {
                            1.0
                        } else {
                            0.0
                        },
                        if selection_right_origin_dragging {
                            1.0
                        } else {
                            0.0
                        },
                    ]
                })
                .unwrap_or([0.0, 0.0, 0.0, 0.0]),
            selection_drag_pos_left: selection_drag_pos_left
                .map(|p| [p[0], p[1], 1.0, 0.0])
                .unwrap_or([0.0, 0.0, 0.0, 0.0]),
            selection_drag_pos_right: selection_drag_pos_right
                .map(|p| [p[0], p[1], 1.0, 0.0])
                .unwrap_or([0.0, 0.0, 0.0, 0.0]),
            left_selection_colors: [
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .drag_rectangle[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .drag_rectangle[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .drag_rectangle[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .drag_rectangle[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_hovered[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_hovered[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_hovered[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_hovered[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_dragging[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_dragging[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_dragging[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_border_dragging[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_hovered[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_hovered[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_hovered[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_hovered[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_dragging[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_dragging[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_dragging[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_tint_dragging[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_hovered[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_hovered[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_hovered[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_hovered[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_clicked[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_clicked[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_clicked[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_clicked[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_locked[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_locked[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_locked[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_origin_locked[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_combo_color[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_combo_color[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_combo_color[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .left_selection_colors
                        .selection_combo_color[3] as f32,
                ],
            ],
            right_selection_colors: [
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .drag_rectangle[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .drag_rectangle[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .drag_rectangle[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .drag_rectangle[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_hovered[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_hovered[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_hovered[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_hovered[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_dragging[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_dragging[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_dragging[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_border_dragging[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_hovered[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_hovered[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_hovered[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_hovered[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_dragging[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_dragging[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_dragging[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_tint_dragging[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_hovered[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_hovered[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_hovered[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_hovered[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_clicked[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_clicked[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_clicked[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_clicked[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_locked[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_locked[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_locked[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_origin_locked[3] as f32,
                ],
                [
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_combo_color[0]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_combo_color[1]
                        / 255.0) as f32,
                    (config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_combo_color[2]
                        / 255.0) as f32,
                    config
                        .appearance
                        .colors
                        .right_selection_colors
                        .selection_combo_color[3] as f32,
                ],
            ],
            overlay_meta: [
                if overlay_rect_left.is_some() { 1 } else { 0 },
                if overlay_rect_right.is_some() { 1 } else { 0 },
                if selection_left_bbox_hovered { 1 } else { 0 },
                if selection_right_bbox_hovered { 1 } else { 0 },
            ],
            selection_meta: [
                if selection_rect_left.is_some() { 1 } else { 0 },
                if selection_rect_right.is_some() { 1 } else { 0 },
                left_selected_objects.len().min(u32::MAX as usize) as u32,
                right_selected_objects.len().min(u32::MAX as usize) as u32,
            ],
            spinner_selection_meta: [spinner_selection_side, 0, 0, 0],
            kiai_interval_count: kiai_intervals.len() as u32,
            break_interval_count: break_intervals.len() as u32,
            bookmark_count: bookmark_times.len() as u32,
            red_line_count: red_line_times.len() as u32,
            cursor_pos,
            selected_fade_in_opacity_cap: config
                .appearance
                .general
                .selected_fade_in_opacity_cap
                .clamp(0.0, 1.0) as f32,
            selected_fade_out_opacity_cap: config
                .appearance
                .general
                .selected_fade_out_opacity_cap
                .clamp(0.0, 1.0) as f32,
            selection_color_mix_strength: config
                .appearance
                .general
                .selection_color_mix_strength
                .clamp(0.0, 1.0) as f32,
            selection_left_scale: left_selection_scale as f32,
            selection_right_scale: right_selection_scale as f32,
            selection_left_rotation_degrees: left_selection_rotation_degrees as f32,
            selection_right_rotation_degrees: right_selection_rotation_degrees as f32,
            _pad4: [0, 0, 0],
            selection_exists_meta: [
                if left_selection_exists { 1 } else { 0 },
                if right_selection_exists { 1 } else { 0 },
                0,
                0,
            ],
            selection_origin_left_playfield: selection_origin_left_playfield.unwrap_or([0.0, 0.0]),
            selection_origin_right_playfield: selection_origin_right_playfield
                .unwrap_or([0.0, 0.0]),
            selection_moved_left_playfield,
            selection_moved_right_playfield,
            selection_lock_meta: [
                if left_selection_origin_locked { 1 } else { 0 },
                if right_selection_origin_locked { 1 } else { 0 },
                if left_selection_scale_locked { 1 } else { 0 },
                if right_selection_scale_locked { 1 } else { 0 },
            ],
            selection_box_dragging_meta: [
                if selection_left_bbox_dragging { 1 } else { 0 },
                if selection_right_bbox_dragging { 1 } else { 0 },
                0,
                0,
            ],
            snap_marker_rgba: [
                (config.appearance.colors.snap_marker_rgba[0] / 255.0) as f32,
                (config.appearance.colors.snap_marker_rgba[1] / 255.0) as f32,
                (config.appearance.colors.snap_marker_rgba[2] / 255.0) as f32,
                config.appearance.colors.snap_marker_rgba[3] as f32,
            ],
            snap_marker_style: [
                config.appearance.layout.snap_marker_radius_px.max(0.0) as f32,
                0.0,
                0.0,
                0.0,
            ],
            movable_snap_marker_rgba: [
                (config.appearance.colors.movable_snap_hitbox_rgba[0] / 255.0) as f32,
                (config.appearance.colors.movable_snap_hitbox_rgba[1] / 255.0) as f32,
                (config.appearance.colors.movable_snap_hitbox_rgba[2] / 255.0) as f32,
                config.appearance.colors.movable_snap_hitbox_rgba[3] as f32,
            ],
            movable_snap_marker_style: [
                config
                    .appearance
                    .layout
                    .movable_snap_hitbox_radius_px
                    .max(0.0) as f32,
                0.0,
                0.0,
                0.0,
            ],
            snap_meta: [
                static_snap_count.min(u32::MAX as usize) as u32,
                if drag_happening { 1 } else { 0 },
                movable_snap_count.min(u32::MAX as usize) as u32,
                0,
            ],
            drag_state_marker_rgba: [
                (config.appearance.colors.drag_state_marker_rgba[0] / 255.0) as f32,
                (config.appearance.colors.drag_state_marker_rgba[1] / 255.0) as f32,
                (config.appearance.colors.drag_state_marker_rgba[2] / 255.0) as f32,
                config.appearance.colors.drag_state_marker_rgba[3] as f32,
            ],
            drag_state_marker_style: [
                config
                    .appearance
                    .layout
                    .drag_state_marker_radius_px
                    .max(0.0) as f32,
                0.0,
                0.0,
                0.0,
            ],
            offscreen_playfield_tint_rgba: [
                (config.appearance.colors.offscreen_playfield_tint_rgb[0] / 255.0) as f32,
                (config.appearance.colors.offscreen_playfield_tint_rgb[1] / 255.0) as f32,
                (config.appearance.colors.offscreen_playfield_tint_rgb[2] / 255.0) as f32,
                1.0,
            ],
            offscreen_osu_tint_rgba: [
                (config.appearance.colors.offscreen_osu_tint_rgb[0] / 255.0) as f32,
                (config.appearance.colors.offscreen_osu_tint_rgb[1] / 255.0) as f32,
                (config.appearance.colors.offscreen_osu_tint_rgb[2] / 255.0) as f32,
                1.0,
            ],
            timeline_window_ms,
            timeline_current_x: timeline_current_x as f32,
            timeline_zoom: timeline_zoom as f32,
            timeline_object_meta: [
                timeline_points_gpu.len().min(u32::MAX as usize) as u32,
                timeline_x_boxes_gpu.len().min(u32::MAX as usize) as u32,
                0,
                0,
            ],
            timeline_style: [
                timeline_radius_px as f32,
                config
                    .appearance
                    .timeline
                    .slider_outline_thickness_percent
                    .clamp(0.0, 1.0) as f32,
                config
                    .appearance
                    .timeline
                    .slider_repeat_point_radius_percent
                    .clamp(0.0, 1.0) as f32,
                config
                    .appearance
                    .timeline
                    .slider_end_point_radius_percent
                    .clamp(0.0, 1.0) as f32,
            ],
            timeline_slider_outline_rgba: [
                (config.appearance.colors.timeline_slider_outline_rgba[0] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_outline_rgba[1] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_outline_rgba[2] / 255.0) as f32,
                config.appearance.colors.timeline_slider_outline_rgba[3] as f32,
            ],
            timeline_slider_head_body_rgba: [
                (config.appearance.colors.timeline_slider_head_body_rgba[0] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_head_body_rgba[1] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_head_body_rgba[2] / 255.0) as f32,
                config.appearance.colors.timeline_slider_head_body_rgba[3] as f32,
            ],
            timeline_slider_head_overlay_rgba: [
                (config.appearance.colors.timeline_slider_head_overlay_rgba[0] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_head_overlay_rgba[1] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_head_overlay_rgba[2] / 255.0) as f32,
                config.appearance.colors.timeline_slider_head_overlay_rgba[3] as f32,
            ],
            timeline_circle_head_body_rgba: [
                (config.appearance.colors.timeline_circle_head_body_rgba[0] / 255.0) as f32,
                (config.appearance.colors.timeline_circle_head_body_rgba[1] / 255.0) as f32,
                (config.appearance.colors.timeline_circle_head_body_rgba[2] / 255.0) as f32,
                config.appearance.colors.timeline_circle_head_body_rgba[3] as f32,
            ],
            timeline_circle_head_overlay_rgba: [
                (config.appearance.colors.timeline_circle_head_overlay_rgba[0] / 255.0) as f32,
                (config.appearance.colors.timeline_circle_head_overlay_rgba[1] / 255.0) as f32,
                (config.appearance.colors.timeline_circle_head_overlay_rgba[2] / 255.0) as f32,
                config.appearance.colors.timeline_circle_head_overlay_rgba[3] as f32,
            ],
            timeline_slider_head_point_rgba: [
                (config.appearance.colors.timeline_slider_head_point_rgba[0] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_head_point_rgba[1] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_head_point_rgba[2] / 255.0) as f32,
                config.appearance.colors.timeline_slider_head_point_rgba[3] as f32,
            ],
            timeline_slider_repeat_point_rgba: [
                (config.appearance.colors.timeline_slider_repeat_point_rgba[0] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_repeat_point_rgba[1] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_repeat_point_rgba[2] / 255.0) as f32,
                config.appearance.colors.timeline_slider_repeat_point_rgba[3] as f32,
            ],
            timeline_slider_end_point_rgba: [
                (config.appearance.colors.timeline_slider_end_point_rgba[0] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_end_point_rgba[1] / 255.0) as f32,
                (config.appearance.colors.timeline_slider_end_point_rgba[2] / 255.0) as f32,
                config.appearance.colors.timeline_slider_end_point_rgba[3] as f32,
            ],
            timeline_past_grayscale_strength: config
                .appearance
                .timeline
                .timeline_past_grayscale_strength
                .clamp(0.0, 1.0) as f32,
            _timeline_past_pad: [0.0, 0.0, 0.0],
            timeline_past_tint_rgba: [
                (config.appearance.colors.timeline_past_tint_rgba[0] / 255.0) as f32,
                (config.appearance.colors.timeline_past_tint_rgba[1] / 255.0) as f32,
                (config.appearance.colors.timeline_past_tint_rgba[2] / 255.0) as f32,
                config.appearance.colors.timeline_past_tint_rgba[3] as f32,
            ],
            timeline_past_object_tint_rgba: [
                (config.appearance.colors.timeline_past_object_tint_rgba[0] / 255.0) as f32,
                (config.appearance.colors.timeline_past_object_tint_rgba[1] / 255.0) as f32,
                (config.appearance.colors.timeline_past_object_tint_rgba[2] / 255.0) as f32,
                config.appearance.colors.timeline_past_object_tint_rgba[3] as f32,
            ],
            _pad_end: [0.0, 0.0, 0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        if snap_markers_upload.len() > self.snap_markers_capacity {
            self.snap_markers_capacity = snap_markers_upload.len().next_power_of_two().max(1);
            self.snap_markers_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("snap markers buffer (resized)"),
                size: (self.snap_markers_capacity * std::mem::size_of::<[f32; 2]>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let snap_markers_bind_group_layout = self.overlay_pipeline.get_bind_group_layout(2);
            self.snap_markers_bind_group =
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("snap markers bind group"),
                    layout: &snap_markers_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.snap_markers_buffer.as_entire_binding(),
                    }],
                });
        }

        if !snap_markers_upload.is_empty() {
            self.queue.write_buffer(
                &self.snap_markers_buffer,
                0,
                bytemuck::cast_slice(snap_markers_upload.as_slice()),
            );
        }

        if timeline_points_gpu.len() > self.timeline_points_capacity {
            self.timeline_points_capacity = timeline_points_gpu.len().next_power_of_two().max(1);
            self.timeline_points_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("timeline points buffer (resized)"),
                size: (self.timeline_points_capacity * std::mem::size_of::<TimelinePointGpu>())
                    as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let timeline_points_bind_group_layout =
                self.timeline_slider_pipeline.get_bind_group_layout(2);
            self.timeline_points_bind_group =
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("timeline points bind group"),
                    layout: &timeline_points_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.timeline_points_buffer.as_entire_binding(),
                    }],
                });
        }

        if timeline_x_boxes_gpu.len() > self.timeline_x_boxes_capacity {
            self.timeline_x_boxes_capacity = timeline_x_boxes_gpu.len().next_power_of_two().max(1);
            self.timeline_x_boxes_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("timeline x boxes buffer (resized)"),
                size: (self.timeline_x_boxes_capacity * std::mem::size_of::<TimelineXBoxGpu>())
                    as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let timeline_x_boxes_bind_group_layout =
                self.timeline_slider_pipeline.get_bind_group_layout(3);
            self.timeline_x_boxes_bind_group =
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("timeline x boxes bind group"),
                    layout: &timeline_x_boxes_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.timeline_x_boxes_buffer.as_entire_binding(),
                    }],
                });
        }

        if !timeline_points_gpu.is_empty() {
            self.queue.write_buffer(
                &self.timeline_points_buffer,
                0,
                bytemuck::cast_slice(timeline_points_gpu.as_slice()),
            );
        }

        if !timeline_x_boxes_gpu.is_empty() {
            self.queue.write_buffer(
                &self.timeline_x_boxes_buffer,
                0,
                bytemuck::cast_slice(timeline_x_boxes_gpu.as_slice()),
            );
        }

        // Ensure slider segment buffers are large enough; if not, recreate them (and bind group).
        if slider_segs.len() > self.slider_segs_capacity {
            self.slider_segs_capacity = slider_segs.len().next_power_of_two().max(1);
            self.slider_segs_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("slider segs buffer (resized)"),
                size: (self.slider_segs_capacity * std::mem::size_of::<SliderSegGpu>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        if slider_boxes.len() > self.slider_boxes_capacity {
            self.slider_boxes_capacity = slider_boxes.len().next_power_of_two().max(1);
            self.slider_boxes_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("slider boxes buffer (resized)"),
                size: (self.slider_boxes_capacity * std::mem::size_of::<SliderBoxGpu>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        if slider_draw_indices.len() > self.slider_draw_indices_capacity {
            self.slider_draw_indices_capacity =
                slider_draw_indices.len().next_power_of_two().max(1);
            self.slider_draw_indices_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("slider draw indices buffer (resized)"),
                size: (self.slider_draw_indices_capacity * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        // Recreate bind group every frame (buffers may have been replaced).
        // This is cheap relative to rendering and keeps logic simple.
        let slider_bind_group_layout = self.sliders_pipeline.get_bind_group_layout(3);
        self.slider_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("slider data bind group"),
            layout: &slider_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.slider_segs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.slider_boxes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.slider_draw_indices_buffer.as_entire_binding(),
                },
            ],
        });

        if !slider_segs.is_empty() {
            self.queue.write_buffer(
                &self.slider_segs_buffer,
                0,
                bytemuck::cast_slice(slider_segs.as_slice()),
            );
        }
        if !slider_boxes.is_empty() {
            self.queue.write_buffer(
                &self.slider_boxes_buffer,
                0,
                bytemuck::cast_slice(slider_boxes.as_slice()),
            );
        }
        if !slider_draw_indices.is_empty() {
            self.queue.write_buffer(
                &self.slider_draw_indices_buffer,
                0,
                bytemuck::cast_slice(slider_draw_indices.as_slice()),
            );
        }

        // Upload only the active instances (after slider buffers so indices are valid).
        self.queue.write_buffer(
            &self.objects_buffer,
            0,
            bytemuck::cast_slice(&circles_to_upload[..count]),
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        {
            let (color_view, resolve_target) = if self.msaa_samples > 1 {
                (
                    self.msaa_color_view
                        .as_ref()
                        .expect("msaa_color_view missing"),
                    Some(&swapchain_view),
                )
            } else {
                (&swapchain_view, None)
            };

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Background first.
            rpass.set_pipeline(&self.background_pipeline);
            rpass.set_bind_group(0, &self.globals_bind_group, &[]);
            rpass.set_bind_group(1, &self.texture_bind_group, &[]);
            rpass.draw(0..6, 0..1);

            // Draw objects back-to-front. (Later in the list draws first.)
            // For sliders: draw body (per box) first, then caps, then head circle.
            for obj_i in (0..count).rev() {
                let slider_draw_iid = slider_draw_lookup.get(obj_i).copied().unwrap_or(-1);
                if slider_draw_iid >= 0 {
                    let circle_gpu = &circles_to_upload[obj_i];
                    let box_start = circle_gpu.slider_box_start;
                    let box_count = circle_gpu.slider_box_count;
                    if box_count > 0 {
                        // Slider body: one draw over the box instances for this slider.
                        rpass.set_pipeline(&self.sliders_pipeline);
                        rpass.set_bind_group(0, &self.globals_bind_group, &[]);
                        rpass.set_bind_group(1, &self.texture_bind_group, &[]);
                        rpass.set_bind_group(2, &self.objects_bind_group, &[]);
                        rpass.set_bind_group(3, &self.slider_bind_group, &[]);
                        let start = box_start;
                        let end = box_start + box_count;
                        rpass.draw(0..6, start..end);
                    }

                    // Slider caps (end circle + reverse arrows) per slider.
                    rpass.set_pipeline(&self.slider_caps_pipeline);
                    rpass.set_bind_group(0, &self.globals_bind_group, &[]);
                    rpass.set_bind_group(1, &self.texture_bind_group, &[]);
                    rpass.set_bind_group(2, &self.objects_bind_group, &[]);
                    rpass.set_bind_group(3, &self.slider_bind_group, &[]);
                    let s = slider_draw_iid as u32;
                    rpass.draw(0..6, s..(s + 1));
                }

                rpass.set_pipeline(&self.circles_pipeline);
                rpass.set_bind_group(0, &self.globals_bind_group, &[]);
                rpass.set_bind_group(1, &self.texture_bind_group, &[]);
                rpass.set_bind_group(2, &self.objects_bind_group, &[]);
                let o = obj_i as u32;
                rpass.draw(0..6, o..(o + 1));
            }

            // HUD pass.
            rpass.set_pipeline(&self.hud_pipeline);
            rpass.set_bind_group(0, &self.globals_bind_group, &[]);
            rpass.set_bind_group(1, &self.texture_bind_group, &[]);
            rpass.set_bind_group(2, &self.snap_markers_bind_group, &[]);
            rpass.draw(0..6, 0..1);

            rpass.set_pipeline(&self.timeline_kiai_pipeline);
            rpass.set_bind_group(0, &self.globals_bind_group, &[]);
            rpass.set_bind_group(1, &self.timeline_empty_bind_group, &[]);
            rpass.set_bind_group(2, &self.timeline_empty_bind_group, &[]);
            rpass.set_bind_group(3, &self.timeline_kiai_bind_group, &[]);
            rpass.draw(0..6, 0..1);

            rpass.set_pipeline(&self.timeline_break_pipeline);
            rpass.set_bind_group(0, &self.globals_bind_group, &[]);
            rpass.set_bind_group(3, &self.timeline_break_bind_group, &[]);
            rpass.draw(0..6, 0..1);

            rpass.set_pipeline(&self.timeline_bookmark_pipeline);
            rpass.set_bind_group(0, &self.globals_bind_group, &[]);
            rpass.set_bind_group(3, &self.timeline_bookmark_bind_group, &[]);
            rpass.draw(0..6, 0..1);

            if !timeline_x_boxes_gpu.is_empty() {
                rpass.set_pipeline(&self.timeline_slider_pipeline);
                rpass.set_bind_group(0, &self.globals_bind_group, &[]);
                rpass.set_bind_group(1, &self.timeline_empty_bind_group, &[]);
                rpass.set_bind_group(2, &self.timeline_points_bind_group, &[]);
                rpass.set_bind_group(3, &self.timeline_x_boxes_bind_group, &[]);
                rpass.draw(0..6, 0..(timeline_x_boxes_gpu.len() as u32));
            }

            // Overlay pass last so snap and drag-state markers render above everything.
            rpass.set_pipeline(&self.overlay_pipeline);
            rpass.set_bind_group(0, &self.globals_bind_group, &[]);
            rpass.set_bind_group(1, &self.texture_bind_group, &[]);
            rpass.set_bind_group(2, &self.snap_markers_bind_group, &[]);
            rpass.draw(0..6, 0..1);
        }

        let cpu_perf_ms = frame_start.elapsed().as_secs_f64() * 1000.0;
        let gpu_start = Instant::now();
        self.queue.submit(Some(encoder.finish()));
        output.present();
        let gpu_perf_ms = gpu_start.elapsed().as_secs_f64() * 1000.0;
        let cpu_pass_x10 = (cpu_perf_ms.clamp(0.0, u32::MAX as f64 / 10.0) * 10.0).round() as u32;
        let gpu_pass_x10 = (gpu_perf_ms.clamp(0.0, u32::MAX as f64 / 10.0) * 10.0).round() as u32;
        const PERF_WINDOW: Duration = Duration::from_millis(100);
        let now = Instant::now();
        self.cpu_pass_x10 =
            Self::update_recent_peak(&mut self.cpu_pass_history, now, cpu_pass_x10, PERF_WINDOW);
        self.gpu_pass_x10 =
            Self::update_recent_peak(&mut self.gpu_pass_history, now, gpu_pass_x10, PERF_WINDOW);
        Ok(())
    }
}
