pub fn normalize_msaa_samples(samples: u32) -> u32 {
    match samples {
        0 | 1 => 1,
        2 => 2,
        3 | 4 => 4,
        _ => 8,
    }
}

pub fn select_supported_msaa_samples(
    adapter: &wgpu::Adapter,
    format: wgpu::TextureFormat,
    requested: u32,
) -> u32 {
    // WebGPU guarantees 1x and 4x for common swapchain formats.
    // 2x/8x (and others) are adapter-specific and require the
    // TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES feature to be enabled.
    let requested = normalize_msaa_samples(requested);

    let features = adapter.get_texture_format_features(format);
    let flags = features.flags;

    let mut supported = vec![1u32];
    if flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X2) {
        supported.push(2);
    }
    if flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4) {
        supported.push(4);
    }
    if flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X8) {
        supported.push(8);
    }

    supported.sort_unstable();
    supported
        .into_iter()
        .filter(|s| *s <= requested)
        .max()
        .unwrap_or(1)
}

pub fn create_msaa_target(
    device: &wgpu::Device,
    surface_config: &wgpu::SurfaceConfiguration,
    samples: u32,
) -> (Option<wgpu::Texture>, Option<wgpu::TextureView>) {
    if samples <= 1 {
        return (None, None);
    }

    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa color"),
        size: wgpu::Extent3d {
            width: surface_config.width.max(1),
            height: surface_config.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format: surface_config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    (Some(tex), Some(view))
}
