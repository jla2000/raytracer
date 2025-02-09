use std::io::Cursor;

use image::{EncodableLayout, ImageFormat, ImageReader};
use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    Device, Extent3d, Queue, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension,
};

pub fn load_noise(queue: &Queue, device: &Device) -> TextureView {
    let noise_images = [
        include_bytes!("../noise/HDR_RGBA_0.png"),
        include_bytes!("../noise/HDR_RGBA_1.png"),
        include_bytes!("../noise/HDR_RGBA_2.png"),
        include_bytes!("../noise/HDR_RGBA_4.png"),
        include_bytes!("../noise/HDR_RGBA_5.png"),
        include_bytes!("../noise/HDR_RGBA_6.png"),
        include_bytes!("../noise/HDR_RGBA_7.png"),
    ];

    let mut noise_buffer = Vec::new();

    for noise_image in &noise_images {
        let mut reader = ImageReader::new(Cursor::new(noise_image));
        reader.set_format(ImageFormat::Png);

        let decoded = reader.decode().unwrap();
        let formmatted = decoded.to_rgba32f();
        noise_buffer.extend_from_slice(formmatted.as_bytes());
    }

    let noise_texture = device.create_texture_with_data(
        &queue,
        &TextureDescriptor {
            label: None,
            size: Extent3d {
                width: 256,
                height: 256,
                depth_or_array_layers: noise_images.len() as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        },
        TextureDataOrder::MipMajor,
        &noise_buffer,
    );

    noise_texture.create_view(&TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..Default::default()
    })
}
