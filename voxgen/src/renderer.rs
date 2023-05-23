pub mod atlas;
pub mod buffer;
pub mod debug;
pub mod mesh;
pub mod pipelines;
pub mod texture;
pub mod ui;
pub mod world;

use bevy_ecs::world::World;
use vek::Vec3;
pub use world::WorldRenderer;

use std::time::Duration;

use crate::{
    scene::{camera::{Camera, CameraUniform}, Scene},
    ui::EguiInstance,
};

use self::{buffer::Buffer, debug::DebugRenderer, texture::Texture, ui::UIRenderer};

trait Renderable {
    fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        global_uniforms: &'a wgpu::BindGroup,
    );
}

pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    world_renderer: WorldRenderer,
    debug_renderer: DebugRenderer,
    depth: Texture,
    camera_uniform: CameraUniform,
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: Buffer<CameraUniform>,
    egui_render_pass: egui_wgpu_backend::RenderPass,
    pub gui: EguiInstance,
}

impl Renderer {
    pub async fn new(winit_impl: &winit::window::Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });
        let surface = unsafe { instance.create_surface(&winit_impl) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::POLYGON_MODE_LINE,
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();
        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let size = winit_impl.inner_size();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let transform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
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
        let depth = Texture::with_depth(&config, &device);

        let camera_uniform = CameraUniform::empty();

        let transform_buffer = Buffer::new(
            &device,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            &[camera_uniform],
        );

        let transform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &transform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: transform_buffer.buf.as_entire_binding(),
            }],
        });
        let world_renderer =
            WorldRenderer::new(&device, &queue, &config, &transform_bind_group_layout);
        let debug_renderer = DebugRenderer::new(&device, &config, &transform_bind_group_layout);
        let egui_render_pass = egui_wgpu_backend::RenderPass::new(&device, surface_format, 1);
        let gui = EguiInstance::new(&winit_impl);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            world_renderer,
            depth,
            camera_bind_group: transform_bind_group,
            camera_buffer: transform_buffer,
            camera_uniform,
            egui_render_pass,
            gui,
            debug_renderer,
        }
    }

    pub fn toggle_wireframe(&mut self) {
        self.world_renderer.wireframe = !self.world_renderer.wireframe;
    }

    pub fn resize(&mut self, scene: &mut Scene,  new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth = Texture::with_depth(&self.config, &self.device);
            scene.resize(new_size.width as f32, new_size.height as f32);
        }
    }

    pub fn input(&mut self, _: &winit::event::Event<()>) {
    }

    pub fn update(&mut self, scene: &mut Scene, dt: Duration) {
        scene.tick(&self.queue, dt);
        self.camera_uniform.update(&scene.camera);
        self.camera_buffer.update(&self.queue, &[self.camera_uniform], 0);
        self.world_renderer.tick(Vec3::zero(), &self.device);
    }

    pub fn render(&mut self, scale_factor: f32, dt: f32) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render enconder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.3,
                            b: 0.6,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            self.world_renderer
                .render(&mut render_pass, &self.camera_bind_group);
            self.debug_renderer
                .render(&mut render_pass, &self.camera_bind_group);
        }
        let mut ui_renderer = UIRenderer::new(&mut encoder, self, dt, Vec3::zero());
        ui_renderer.draw_egui(&surface_texture, scale_factor);

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        Ok(())
    }
}
