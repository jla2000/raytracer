use std::io::Cursor;

use image::{EncodableLayout, GenericImageView, ImageFormat, ImageReader};
use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    Device, Extent3d, Queue, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

pub fn load_skybox(device: &Device, queue: &Queue) -> TextureView {
    let skybox_bytes = include_bytes!("../assets/skybox/zwartkops_straight_afternoon_4k.hdr");
    let reader = ImageReader::with_format(Cursor::new(skybox_bytes), ImageFormat::Hdr);
    let image = reader.decode().unwrap();
    let image_formatted = image.to_rgba32f();
    let (width, height) = image.dimensions();

    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            label: None,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        },
        TextureDataOrder::MipMajor,
        image_formatted.as_bytes(),
    );

    texture.create_view(&TextureViewDescriptor::default())
}
