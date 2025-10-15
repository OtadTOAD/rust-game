use crate::engine::VoxelWorld;
use crate::system::dummy_vertex::DummyVertex;

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, PrimaryAutoCommandBuffer,
    RenderPassBeginInfo, SubpassContents,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{AttachmentImage, ImageAccess, StorageImage, SwapchainImage};
use vulkano::instance::debug::{
    DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
    DebugUtilsMessengerCreateInfo,
};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::StandardMemoryAllocator;

use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::rasterization::{CullMode, RasterizationState};
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::swapchain::{
    self, AcquireError, PresentMode, Surface, Swapchain, SwapchainAcquireFuture,
    SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano::{Version, VulkanLibrary};

use vulkano_win::VkSurfaceBuild;

use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use nalgebra_glm::{TMat4, TVec3, Vec2, half_pi, identity, inverse, perspective, vec2, vec3};

use std::mem;
use std::sync::Arc;

vulkano::impl_vertex!(DummyVertex, position);

mod voxel_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/system/shaders/voxel.vert",
    }
}

mod voxel_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/system/shaders/voxel.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};

            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

#[derive(Debug, Clone)]
enum RenderStage {
    Stopped,
    Voxel,
    NeedsRedraw,
}

pub struct System {
    surface: Arc<Surface>,
    pub device: Arc<Device>,
    queue: Arc<Queue>,
    vp: VP,
    swapchain: Arc<Swapchain>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    descriptor_set_allocator: StandardDescriptorSetAllocator,
    command_buffer_allocator: StandardCommandBufferAllocator,
    render_pass: Arc<RenderPass>,
    voxel_pipeline: Arc<GraphicsPipeline>,
    vp_buffer: Arc<CpuAccessibleBuffer<voxel_frag::ty::CameraUBO>>,
    dummy_verts: Arc<CpuAccessibleBuffer<[DummyVertex]>>,
    vp_set: Arc<PersistentDescriptorSet>,
    viewport: Viewport,
    framebuffers: Vec<Arc<Framebuffer>>,
    render_stage: RenderStage,
    commands: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    image_index: u32,
    acquire_future: Option<SwapchainAcquireFuture>,

    voxel_image: Option<Arc<StorageImage>>,
    voxel_image_view: Option<Arc<ImageView<StorageImage>>>,
}

#[derive(Debug, Clone)]
struct VP {
    view: TMat4<f32>,
    projection: TMat4<f32>,
    camera_pos: TVec3<f32>,
    resolution: Vec2,
}

impl VP {
    fn new() -> VP {
        VP {
            view: identity(),
            projection: identity(),
            camera_pos: vec3(0.0, 0.0, 0.0),
            resolution: vec2(600.0, 800.0),
        }
    }
}

fn get_camera_ubo(vp: &VP) -> voxel_frag::ty::CameraUBO {
    let inv_proj = inverse(&vp.projection);
    let inv_view = inverse(&vp.view);

    voxel_frag::ty::CameraUBO {
        inv_proj: inv_proj.into(),
        inv_view: inv_view.into(),
        cam_pos_and_scale: [vp.camera_pos.x, vp.camera_pos.y, vp.camera_pos.z, 80.0],
        resolution: vp.resolution.into(),
    }
}

impl System {
    pub fn new(event_loop: &EventLoop<()>) -> System {
        let instance = {
            let library = VulkanLibrary::new().unwrap();
            let extensions = vulkano_win::required_extensions(&library);

            let mut layers = vec![];
            if library
                .layer_properties()
                .unwrap()
                .into_iter()
                .any(|l| l.name() == "VK_LAYER_KHRONOS_validation")
            {
                layers.push("VK_LAYER_KHRONOS_validation".to_string());
            } else {
                println!("NO VALIDATION!")
            }

            Instance::new(
                library,
                InstanceCreateInfo {
                    enabled_extensions: extensions,
                    enumerate_portability: true, // required for MoltenVK on macOS
                    max_api_version: Some(Version::V1_1),
                    enabled_layers: layers,
                    ..Default::default()
                },
            )
            .unwrap()
        };

        unsafe {
            let mut severity = DebugUtilsMessageSeverity::empty();
            severity.error = true;
            severity.verbose = true;
            severity.warning = true;
            severity.information = true;
            let mut debug_type = DebugUtilsMessageType::empty();
            debug_type.validation = true;
            debug_type.performance = true;
            debug_type.general = true;

            let _debug_messenger = DebugUtilsMessenger::new(
                instance.clone(),
                DebugUtilsMessengerCreateInfo {
                    message_severity: severity,
                    message_type: debug_type,
                    ..DebugUtilsMessengerCreateInfo::user_callback(Arc::new(|msg| {
                        println!("[VULKAN {:?}] {}", msg.severity, msg.description);
                    }))
                },
            )
            .ok();
        }

        let surface = WindowBuilder::new()
            .build_vk_surface(event_loop, instance.clone())
            .unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ext_full_screen_exclusive: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        // pick first queue_familiy_index that handles graphics and can draw on the surface created by winit
                        q.queue_flags.graphics
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| {
                // lower score for preferred device types
                match p.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                }
            })
            .expect("No suitable physical device found");

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .unwrap();

        let queue = queues.next().unwrap();

        let mut vp = VP::new();

        let (swapchain, images) = {
            let caps = device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();

            let usage = caps.supported_usage_flags;
            let alpha = caps.supported_composite_alpha.iter().next().unwrap();

            let image_format = Some(
                device
                    .physical_device()
                    .surface_formats(&surface, Default::default())
                    .unwrap()[0]
                    .0,
            );

            let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
            let image_extent: [u32; 2] = window.inner_size().into();

            let aspect_ratio = image_extent[0] as f32 / image_extent[1] as f32;
            vp.projection = perspective(aspect_ratio, half_pi(), 0.01, 100.0);

            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: caps.min_image_count,
                    image_format,
                    image_extent,
                    image_usage: usage,
                    composite_alpha: alpha,
                    present_mode: PresentMode::Immediate,
                    full_screen_exclusive: swapchain::FullScreenExclusive::Disallowed,
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(device.clone(), Default::default());

        let deferred_vert = voxel_vert::load(device.clone()).unwrap();
        let deferred_frag = voxel_frag::load(device.clone()).unwrap();

        let render_pass = vulkano::ordered_passes_renderpass!(device.clone(),
            attachments: {
                final_color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16_UNORM,
                    samples: 1,
                }
            },
            passes: [
                {
                    color: [final_color],
                    depth_stencil: {depth},
                    input: []
                }
            ]
        )
        .unwrap();

        let voxel_pass = Subpass::from(render_pass.clone(), 0).unwrap();

        let voxel_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<DummyVertex>())
            .vertex_shader(deferred_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(deferred_frag.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::disabled())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::None))
            .render_pass(voxel_pass.clone())
            .build(device.clone())
            .unwrap();

        let vp_buffer = CpuAccessibleBuffer::from_data(
            &memory_allocator,
            BufferUsage {
                uniform_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            get_camera_ubo(&vp),
        )
        .unwrap();

        let dummy_verts = CpuAccessibleBuffer::from_iter(
            &memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            DummyVertex::list().iter().cloned(),
        )
        .unwrap();

        let vp_layout = voxel_pipeline.layout().set_layouts().get(0).unwrap();
        let vp_set = PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            vp_layout.clone(),
            [WriteDescriptorSet::buffer(1, vp_buffer.clone())],
        )
        .unwrap();

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let framebuffers = System::window_size_dependent_setup(
            &memory_allocator,
            &images,
            render_pass.clone(),
            &mut viewport,
        );

        let render_stage = RenderStage::Stopped;

        let commands = None;
        let image_index = 0;
        let acquire_future = None;
        let voxel_image = None;
        let voxel_image_view = None;

        System {
            surface,
            device,
            queue,
            vp,
            swapchain,
            memory_allocator,
            descriptor_set_allocator,
            command_buffer_allocator,
            render_pass,
            voxel_pipeline,
            vp_buffer,
            dummy_verts,
            vp_set,
            viewport,
            framebuffers,
            render_stage,
            commands,
            image_index,
            acquire_future,
            voxel_image,
            voxel_image_view,
        }
    }

    /*
    pub fn create_voxel_image(
        &self,
        voxel_world: &VoxelWorld,
    ) -> (Arc<StorageImage>, Arc<ImageView<StorageImage>>) {
        let (_min_pos, _max_pos, world_size) = voxel_world.get_world_bounds();

        let image = StorageImage::new(
            &self.memory_allocator,
            ImageDimensions::Dim3d {
                width: world_size[0],
                height: world_size[1],
                depth: world_size[2],
            },
            Format::R8_UINT,
            Some(self.queue.queue_family_index()),
        )
        .unwrap();

        let image_view = ImageView::new_default(image.clone()).unwrap();
        (image, image_view)
    }

    pub fn upload_voxel_data(&self, voxel_world: &VoxelWorld, voxel_image: &Arc<StorageImage>) {
        let staging_buffer = voxel_world.create_staging_buffer(&self.memory_allocator);

        let mut builder = AutoCommandBufferBuilder::primary(
            &StandardCommandBufferAllocator::new(self.device.clone(), Default::default()),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                staging_buffer.clone(),
                voxel_image.clone(),
            ))
            .unwrap();

        let finished = builder
            .build()
            .unwrap()
            .execute(self.queue.clone())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();

        finished.wait(None).unwrap();
    }

    pub fn set_voxel_image(
        &mut self,
        image: Arc<StorageImage>,
        image_view: Arc<ImageView<StorageImage>>,
    ) {
        self.voxel_image = Some(image);
        self.voxel_image_view = Some(image_view);
    }

    pub fn update_voxel_image_dynamic(
        &self,
        voxel_world: &VoxelWorld,
        staging_buffer: &Arc<CpuAccessibleBuffer<[u8]>>,
        voxel_image: &Arc<StorageImage>,
    ) {
        let mut builder = AutoCommandBufferBuilder::primary(
            &StandardCommandBufferAllocator::new(self.device.clone(), Default::default()),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                staging_buffer.clone(),
                voxel_image.clone(),
            ))
            .unwrap();

        let command_buffer = builder.build().unwrap();

        let finished = command_buffer
            .execute(self.queue.clone())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();

        finished.wait(None).unwrap();
    }*/

    pub fn create_voxel_image(&mut self, size: [u32; 3]) {
        use vulkano::format::Format;
        use vulkano::image::ImageDimensions;

        let image = StorageImage::new(
            &self.memory_allocator,
            ImageDimensions::Dim3d {
                width: size[0].max(1),
                height: size[1].max(1),
                depth: size[2].max(1),
            },
            Format::R8_UINT,
            Some(self.queue.queue_family_index()),
        )
        .unwrap();

        let image_view = ImageView::new_default(image.clone()).unwrap();
        self.voxel_image = Some(image);
        self.voxel_image_view = Some(image_view);
    }

    pub fn update_voxel_image(&mut self, voxel_world: &mut VoxelWorld) {
        if !voxel_world.needs_gpu_update {
            return;
        }

        if let Some(image) = &self.voxel_image {
            let staging_buffer = voxel_world.create_staging_buffer(&self.memory_allocator);

            let mut builder = AutoCommandBufferBuilder::primary(
                &self.command_buffer_allocator,
                self.queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();

            builder
                .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                    staging_buffer,
                    image.clone(),
                ))
                .unwrap();

            let command_buffer = builder.build().unwrap();
            let future = vulkano::sync::now(self.queue.device().clone())
                .then_execute(self.queue.clone(), command_buffer)
                .unwrap()
                .then_signal_fence_and_flush()
                .unwrap();
            future.wait(None).unwrap();

            voxel_world.needs_gpu_update = false;
        }
    }

    pub fn voxel(&mut self) {
        match self.render_stage {
            RenderStage::Voxel => {}
            RenderStage::NeedsRedraw => {
                self.recreate_swapchain();
                self.render_stage = RenderStage::Stopped;
                self.commands = None;
                return;
            }
            _ => {
                self.render_stage = RenderStage::Stopped;
                self.commands = None;
                return;
            }
        }

        let voxel_buffer = self
            .voxel_image_view
            .as_ref()
            .expect("Voxel image not set!");

        let sampler = Sampler::new(
            self.device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                ..Default::default()
            },
        )
        .unwrap();

        let voxel_layout = self.voxel_pipeline.layout().set_layouts().get(0).unwrap();
        let voxel_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            voxel_layout.clone(),
            [
                WriteDescriptorSet::image_view_sampler(0, voxel_buffer.clone(), sampler.clone()),
                WriteDescriptorSet::buffer(1, self.vp_buffer.clone()),
            ],
        )
        .unwrap();

        self.commands
            .as_mut()
            .unwrap()
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.voxel_pipeline.clone()) // your voxel raymarch pipeline
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.voxel_pipeline.layout().clone(),
                0,
                voxel_set.clone(),
            )
            .bind_vertex_buffers(0, self.dummy_verts.clone()) // fullscreen triangle
            .draw(self.dummy_verts.len() as u32, 1, 0, 0)
            .unwrap();
    }

    pub fn finish(&mut self, previous_frame_end: &mut Option<Box<dyn GpuFuture>>) {
        match self.render_stage {
            RenderStage::Voxel => {}
            RenderStage::NeedsRedraw => {
                self.recreate_swapchain();
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
            _ => {
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
        }

        let mut commands = self.commands.take().unwrap();
        commands.end_render_pass().unwrap();
        let command_buffer = commands.build().unwrap();

        let af = self.acquire_future.take().unwrap();

        let mut local_future: Option<Box<dyn GpuFuture>> =
            Some(Box::new(sync::now(self.device.clone())) as Box<dyn GpuFuture>);

        mem::swap(&mut local_future, previous_frame_end);

        let future = local_future
            .take()
            .unwrap()
            .join(af)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.clone(),
                    self.image_index,
                ),
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                *previous_frame_end = Some(Box::new(future) as Box<_>);
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain();
                *previous_frame_end = Some(Box::new(sync::now(self.device.clone())) as Box<_>);
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                *previous_frame_end = Some(Box::new(sync::now(self.device.clone())) as Box<_>);
            }
        }

        self.commands = None;
        self.render_stage = RenderStage::Stopped;
    }

    pub fn set_view(&mut self, view: &TMat4<f32>) {
        self.vp.view = view.clone();
        let look = inverse(&view);
        self.vp.camera_pos = vec3(look[12], look[13], look[14]);
        self.vp_buffer = CpuAccessibleBuffer::from_data(
            &self.memory_allocator,
            BufferUsage {
                uniform_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            get_camera_ubo(&self.vp),
        )
        .unwrap();

        let vp_layout = self.voxel_pipeline.layout().set_layouts().get(0).unwrap();
        self.vp_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            vp_layout.clone(),
            [WriteDescriptorSet::buffer(1, self.vp_buffer.clone())],
        )
        .unwrap();

        self.render_stage = RenderStage::Stopped;
    }

    pub fn start(&mut self) {
        match self.render_stage {
            RenderStage::Stopped => {
                self.render_stage = RenderStage::Voxel;
            }
            RenderStage::NeedsRedraw => {
                self.recreate_swapchain();
                self.render_stage = RenderStage::Stopped;
                self.commands = None;
                return;
            }
            _ => {
                self.render_stage = RenderStage::Stopped;
                self.commands = None;
                return;
            }
        }

        let (image_index, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain();
                    return;
                }
                Err(err) => panic!("{:?}", err),
            };

        if suboptimal {
            self.recreate_swapchain();
            return;
        }

        let clear_values = vec![Some([0.0, 0.0, 0.0, 1.0].into()), Some(1.0.into())];

        let mut commands = AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        commands
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values,
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_index as usize].clone(),
                    )
                },
                SubpassContents::Inline,
            )
            .unwrap();

        self.commands = Some(commands);
        self.image_index = image_index;
        self.acquire_future = Some(acquire_future);
    }

    pub fn recreate_swapchain(&mut self) {
        self.render_stage = RenderStage::NeedsRedraw;
        self.commands = None;

        let window = self
            .surface
            .object()
            .unwrap()
            .downcast_ref::<Window>()
            .unwrap();
        let image_extent: [u32; 2] = window.inner_size().into();

        let aspect_ratio = image_extent[0] as f32 / image_extent[1] as f32;
        self.vp.projection = perspective(aspect_ratio, half_pi(), 0.01, 300.0);
        self.vp.resolution = vec2(image_extent[0] as f32, image_extent[1] as f32);

        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent,
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        let new_framebuffers = System::window_size_dependent_setup(
            &self.memory_allocator,
            &new_images,
            self.render_pass.clone(),
            &mut self.viewport,
        );

        self.swapchain = new_swapchain;
        self.framebuffers = new_framebuffers;

        self.vp_buffer = CpuAccessibleBuffer::from_data(
            &self.memory_allocator,
            BufferUsage {
                uniform_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            get_camera_ubo(&self.vp),
        )
        .unwrap();

        let vp_layout = self.voxel_pipeline.layout().set_layouts().get(0).unwrap();
        self.vp_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            vp_layout.clone(),
            [WriteDescriptorSet::buffer(1, self.vp_buffer.clone())],
        )
        .unwrap();

        self.render_stage = RenderStage::Stopped;
    }

    fn window_size_dependent_setup(
        allocator: &StandardMemoryAllocator,
        images: &[Arc<SwapchainImage>],
        render_pass: Arc<RenderPass>,
        viewport: &mut Viewport,
    ) -> Vec<Arc<Framebuffer>> {
        let dimensions = images[0].dimensions().width_height();
        viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

        let depth_buffer = ImageView::new_default(
            AttachmentImage::transient(allocator, dimensions, Format::D16_UNORM).unwrap(),
        )
        .unwrap();

        let framebuffers = images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view, depth_buffer.clone()],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();

        framebuffers
    }
}
