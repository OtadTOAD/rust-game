use std::sync::Arc;

use image::ImageReader;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    format::Format,
    image::{ImageDimensions, ImmutableImage, MipmapsCount, view::ImageView},
    memory::allocator::StandardMemoryAllocator,
};

pub struct Skybox {
    pixels_data: Vec<[f32; 4]>,
    width: u32,
    height: u32,
    pub image_view: Option<Arc<ImageView<ImmutableImage>>>,
}

impl Skybox {
    pub fn new(file_path: &str) -> Self {
        let img = ImageReader::open(file_path)
            .unwrap()
            .decode()
            .unwrap()
            .to_rgba32f();

        let (width, height) = img.dimensions();
        let pixels_data: Vec<[f32; 4]> = img
            .chunks_exact(4)
            .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
            .collect();

        Self {
            pixels_data,
            width,
            height,
            image_view: None,
        }
    }

    pub fn load(
        &mut self,
        allocator: &StandardMemoryAllocator,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        let dimensions = ImageDimensions::Dim2d {
            width: self.width,
            height: self.height,
            array_layers: 1,
        };

        let image = ImmutableImage::from_iter(
            allocator,
            self.pixels_data.iter().cloned(),
            dimensions,
            MipmapsCount::One,
            Format::R32G32B32A32_SFLOAT,
            command_buffer,
        )
        .unwrap();

        self.image_view = Some(ImageView::new_default(image).unwrap());
    }
}
