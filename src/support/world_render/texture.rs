use crate::{Filter, Format, Sampler, Texture, WorldTexture, WrappingMode};
use anyhow::Result;

pub fn from_world_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world_texture: &WorldTexture,
    label: &str,
) -> Result<Texture> {
    let size = wgpu::Extent3d {
        width: world_texture.description.width,
        height: world_texture.description.height,
        depth_or_array_layers: 1,
    };

    let format = map_texture_format(world_texture.description.format);

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &world_texture.description.pixels,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(world_texture.description.bytes_per_row()),
            rows_per_image: std::num::NonZeroU32::new(world_texture.description.height),
        },
        size,
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("WorldTextureView"),
        format: Some(format),
        dimension: Some(wgpu::TextureViewDimension::D2),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    });

    let sampler = device.create_sampler(&map_sampler(&world_texture.sampler));

    Ok(Texture {
        texture,
        view,
        sampler,
    })
}

fn map_texture_format(texture_format: Format) -> wgpu::TextureFormat {
    println!("texture format: {:?}", texture_format);
    // FIXME: Map texture formats
    wgpu::TextureFormat::Rgba8UnormSrgb
}

fn map_sampler(sampler: &Sampler) -> wgpu::SamplerDescriptor<'static> {
    let min_filter = match sampler.min_filter {
        Filter::Linear => wgpu::FilterMode::Linear,
        Filter::Nearest => wgpu::FilterMode::Nearest,
    };

    let mipmap_filter = match sampler.min_filter {
        Filter::Linear => wgpu::FilterMode::Linear,
        Filter::Nearest => wgpu::FilterMode::Nearest,
    };

    let mag_filter = match sampler.mag_filter {
        Filter::Nearest => wgpu::FilterMode::Nearest,
        Filter::Linear => wgpu::FilterMode::Linear,
    };

    let address_mode_u = match sampler.wrap_s {
        WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        WrappingMode::Repeat => wgpu::AddressMode::Repeat,
    };

    let address_mode_v = match sampler.wrap_t {
        WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        WrappingMode::Repeat => wgpu::AddressMode::Repeat,
    };

    let address_mode_w = wgpu::AddressMode::Repeat;

    wgpu::SamplerDescriptor {
        address_mode_u,
        address_mode_v,
        address_mode_w,
        mag_filter,
        min_filter,
        mipmap_filter,
        ..Default::default()
    }
}
