use crate::skin::Texture;

pub fn upload_texture_2d_srgb(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    tex: &Texture,
    pad_to_nominal: bool,
) -> (wgpu::Texture, wgpu::TextureView) {
    let w_src = tex.width.max(1);
    let h_src = tex.height.max(1);

    // Pad undersized skin sprites to the nominal resolution so they don't get implicitly
    // scaled down just because the author left out transparent borders.
    let nominal_px: u32 = if pad_to_nominal {
        if tex.is_2x { 256 } else { 128 }
    } else {
        1
    };

    let w = w_src.max(nominal_px);
    let h = h_src.max(nominal_px);
    let tex_size = wgpu::Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: tex_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // Avoid panics/crashes on invalid buffers.
    if tex.width > 0
        && tex.height > 0
        && tex.rgba.len() == (tex.width as usize) * (tex.height as usize) * 4
    {
        if w == tex.width && h == tex.height {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                tex.rgba.as_slice(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * tex.width),
                    rows_per_image: Some(tex.height),
                },
                wgpu::Extent3d {
                    width: tex.width,
                    height: tex.height,
                    depth_or_array_layers: 1,
                },
            );
        } else {
            // Center the source image in a transparent padded canvas.
            let mut padded = vec![0u8; (w as usize) * (h as usize) * 4];
            let x_off = ((w - tex.width) / 2) as usize;
            let y_off = ((h - tex.height) / 2) as usize;
            let dst_stride = (w as usize) * 4;
            let src_stride = (tex.width as usize) * 4;
            for y in 0..(tex.height as usize) {
                let dst_y = y + y_off;
                let dst_i = dst_y * dst_stride + x_off * 4;
                let src_i = y * src_stride;
                padded[dst_i..dst_i + src_stride]
                    .copy_from_slice(&tex.rgba[src_i..src_i + src_stride]);
            }

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                padded.as_slice(),
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
    }

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

pub fn upload_texture_2d_array_srgb(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    frames: &[Texture],
    pad_to_nominal: bool,
) -> anyhow::Result<(wgpu::Texture, wgpu::TextureView)> {
    if frames.is_empty() {
        anyhow::bail!("{label}: no frames");
    }

    let mut max_w: u32 = 0;
    let mut max_h: u32 = 0;
    let mut any_2x = false;
    for (i, t) in frames.iter().enumerate() {
        if t.width == 0 || t.height == 0 {
            anyhow::bail!("{label}: frame {i} has invalid size (0x0)");
        }
        if t.rgba.len() != (t.width as usize) * (t.height as usize) * 4 {
            anyhow::bail!("{label}: frame {i} has invalid RGBA buffer size");
        }
        any_2x |= t.is_2x;
        max_w = max_w.max(t.width);
        max_h = max_h.max(t.height);
    }

    let nominal_px: u32 = if pad_to_nominal {
        if any_2x { 256 } else { 128 }
    } else {
        1
    };

    let layer_w = max_w.max(nominal_px);
    let layer_h = max_h.max(nominal_px);
    let layers = frames.len() as u32;

    let tex_desc = wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: layer_w,
            height: layer_h,
            depth_or_array_layers: layers,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    };

    let texture = device.create_texture(&tex_desc);

    // Clear each layer to transparent (texture contents are otherwise undefined).
    let clear_rgba = vec![0u8; (layer_w * layer_h * 4) as usize];
    for (layer, t) in frames.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
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
                bytes_per_row: Some(4 * layer_w),
                rows_per_image: Some(layer_h),
            },
            wgpu::Extent3d {
                width: layer_w,
                height: layer_h,
                depth_or_array_layers: 1,
            },
        );

        let x_off_px = (layer_w - t.width) / 2;
        let y_off_px = (layer_h - t.height) / 2;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: x_off_px,
                    y: y_off_px,
                    z: layer as u32,
                },
                aspect: wgpu::TextureAspect::All,
            },
            t.rgba.as_slice(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * t.width),
                rows_per_image: Some(t.height),
            },
            wgpu::Extent3d {
                width: t.width,
                height: t.height,
                depth_or_array_layers: 1,
            },
        );
    }

    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("array texture view"),
        format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(layers),
        ..Default::default()
    });

    Ok((texture, view))
}
