use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use crate::skin::load_texture;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Icon, Window, WindowId},
};

use winit::platform::run_on_demand::EventLoopExtRunOnDemand;

const ROW_HEIGHT: f32 = 36.0;
const IMAGE_ROW_HEIGHT: f32 = 64.0;
const ROW_GAP: f32 = 6.0;
const TOP_PADDING: f32 = 12.0;
const QUESTION_HEIGHT: f32 = 36.0;
const SIDE_PADDING: f32 = 12.0;
const TEXT_SCALE: f32 = 3.0;
const INPUT_BOX_HEIGHT: f32 = 52.0;
const INPUT_BOX_BOTTOM_PADDING: f32 = 10.0;
const WINDOW_TITLE: &str = "osu-editor dialogue";
const IMAGE_INSET: f32 = 4.0;
const IMAGE_TEXT_GAP: f32 = 10.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DialogMode {
    Selection,
    TextPrompt,
}

pub struct DialogueApp {
    app: DialogueRuntime,
}

impl DialogueApp {
    pub fn new() -> Self {
        Self {
            app: DialogueRuntime::new(String::new(), Vec::new()),
        }
    }

    pub fn select(
        &mut self,
        event_loop: &mut EventLoop<()>,
        question: &str,
        entries: &[String],
    ) -> Option<usize> {
        if entries.is_empty() {
            return None;
        }

        self.app.prepare_selection(question, entries);
        if let Err(err) = event_loop.run_app_on_demand(&mut self.app) {
            println!("Selector event loop error: {err:?}");
            return None;
        }
        self.app.selected_index.take()
    }

    pub fn select_with_images(
        &mut self,
        event_loop: &mut EventLoop<()>,
        question: &str,
        entries: &[String],
        image_entries: &[Option<Vec<u8>>],
    ) -> Option<usize> {
        if entries.is_empty() {
            return None;
        }

        self.app
            .prepare_selection_with_images(question, entries, image_entries);
        if let Err(err) = event_loop.run_app_on_demand(&mut self.app) {
            println!("Selector event loop error: {err:?}");
            return None;
        }
        self.app.selected_index.take()
    }

    pub fn prompt_text(
        &mut self,
        event_loop: &mut EventLoop<()>,
        title: &str,
        prompt: &str,
    ) -> Option<String> {
        self.app.prepare_text_prompt(title, prompt);
        if let Err(err) = event_loop.run_app_on_demand(&mut self.app) {
            println!("Selector event loop error: {err:?}");
            return None;
        }
        self.app.submitted_text.take()
    }

    pub fn confirm(&mut self, event_loop: &mut EventLoop<()>, question: &str) -> bool {
        let options = vec!["Yes".to_string(), "No".to_string()];
        matches!(self.select(event_loop, question, &options), Some(0))
    }
}

struct DialogueRuntime {
    question_text: String,
    entries: Vec<String>,
    window: Option<Arc<Window>>,
    renderer: Option<SelectorGpu>,
    width: u32,
    height: u32,
    cursor_x: f64,
    cursor_y: f64,
    hovered_index: Option<usize>,
    dragging_left: bool,
    drag_started_on_item: bool,
    scroll_px: f64,
    mode: DialogMode,
    prompt_label: String,
    input_text: String,
    selection_with_images: bool,
    selection_images: Vec<Option<Vec<u8>>>,
    selection_images_dirty: bool,
    submitted_text: Option<String>,
    selected_index: Option<usize>,
}

#[derive(Clone, Copy)]
struct OptionRowCoords {
    index: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl DialogueRuntime {
    fn new(question_text: String, entries: Vec<String>) -> Self {
        Self {
            question_text,
            entries,
            window: None,
            renderer: None,
            width: 560,
            height: 720,
            cursor_x: 0.0,
            cursor_y: 0.0,
            hovered_index: None,
            dragging_left: false,
            drag_started_on_item: false,
            scroll_px: 0.0,
            mode: DialogMode::Selection,
            prompt_label: String::new(),
            input_text: String::new(),
            selection_with_images: false,
            selection_images: Vec::new(),
            selection_images_dirty: true,
            submitted_text: None,
            selected_index: None,
        }
    }

    fn row_height(&self) -> f64 {
        if self.selection_with_images {
            IMAGE_ROW_HEIGHT as f64
        } else {
            ROW_HEIGHT as f64
        }
    }

    fn list_bottom(&self) -> f64 {
        (self.height as f64 - INPUT_BOX_HEIGHT as f64 - INPUT_BOX_BOTTOM_PADDING as f64).max(0.0)
    }

    fn list_top(&self) -> f64 {
        TOP_PADDING as f64 + QUESTION_HEIGHT as f64
    }

    fn total_content_height(&self) -> f64 {
        self.list_top() + TOP_PADDING as f64 + (self.row_height() + ROW_GAP as f64) * self.entries.len() as f64
    }

    fn prepare_selection(&mut self, question: &str, entries: &[String]) {
        self.mode = DialogMode::Selection;
        self.question_text = question.to_string();
        self.entries = entries.to_vec();
        self.hovered_index = None;
        self.dragging_left = false;
        self.drag_started_on_item = false;
        self.selected_index = None;
        self.submitted_text = None;
        self.scroll_px = 0.0;
        self.prompt_label = "Type index/name, press Enter".to_string();
        self.input_text.clear();
        self.selection_with_images = false;
        self.selection_images.clear();
        self.selection_images_dirty = true;

        if let Some(renderer) = self.renderer.as_mut() {
            renderer.sync_selection_images(&self.selection_images);
        }

        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
            window.request_redraw();
        }
    }

    fn prepare_selection_with_images(
        &mut self,
        question: &str,
        entries: &[String],
        image_entries: &[Option<Vec<u8>>],
    ) {
        self.mode = DialogMode::Selection;
        self.question_text = question.to_string();
        self.entries = entries.to_vec();
        self.hovered_index = None;
        self.dragging_left = false;
        self.drag_started_on_item = false;
        self.selected_index = None;
        self.submitted_text = None;
        self.scroll_px = 0.0;
        self.prompt_label = "Type index/name, press Enter".to_string();
        self.input_text.clear();
        self.selection_with_images = true;
        self.selection_images = (0..self.entries.len())
            .map(|i| image_entries.get(i).cloned().flatten())
            .collect();
        self.selection_images_dirty = true;

        if let Some(renderer) = self.renderer.as_mut() {
            renderer.sync_selection_images(&self.selection_images);
            self.selection_images_dirty = false;
        }

        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
            window.request_redraw();
        }
    }

    fn prepare_text_prompt(&mut self, title: &str, prompt: &str) {
        self.mode = DialogMode::TextPrompt;
        self.question_text = if prompt.is_empty() {
            title.to_string()
        } else {
            prompt.to_string()
        };
        self.entries.clear();
        self.hovered_index = None;
        self.dragging_left = false;
        self.drag_started_on_item = false;
        self.selected_index = None;
        self.submitted_text = None;
        self.scroll_px = 0.0;
        self.prompt_label = "Type value, press Enter".to_string();
        self.input_text.clear();
        self.selection_with_images = false;
        self.selection_images.clear();
        self.selection_images_dirty = true;

        if let Some(renderer) = self.renderer.as_mut() {
            renderer.sync_selection_images(&self.selection_images);
        }

        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
            window.request_redraw();
        }
    }

    fn max_scroll(&self) -> f64 {
        (self.total_content_height() - self.list_bottom()).max(0.0)
    }

    fn clamp_scroll(&mut self) {
        self.scroll_px = self.scroll_px.clamp(0.0, self.max_scroll());
    }

    fn item_bounds(&self, index: usize) -> (f64, f64, f64, f64) {
        let x = SIDE_PADDING as f64;
        let y = self.list_top() + index as f64 * (self.row_height() + ROW_GAP as f64) - self.scroll_px;
        let w = (self.width as f64 - SIDE_PADDING as f64 * 2.0).max(0.0);
        let h = self.row_height();
        (x, y, w, h)
    }

    fn hit_test_index(&self, x: f64, y: f64) -> Option<usize> {
        if y > self.list_bottom() {
            return None;
        }
        for i in 0..self.entries.len() {
            let (rx, ry, rw, rh) = self.item_bounds(i);
            if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
                return Some(i);
            }
        }
        None
    }

    fn update_hover(&mut self) {
        self.hovered_index = self.hit_test_index(self.cursor_x, self.cursor_y);
    }

    fn render(&mut self) {
        let list_bottom = self.list_bottom() as f32;
        let input_text = self.input_text.clone();
        let prompt_label = self.prompt_label.clone();
        let question_text = self.question_text.clone();
        let is_selection_mode = self.mode == DialogMode::Selection;
        if self.selection_images_dirty {
            if let Some(renderer) = self.renderer.as_mut() {
                renderer.sync_selection_images(&self.selection_images);
            }
            self.selection_images_dirty = false;
        }
        let mut option_rows: Vec<OptionRowCoords> = Vec::new();
        if is_selection_mode {
            for i in 0..self.entries.len() {
                let (x, y, w, h) = self.item_bounds(i);
                if y + h < 0.0 || y > list_bottom as f64 {
                    continue;
                }
                option_rows.push(OptionRowCoords {
                    index: i,
                    x: x as f32,
                    y: y as f32,
                    w: w as f32,
                    h: h as f32,
                });
            }
        }
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        renderer.render(
            self.width,
            self.height,
            &self.entries,
            self.hovered_index,
            self.dragging_left,
            option_rows.as_slice(),
            input_text.as_str(),
            prompt_label.as_str(),
            question_text.as_str(),
            is_selection_mode,
            self.selection_with_images,
        );
    }

    fn submit_selection_from_text(&mut self) -> bool {
        let input = self.input_text.trim();
        if input.is_empty() {
            return false;
        }

        if let Ok(idx) = input.parse::<usize>() {
            if idx < self.entries.len() {
                self.selected_index = Some(idx);
                self.input_text.clear();
                return true;
            }
            return false;
        }

        let needle = input.to_ascii_lowercase();
        if let Some((idx, _)) = self
            .entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.to_ascii_lowercase().contains(&needle))
        {
            self.selected_index = Some(idx);
            self.input_text.clear();
            return true;
        }

        false
    }

    fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn cleanup_after_interaction(&mut self) {
        self.hovered_index = None;
        self.dragging_left = false;
        self.drag_started_on_item = false;
        self.scroll_px = 0.0;
        self.question_text.clear();
        self.entries.clear();
        self.prompt_label.clear();
        self.input_text.clear();
        self.selection_with_images = false;
        self.selection_images.clear();
        self.selection_images_dirty = true;

        self.render();
    }
}

impl ApplicationHandler for DialogueRuntime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);

        if self.window.is_some() {
            if let Some(window) = self.window.as_ref() {
                window.set_visible(true);
                window.request_redraw();
            }
            return;
        }

        let window_icon = load_window_icon();

        let attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(self.width, self.height))
            .with_min_inner_size(LogicalSize::new(260, 180))
            .with_visible(true)
            .with_resizable(true)
            .with_decorations(true)
            .with_active(true)
            .with_window_icon(window_icon);

        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(err) => {
                println!("Failed to create selection window: {err}");
                event_loop.exit();
                return;
            }
        };

        let size = window.inner_size();
        self.width = size.width.max(1);
        self.height = size.height.max(1);

        self.renderer = match SelectorGpu::new(window.clone(), self.width, self.height) {
            Some(renderer) => Some(renderer),
            None => {
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window);
        self.clamp_scroll();
        self.update_hover();
        self.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.selected_index = None;
                self.submitted_text = None;
                self.cleanup_after_interaction();
                if let Some(window) = self.window.as_ref() {
                    window.set_visible(false);
                }
                event_loop.exit();
            }
            WindowEvent::Destroyed => {
                self.window = None;
                self.renderer = None;
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                self.width = new_size.width.max(1);
                self.height = new_size.height.max(1);
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(self.width, self.height);
                }
                self.clamp_scroll();
                self.update_hover();
                self.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_x = position.x;
                self.cursor_y = position.y;
                self.update_hover();
                self.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button != MouseButton::Left {
                    return;
                }
                if self.mode != DialogMode::Selection {
                    return;
                }
                match state {
                    ElementState::Pressed => {
                        self.dragging_left = true;
                        self.drag_started_on_item = self.hovered_index.is_some();
                    }
                    ElementState::Released => {
                        if self.dragging_left
                            && self.drag_started_on_item
                            && self.hovered_index.is_some()
                        {
                            self.selected_index = self.hovered_index;
                            self.cleanup_after_interaction();
                            event_loop.exit();
                            return;
                        }
                        self.dragging_left = false;
                        self.drag_started_on_item = false;
                    }
                }
                self.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                match &event.logical_key {
                    Key::Named(NamedKey::Enter) => {
                        if self.mode == DialogMode::Selection {
                            if self.submit_selection_from_text() {
                                self.cleanup_after_interaction();
                                event_loop.exit();
                                return;
                            }
                        } else {
                            let submitted = self.input_text.trim().to_string();
                            if !submitted.is_empty() {
                                self.submitted_text = Some(submitted);
                                self.cleanup_after_interaction();
                                event_loop.exit();
                                return;
                            }
                        }
                    }
                    Key::Named(NamedKey::Backspace) => {
                        self.input_text.pop();
                    }
                    _ => {
                        if let Some(text) = event.text.as_ref() {
                            for ch in text.chars() {
                                if !ch.is_control() {
                                    self.input_text.push(ch);
                                }
                            }
                        }
                    }
                }

                self.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta_px = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -(y as f64) * 36.0,
                    MouseScrollDelta::PixelDelta(p) => -p.y,
                };
                self.scroll_px += delta_px;
                self.clamp_scroll();
                self.update_hover();
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                self.render();
                self.request_redraw();
            }
            _ => {}
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ScreenUniform {
    screen_size: [f32; 2],
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RectInstance {
    pos: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TextVertex {
    pos: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
    ch: u32,
    _pad: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ImageInstance {
    pos: [f32; 2],
    size: [f32; 2],
}

struct ImageDraw {
    image_slot: usize,
    instance_idx: usize,
}

struct RowImageTexture {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

struct SelectorGpu {
    _window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    rect_pipeline: wgpu::RenderPipeline,
    rect_instance_buffer: wgpu::Buffer,
    rect_instance_capacity: usize,

    text_pipeline: wgpu::RenderPipeline,
    text_vertex_buffer: wgpu::Buffer,
    text_vertex_capacity: usize,

    image_bind_group_layout: wgpu::BindGroupLayout,
    image_sampler: wgpu::Sampler,
    image_pipeline: wgpu::RenderPipeline,
    image_instance_buffer: wgpu::Buffer,
    image_instance_capacity: usize,
    selection_images: Vec<Option<RowImageTexture>>,
}

impl SelectorGpu {
    fn new(window: Arc<Window>, width: u32, height: u32) -> Option<Self> {
        let instance = wgpu::Instance::default();
        let surface = match instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(err) => {
                println!("Failed to create selector surface: {err}");
                return None;
            }
        };

        let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })) {
            Ok(adapter) => adapter,
            Err(err) => {
                println!("Failed to request selector adapter: {err}");
                return None;
            }
        };

        let (device, queue) = match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("selector-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::default(),
        })) {
            Ok(pair) => pair,
            Err(err) => {
                println!("Failed to request selector device: {err}");
                return None;
            }
        };

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let uniform = ScreenUniform {
            screen_size: [config.width as f32, config.height as f32],
            _pad: [0.0, 0.0],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("selector-uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("selector-uniform-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("selector-uniform-bind-group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("selector-rect-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu/shaders/60_selector_rect.wgsl").into()),
        });
        let rect_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("selector-rect-pipeline-layout"),
            bind_group_layouts: &[&uniform_layout],
            immediate_size: 0,
        });

        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("selector-rect-pipeline"),
            layout: Some(&rect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32x4
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let rect_instance_capacity = 512usize;
        let rect_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selector-rect-instance-buffer"),
            size: (rect_instance_capacity * std::mem::size_of::<RectInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("selector-text-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu/shaders/61_selector_text.wgsl").into()),
        });
        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("selector-text-pipeline-layout"),
            bind_group_layouts: &[&uniform_layout],
            immediate_size: 0,
        });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("selector-text-pipeline"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TextVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32x4,
                        3 => Uint32
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let text_vertex_capacity = 4096usize;
        let text_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selector-text-vertex-buffer"),
            size: (text_vertex_capacity * std::mem::size_of::<TextVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let image_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("selector-image-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let image_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("selector-image-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let image_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("selector-image-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu/shaders/62_selector_image.wgsl").into()),
        });
        let image_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("selector-image-pipeline-layout"),
            bind_group_layouts: &[&uniform_layout, &image_bind_group_layout],
            immediate_size: 0,
        });
        let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("selector-image-pipeline"),
            layout: Some(&image_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &image_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ImageInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &image_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let image_instance_capacity = 256usize;
        let image_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selector-image-instance-buffer"),
            size: (image_instance_capacity * std::mem::size_of::<ImageInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Some(Self {
            _window: window,
            surface,
            device,
            queue,
            config,
            uniform_buffer,
            uniform_bind_group,
            rect_pipeline,
            rect_instance_buffer,
            rect_instance_capacity,
            text_pipeline,
            text_vertex_buffer,
            text_vertex_capacity,
            image_bind_group_layout,
            image_sampler,
            image_pipeline,
            image_instance_buffer,
            image_instance_capacity,
            selection_images: Vec::new(),
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
        let uniform = ScreenUniform {
            screen_size: [self.config.width as f32, self.config.height as f32],
            _pad: [0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    fn ensure_rect_capacity(&mut self, needed: usize) {
        if needed <= self.rect_instance_capacity {
            return;
        }
        self.rect_instance_capacity = needed.next_power_of_two().max(64);
        self.rect_instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selector-rect-instance-buffer"),
            size: (self.rect_instance_capacity * std::mem::size_of::<RectInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }

    fn ensure_text_capacity(&mut self, needed: usize) {
        if needed <= self.text_vertex_capacity {
            return;
        }
        self.text_vertex_capacity = needed.next_power_of_two().max(1024);
        self.text_vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selector-text-vertex-buffer"),
            size: (self.text_vertex_capacity * std::mem::size_of::<TextVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }

    fn ensure_image_capacity(&mut self, needed: usize) {
        if needed <= self.image_instance_capacity {
            return;
        }
        self.image_instance_capacity = needed.next_power_of_two().max(64);
        self.image_instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selector-image-instance-buffer"),
            size: (self.image_instance_capacity * std::mem::size_of::<ImageInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }

    fn create_row_image_texture(&self, image_bytes: &[u8]) -> Option<RowImageTexture> {
        let decoded = load_texture(image_bytes)?;
        if decoded.width == 0 || decoded.height == 0 {
            return None;
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("selector-row-image"),
            size: wgpu::Extent3d {
                width: decoded.width,
                height: decoded.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &decoded.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * decoded.width),
                rows_per_image: Some(decoded.height),
            },
            wgpu::Extent3d {
                width: decoded.width,
                height: decoded.height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("selector-row-image-bind-group"),
            layout: &self.image_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.image_sampler),
                },
            ],
        });

        Some(RowImageTexture {
            _texture: texture,
            _view: view,
            bind_group,
        })
    }

    fn sync_selection_images(&mut self, image_entries: &[Option<Vec<u8>>]) {
        self.selection_images = image_entries
            .iter()
            .map(|bytes| bytes.as_ref().and_then(|b| self.create_row_image_texture(b.as_slice())))
            .collect();
    }

    fn render(
        &mut self,
        width: u32,
        height: u32,
        entries: &[String],
        hovered_index: Option<usize>,
        dragging_left: bool,
        option_rows: &[OptionRowCoords],
        input_text: &str,
        prompt_label: &str,
        question_text: &str,
        is_selection_mode: bool,
        selection_with_images: bool,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        if self.config.width != width || self.config.height != height {
            self.resize(width, height);
        }

        let mut rects: Vec<RectInstance> = Vec::with_capacity(entries.len() * 3);
        let mut text_vertices: Vec<TextVertex> = Vec::with_capacity(entries.len() * 80);
        let mut image_instances: Vec<ImageInstance> = Vec::with_capacity(option_rows.len());
        let mut image_draws: Vec<ImageDraw> = Vec::new();

        push_text(
            &mut text_vertices,
            question_text,
            SIDE_PADDING,
            TOP_PADDING + 6.0,
            TEXT_SCALE,
            [0.92, 0.94, 0.97, 1.0],
            width as f32 - SIDE_PADDING,
        );

        if is_selection_mode {
            for row in option_rows {
                let i = row.index;
                let entry = &entries[i];
                let x = row.x;
                let y = row.y;
                let w = row.w;
                let h = row.h;

                let mut color = if i % 2 == 0 {
                    [45.0 / 255.0, 45.0 / 255.0, 56.0 / 255.0, 1.0]
                } else {
                    [39.0 / 255.0, 39.0 / 255.0, 50.0 / 255.0, 1.0]
                };
                if hovered_index == Some(i) {
                    color = if dragging_left {
                        [88.0 / 255.0, 96.0 / 255.0, 148.0 / 255.0, 1.0]
                    } else {
                        [70.0 / 255.0, 78.0 / 255.0, 126.0 / 255.0, 1.0]
                    };
                }

                rects.push(RectInstance {
                    pos: [x, y],
                    size: [w, h],
                    color,
                });

                let mut text_x = x + 10.0;
                if selection_with_images {
                    let image_size = (h - IMAGE_INSET * 2.0).max(0.0);
                    let image_x = x + IMAGE_INSET;
                    let image_y = y + IMAGE_INSET;

                    rects.push(RectInstance {
                        pos: [image_x, image_y],
                        size: [image_size, image_size],
                        color: [0.0, 0.0, 0.0, 1.0],
                    });

                    if self.selection_images.get(i).and_then(|img| img.as_ref()).is_some() {
                        image_instances.push(ImageInstance {
                            pos: [image_x, image_y],
                            size: [image_size, image_size],
                        });
                        image_draws.push(ImageDraw {
                            image_slot: i,
                            instance_idx: image_instances.len() - 1,
                        });
                    }

                    text_x = image_x + image_size + IMAGE_TEXT_GAP;
                }

                let indexed = format!("[{}] {}", i, entry);
                let text_y = y + (h - (7.0 * TEXT_SCALE)) * 0.5;
                push_text(
                    &mut text_vertices,
                    indexed.as_str(),
                    text_x,
                    text_y,
                    TEXT_SCALE,
                    [232.0 / 255.0, 236.0 / 255.0, 248.0 / 255.0, 1.0],
                    width as f32 - 10.0,
                );
            }
        }

        let input_w = (width as f32 - SIDE_PADDING * 2.0).max(0.0);
        let input_y = (height as f32 - INPUT_BOX_HEIGHT - INPUT_BOX_BOTTOM_PADDING).max(0.0);

        rects.push(RectInstance {
            pos: [SIDE_PADDING, input_y],
            size: [input_w, INPUT_BOX_HEIGHT],
            color: [0.22, 0.22, 0.26, 1.0],
        });
        rects.push(RectInstance {
            pos: [SIDE_PADDING + 1.0, input_y + 1.0],
            size: [input_w - 2.0, INPUT_BOX_HEIGHT - 2.0],
            color: [0.16, 0.16, 0.20, 1.0],
        });

        let prompt = if input_text.is_empty() {
            prompt_label
        } else {
            input_text
        };
        push_text(
            &mut text_vertices,
            prompt,
            SIDE_PADDING + 10.0,
            input_y + 14.0,
            TEXT_SCALE,
            [0.92, 0.94, 0.97, 1.0],
            width as f32 - SIDE_PADDING - 10.0,
        );

        self.ensure_rect_capacity(rects.len());
        self.ensure_text_capacity(text_vertices.len());
        self.ensure_image_capacity(image_instances.len());

        if !rects.is_empty() {
            self.queue
                .write_buffer(&self.rect_instance_buffer, 0, bytemuck::cast_slice(&rects));
        }
        if !text_vertices.is_empty() {
            self.queue.write_buffer(
                &self.text_vertex_buffer,
                0,
                bytemuck::cast_slice(&text_vertices),
            );
        }
        if !image_instances.is_empty() {
            self.queue.write_buffer(
                &self.image_instance_buffer,
                0,
                bytemuck::cast_slice(&image_instances),
            );
        }

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                println!("Selector surface out of memory.");
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => return,
            Err(wgpu::SurfaceError::Other) => return,
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("selector-encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("selector-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 22.0 / 255.0,
                            g: 22.0 / 255.0,
                            b: 28.0 / 255.0,
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

            if !rects.is_empty() {
                pass.set_pipeline(&self.rect_pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.set_vertex_buffer(0, self.rect_instance_buffer.slice(..));
                pass.draw(0..6, 0..rects.len() as u32);
            }

            if !text_vertices.is_empty() {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.set_vertex_buffer(0, self.text_vertex_buffer.slice(..));
                pass.draw(0..6, 0..text_vertices.len() as u32);
            }

            if !image_draws.is_empty() {
                pass.set_pipeline(&self.image_pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);

                let stride = std::mem::size_of::<ImageInstance>() as u64;
                for draw in image_draws {
                    let Some(Some(image)) = self.selection_images.get(draw.image_slot) else {
                        continue;
                    };
                    pass.set_bind_group(1, &image.bind_group, &[]);
                    let offset = draw.instance_idx as u64 * stride;
                    pass.set_vertex_buffer(0, self.image_instance_buffer.slice(offset..offset + stride));
                    pass.draw(0..6, 0..1);
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

fn push_text(
    out: &mut Vec<TextVertex>,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    color: [f32; 4],
    max_width: f32,
) {
    let mut cx = x;
    let char_w = 5.0 * scale;
    let char_h = 7.0 * scale;
    let advance = char_w + scale * 1.5;

    for ch in text.chars() {
        if cx + char_w > max_width {
            break;
        }

        let code = if (ch as u32) < 128 { ch as u32 } else { '?' as u32 };
        out.push(TextVertex {
            pos: [cx, y],
            size: [char_w, char_h],
            color,
            ch: code,
            _pad: [0, 0, 0],
        });

        cx += advance;
    }
}

fn load_window_icon() -> Option<Icon> {
    match std::fs::read("assets/icon.png") {
        Ok(bytes) => match load_texture(&bytes) {
            Some(tex) => match Icon::from_rgba(tex.rgba, tex.width, tex.height) {
                Ok(icon) => Some(icon),
                Err(err) => {
                    println!("Failed to create dialogue window icon: {err}");
                    None
                }
            },
            None => {
                println!("Failed to decode assets/icon.png for dialogue window icon");
                None
            }
        },
        Err(err) => {
            println!("Failed to read assets/icon.png for dialogue window icon: {err}");
            None
        }
    }
}

