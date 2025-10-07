use std::{
    fs::{self},
    io::Cursor,
    sync::Arc,
};

use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    format::Format,
    image::{ImageDimensions, ImageViewAbstract, ImmutableImage, MipmapsCount, view::ImageView},
    memory::allocator::StandardMemoryAllocator,
};

#[derive(Clone)]
pub struct Texture {
    pub data: Vec<u8>,
    pub dimensions: ImageDimensions,
}

pub struct Material {
    albedo_ao_texture: Texture,
    surface_texture: Texture,

    pub albedo_ao: Option<Arc<ImageView<ImmutableImage>>>, // RGB = albedo, A = AO
    pub surface: Option<Arc<ImageView<ImmutableImage>>>, // RG = normal, B = roughness, A = metallic
}

fn create_texture(path: &str) -> Texture {
    let png_bytes = fs::read(path).unwrap();
    let cursor = Cursor::new(png_bytes);
    let decoder = png::Decoder::new(cursor);
    let mut reader = decoder.read_info().unwrap();
    let info = reader.info();
    let image_dimensions = ImageDimensions::Dim2d {
        width: info.width,
        height: info.height,
        array_layers: 1,
    };
    let mut image_data = Vec::new();
    let depth: u32 = match info.bit_depth {
        png::BitDepth::One => 1,
        png::BitDepth::Two => 2,
        png::BitDepth::Four => 4,
        png::BitDepth::Eight => 8,
        png::BitDepth::Sixteen => 16,
    };
    image_data.resize((info.width * info.height * depth) as usize, 0);
    reader.next_frame(&mut image_data).unwrap();

    Texture {
        data: image_data,
        dimensions: image_dimensions,
    }
}

fn load_texture(
    allocator: &StandardMemoryAllocator,
    command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    texture: &Texture,
) -> Arc<ImageView<ImmutableImage>> {
    let image = ImmutableImage::from_iter(
        allocator,
        texture.data.iter().cloned(),
        texture.dimensions,
        MipmapsCount::One,
        Format::R8G8B8A8_UNORM,
        command_buffer,
    )
    .unwrap();

    ImageView::new_default(image).unwrap()
}

impl Material {
    pub fn new(name: &str) -> Self {
        let albedo_ao_path = format!("assets/textures/{}_albedo_ao.png", name);
        let surface_path = format!("assets/textures/{}_material.png", name);

        Self {
            albedo_ao_texture: create_texture(&albedo_ao_path),
            surface_texture: create_texture(&surface_path),

            albedo_ao: None,
            surface: None,
        }
    }

    pub fn load(
        &mut self,
        allocator: &StandardMemoryAllocator,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        self.albedo_ao = Some(load_texture(
            allocator,
            command_buffer,
            &self.albedo_ao_texture,
        ));
        self.surface = Some(load_texture(
            allocator,
            command_buffer,
            &self.surface_texture,
        ));
    }

    pub fn unpack(&self) -> (Arc<dyn ImageViewAbstract>, Arc<dyn ImageViewAbstract>) {
        (
            self.albedo_ao
                .as_ref()
                .expect("albedo_ao not loaded")
                .clone() as Arc<dyn ImageViewAbstract>,
            self.surface.as_ref().expect("surface not loaded").clone()
                as Arc<dyn ImageViewAbstract>,
        )
    }
}
