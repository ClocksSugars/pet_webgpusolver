use crate::aspng::*;
use crate::mathutils::*;
use crate::rectgrid::*;
use crate::webgpuheat::*;
use crate::wgpuworkhorse;

use std::cell::OnceCell;
use std::fmt::Display;
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;
use winit::window;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window
};
use wgpu::util::{DeviceExt};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use console_log::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowAttributesExtWebSys;

fn gen_print<T>(s: T) where T: Display {
   #[cfg(target_arch = "wasm32")]
   log::info!("{}",s);
   #[cfg(not(target_arch = "wasm32"))]
   println!("{}",s)
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, -1.0, 0.0],
        tex_coords: [0.0, 1.0],
    }, // 0 bottom left
    Vertex {
        position: [1.0, -1.0, 0.0],
        tex_coords: [1.0, 1.0],
    }, // 1 bottom right
    Vertex {
        position: [1.0, 1.0, 0.0],
        tex_coords: [1.0, 0.0],
    }, // 2 top right
    Vertex {
        position: [-1.0, 1.0, 0.0],
        tex_coords: [0.0, 0.0],
    }, // 3 top left
];

// our rectangle is two triangles bisecting the screen on its anti-diagonal
const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3, ];// /* padding */ 0,];

pub struct WgpuState {
   pub surface: wgpu::Surface<'static>,
   pub device: wgpu::Device,
   pub queue: wgpu::Queue,
   pub config: wgpu::SurfaceConfiguration,
   #[cfg(not(target_arch =  "wasm32"))]
   pub is_surface_configured: bool,
   pub render_pipeline: wgpu::RenderPipeline,
   pub vertex_buffer: wgpu::Buffer,
   pub index_buffer: wgpu::Buffer,
   pub texture_draw_bind_group: wgpu::BindGroup,
   pub texture_buffer: wgpu::Texture,
   pub texture_view: wgpu::TextureView,
   pub texture_sampler: wgpu::Sampler,
   pub heateq: HeatComputer,
   pub pending_queue: std::cell::Cell<Vec<wgpu::CommandBuffer>>
}

impl WgpuState {

   pub async fn new(
      #[cfg(target_arch = "wasm32")]
      valid_pre_surface: web_sys::HtmlCanvasElement,
      #[cfg(not(target_arch = "wasm32"))]
      valid_pre_surface: Arc<Window>,
   ) -> anyhow::Result<Self> {

      Self::new_with(valid_pre_surface, 256, 256).await
   }

   pub async fn new_with(
      #[cfg(target_arch = "wasm32")]
      valid_pre_surface: web_sys::HtmlCanvasElement,
      #[cfg(not(target_arch = "wasm32"))]
      valid_pre_surface: Arc<Window>,
      width: u32,
      height: u32,
   ) -> anyhow::Result<Self>
   {
      let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
          #[cfg(not(target_arch = "wasm32"))]
          backends: wgpu::Backends::PRIMARY,
          #[cfg(target_arch = "wasm32")]
          backends: wgpu::Backends::BROWSER_WEBGPU,
          ..Default::default()
      });

      #[cfg(target_arch = "wasm32")]
      let surface: wgpu::Surface = {
         let surface_target = wgpu::SurfaceTarget::Canvas(valid_pre_surface);
         instance.create_surface(surface_target).expect("failed to create surface")
      };
      #[cfg(not(target_arch = "wasm32"))]
      let surface: wgpu::Surface = {
         instance.create_surface(valid_pre_surface.clone()).unwrap()
      };

      gen_print("surface okay");

      let size = wgpu::Extent3d {
         width: width,
         height: height,
         depth_or_array_layers: 1,
      };


      let adapter = instance
         .request_adapter(&wgpu::RequestAdapterOptions
      {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
         })
         .await?;

      gen_print("adapater okay");

      let (device, queue) = adapter
         .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            // WebGL doesn't support all of wgpu's features, so if
            // we're building for the web we'll have to disable some.
            required_limits: if cfg!(target_arch = "wasm32") {
               adapter.limits()
               //wgpu::Limits::downlevel_defaults()
               //wgpu::Limits::downlevel_webgl2_defaults()
            } else {
               wgpu::Limits::defaults()
            },
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
         })
         .await?;

      gen_print("device okay");

      fn error_capture(error: wgpu::Error) {
         gen_print(error.to_string());
      }
      device.on_uncaptured_error(Arc::new(error_capture));

      let surface_caps = surface.get_capabilities(&adapter);
      println!("surface capabilities: {:?}", surface_caps);
      let surface_format = surface_caps
         .formats
         .iter()
         .copied()
         .find(|f| f.is_srgb())
         .unwrap_or(surface_caps.formats[0]);

      let config = wgpu::SurfaceConfiguration {
         usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
         format: surface_format,
         width: size.width,
         height: size.height,
         present_mode: surface_caps.present_modes[0],
         alpha_mode: surface_caps.alpha_modes[0],
         desired_maximum_frame_latency: 2,
         view_formats: vec![],
      };

      let texture = device.create_texture(&wgpu::TextureDescriptor {
         label: Some("default texture"),
         size,
         mip_level_count: 1,
         sample_count: 1,
         dimension: wgpu::TextureDimension::D2,
         format: wgpu::TextureFormat::Rgba8UnormSrgb,
         usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
         view_formats: &[],
      });

      let default_texture: Vec<u8> = [0,255,0,255].repeat((size.width.div_ceil(64) * size.height * 64) as usize);

      queue.write_texture(
          wgpu::TexelCopyTextureInfo {
              aspect: wgpu::TextureAspect::All,
              texture: &texture,
              mip_level: 0,
              origin: wgpu::Origin3d::ZERO,
          },
          &default_texture,
          wgpu::TexelCopyBufferLayout {
              offset: 0,
              bytes_per_row: Some(256 * size.width.div_ceil(64)),
              rows_per_image: Some(size.height),
          },
          size,
      );

      let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
      let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
         address_mode_u: wgpu::AddressMode::ClampToEdge,
         address_mode_v: wgpu::AddressMode::ClampToEdge,
         address_mode_w: wgpu::AddressMode::ClampToEdge,
         mag_filter: wgpu::FilterMode::Linear,
         min_filter: wgpu::FilterMode::Nearest,
         mipmap_filter: wgpu::FilterMode::Nearest,//wgpu::MipmapFilterMode::Nearest,
         ..Default::default()
      });

      let texture_bind_group_layout =
         device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
               wgpu::BindGroupLayoutEntry {
                     binding: 0,
                     visibility: wgpu::ShaderStages::FRAGMENT,
                     ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
            label: Some("texture_bind_group_layout"),
         });

      let texture_draw_bind_group =
         device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
               wgpu::BindGroupEntry {
                  binding: 0,
                  resource: wgpu::BindingResource::TextureView(&view),
               },
               wgpu::BindGroupEntry {
                  binding: 1,
                  resource: wgpu::BindingResource::Sampler(&sampler),
               },
            ],
            label: Some("diffuse_bind_group"),
         });

      let shader = device.create_shader_module(
         wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(
               include_str!("rectangle.wgsl").into()),
         }
      );

      let render_pipeline_layout =
         device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            //immediate_size: 0,
            push_constant_ranges: &[]
         });

      let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
         label: Some("Render Pipeline"),
         //multiview_mask: None,
         multiview: None,
         layout: Some(&render_pipeline_layout),
         vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Vertex::desc()],
            compilation_options: Default::default(),
         },
         fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
               format: config.format,
               blend: Some(wgpu::BlendState {
                     color: wgpu::BlendComponent::REPLACE,
                     alpha: wgpu::BlendComponent::REPLACE,
               }),
               write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
         }),
         primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
            // or Features::POLYGON_MODE_POINT
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
         },
         depth_stencil: None,
         multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
         },
         // If the pipeline will be used with a multiview render pass, this
         // indicates how many array layers the attachments will have.
         // Useful for optimizing shader compilation on Android
         cache: None,
      });

      let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
         label: Some("Vertex Buffer"),
         contents: bytemuck::cast_slice(VERTICES),
         usage: wgpu::BufferUsages::VERTEX,
      });
      let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
         label: Some("Index Buffer"),
         contents: bytemuck::cast_slice(INDICES),
         usage: wgpu::BufferUsages::INDEX,
      });
      // let num_indices = INDICES.len() as u32;

      gen_print("buffers and shaders okay");

      let mut heateq = HeatComputer::new(
         RectGrid::newbyfunc(
            width as usize,
            height as usize,
            makemiddleRatTinitconds(0.2, 400.)
         ).getarray(),
         width,
         height,
         &device
      );

      gen_print("heat compute okay");

      let delta_t: f32 = 1. / (4. * (width.pow(2) + height.pow(2)) as f32); // safety factor 0.5
      heateq.update_values(&queue, 100, 1., delta_t, 0., 400.);

      let mut encoder = device.create_command_encoder(&Default::default());
      heateq.unsafe_queue_color_job(&mut encoder);
      heateq.unsafe_color_to_texture_queue(&mut encoder,&texture);
      queue.submit([encoder.finish()]);

      //gen_print("heat map job sent");

      // #[cfg(not(target_arch = "wasm32"))]
      // heateq.export_heatmap_buffer(&device, &queue).await;

      #[cfg(target_arch = "wasm32")]
      surface.configure(&device, &config);

      Ok(Self {
          surface: surface,
          device: device,
          queue: queue,
          config: config,
          #[cfg(not(target_arch =  "wasm32"))]
          is_surface_configured: false,
          render_pipeline: render_pipeline,
          vertex_buffer: vertex_buffer,
          index_buffer: index_buffer,
          texture_draw_bind_group: texture_draw_bind_group,
          texture_buffer: texture,
          texture_view: view,
          texture_sampler: sampler,
          heateq: heateq,
          pending_queue: std::cell::Cell::new(Vec::new())
      })
   }

   pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {

      // We can't render unless the surface is configured

      #[cfg(not(target_arch =  "wasm32"))]
      if !self.is_surface_configured {
         return Ok(());
      }

      let output = self.surface.get_current_texture()?;
      let view = output
         .texture
         .create_view(&wgpu::TextureViewDescriptor::default());

      let mut encoder = self
         .device
         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
          });

      {
          let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
              label: Some("Render Pass"),
              //multiview_mask: None,
              color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                  view: &view,
                  resolve_target: None,
                  ops: wgpu::Operations {
                      load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                      store: wgpu::StoreOp::Store,
                  },
                  depth_slice: None,
              })],
              depth_stencil_attachment: None,
              occlusion_query_set: None,
              timestamp_writes: None,
          });

          render_pass.set_pipeline(&self.render_pipeline);
          render_pass.set_bind_group(0, &self.texture_draw_bind_group, &[]);
          render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
          render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
          render_pass.draw_indexed(0..6, 0, 0..1);
      };

      let mut pending_queue =  self.pending_queue.replace(Vec::new());
      pending_queue.push(encoder.finish());
      self.queue.submit(pending_queue.into_boxed_slice());

      output.present();

      Ok(())
   }
}
