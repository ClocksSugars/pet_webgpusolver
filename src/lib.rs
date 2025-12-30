mod mathutils;
mod squaregrid;
mod aspng;
mod webgpuheat;

use crate::aspng::*;
use crate::mathutils::*;
use crate::squaregrid::*;
use crate::webgpuheat::*;

use std::fmt::Display;
use std::sync::Arc;
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


pub struct State {
   surface: wgpu::Surface<'static>,
   device: wgpu::Device,
   queue: wgpu::Queue,
   config: wgpu::SurfaceConfiguration,
   is_surface_configured: bool,
   render_pipeline: wgpu::RenderPipeline,
   vertex_buffer: wgpu::Buffer,
   index_buffer: wgpu::Buffer,
   texture_draw_bind_group: wgpu::BindGroup,
   texture_buffer: wgpu::Texture,
   texture_view: wgpu::TextureView,
   texture_sampler: wgpu::Sampler,
   heateq: HeatComputer,
   window: Arc<Window>,
}

impl State {
   pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
      let size = window.inner_size();

      let instance = wgpu::Instance::default();
      // ::new(&wgpu::InstanceDescriptor {
      //     #[cfg(not(target_arch = "wasm32"))]
      //     backends: wgpu::Backends::PRIMARY,
      //     #[cfg(target_arch = "wasm32")]
      //     backends: wgpu::Backends::GL,
      //     ..Default::default()
      // });

      gen_print("instance okay");

      let surface = instance.create_surface(window.clone()).unwrap();

      gen_print("surface okay");

      let adapter = instance
         .request_adapter(&wgpu::RequestAdapterOptions//::default()).await.ok()?;
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

      {
         println!("VULKAN backends: {:?}",instance.enumerate_adapters(wgpu::Backends::VULKAN).await);
         println!("DX12 backends: {:?}",instance.enumerate_adapters(wgpu::Backends::DX12).await);
         println!("GL backends: {:?}",instance.enumerate_adapters(wgpu::Backends::GL).await);
         println!("BROWSER_WEBGPU backends: {:?}",instance.enumerate_adapters(wgpu::Backends::BROWSER_WEBGPU).await);
         let adcap = adapter.get_downlevel_capabilities();
         //println!("{:?}",adapter.features());
         gen_print(adcap.is_webgpu_compliant());
      }

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
         // Canvases can stretch!
         width: size.width,
         height: size.height,
         present_mode: surface_caps.present_modes[0],
         alpha_mode: surface_caps.alpha_modes[0],
         desired_maximum_frame_latency: 2,
         view_formats: vec![],
      };


      let size = wgpu::Extent3d {
         width: 256,
         height: 256,
         depth_or_array_layers: 1,
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

      let default_texture: Vec<u8> = [0,255,0,255].repeat((size.width * size.height) as usize);

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
              bytes_per_row: Some(4 * size.width),
              rows_per_image: None,
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
         mipmap_filter: wgpu::MipmapFilterMode::Nearest,
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
            immediate_size: 0,
         });

      let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
         label: Some("Render Pipeline"),
         multiview_mask: None,
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

      let length = 256;
      let heateq = HeatComputer::new(
         SquareGrid::newbyfunc(
            length as usize,
            makemiddleRatTinitconds(0.2, 400.)
         ).getarray(),
         length,
         &device
      );

      gen_print("heat compute okay");

      let delta_t: f32 = 1. / (8. * 1. * length.pow(2) as f32); // safety factor 0.5
      heateq.update_values(&queue, 1., delta_t, 0., 400.);

      let mut encoder = device.create_command_encoder(&Default::default());
      heateq.unsafe_queue_color_job(&mut encoder);
      heateq.unsafe_color_to_texture_queue(&mut encoder,&texture);
      queue.submit([encoder.finish()]);

      //gen_print("heat map job sent");

      #[cfg(not(target_arch = "wasm32"))]
      heateq.export_heatmap_buffer(&device, &queue).await;

      Ok(Self {
          surface: surface,
          device: device,
          queue: queue,
          config: config,
          is_surface_configured: false,
          render_pipeline: render_pipeline,
          vertex_buffer: vertex_buffer,
          index_buffer: index_buffer,
          texture_draw_bind_group: texture_draw_bind_group,
          texture_buffer: texture,
          texture_view: view,
          texture_sampler: sampler,
          heateq: heateq,
          window: window,
      })
   }

   pub fn resize(&mut self, width: u32, height: u32) {
       if width > 0 && height > 0 {
           self.config.width = width;//256;
           self.config.height = height;//256;
           self.surface.configure(&self.device, &self.config);
           self.is_surface_configured = true;
       }
   }

   fn handle_key(&mut self, event_loop: &ActiveEventLoop, key: KeyCode, pressed: bool) {
       match (key, pressed) {
           (KeyCode::Escape, true) => event_loop.exit(),
           _ => {}
       }
   }

   fn update(&mut self) {}

   pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
      self.window.request_redraw();

      // We can't render unless the surface is configured
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
              multiview_mask: None,
              color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                  view: &view,
                  resolve_target: None,
                  ops: wgpu::Operations {
                      load: wgpu::LoadOp::Clear(wgpu::Color {
                          r: 0.1,
                          g: 0.2,
                          b: 0.3,
                          a: 1.0,
                      }),
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

      self.queue.submit([encoder.finish()]);
      output.present();

      Ok(())
   }
}





pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl App {
   pub fn new(
      #[cfg(target_arch = "wasm32")]
      event_loop: &EventLoop<State>)
   -> Self {

      #[cfg(target_arch = "wasm32")]
      let proxy = Some(event_loop.create_proxy());

      Self {
         state: None,

         #[cfg(target_arch = "wasm32")]
         proxy: proxy,
      }
   }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
      #[allow(unused_mut)]
      let mut window_attributes = Window::default_attributes();



      #[cfg(target_arch = "wasm32")]
      {
         log::info!("we arrive at app state builder");

         const CANVAS_ID: &str = "canvas";

         let window = wgpu::web_sys::window().unwrap_throw();
         let document = window.document().unwrap_throw();
         let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
         let html_canvas_element = canvas.unchecked_into();
         window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
      }

      let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

      #[cfg(not(target_arch = "wasm32"))]
      {
         let temprt = tokio::runtime::Runtime::new()
           .expect("tokio runtime creation failed");
         self.state = temprt.block_on(State::new(window)).ok();
         //self.state = Some(pollster::block_on(State::new(window)).unwrap());
         gen_print("state creation finished");
      }

      #[cfg(target_arch = "wasm32")]
      {
         // Run the future asynchronously and use the
         // proxy to send the results to the event loop
         if let Some(proxy) = self.proxy.take() {
            wasm_bindgen_futures::spawn_local(async move {
               assert!(proxy
                  .send_event(
                     State::new(window)
                        .await
                        .expect("Unable to create canvas!!!")
                  )
               .is_ok())
            });
         }
      }
   }

   #[allow(unused_mut)]
   fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
       #[cfg(target_arch = "wasm32")]
       {
           event.window.request_redraw();
           event.resize(
               event.window.inner_size().width,
               event.window.inner_size().height,
           );
       }
       self.state = Some(event);
   }

   fn window_event(
       &mut self,
       event_loop: &ActiveEventLoop,
       _window_id: winit::window::WindowId,
       event: WindowEvent,
   ) {
       let state = match &mut self.state {
           Some(canvas) => canvas,
           None => return,
       };

       match event {
           WindowEvent::CloseRequested => event_loop.exit(),
           WindowEvent::Resized(size) => state.resize(size.width, size.height),
           WindowEvent::RedrawRequested => {
               state.update();
               match state.render() {
                   Ok(_) => {}
                   // Reconfigure the surface if it's lost or outdated
                   Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                       let size = state.window.inner_size();
                       state.resize(size.width, size.height);
                   }
                   Err(e) => {
                       log::error!("Unable to render {}", e);
                   }
               }
           }
           WindowEvent::MouseInput { state, button, .. } => match (button, state.is_pressed()) {
               (MouseButton::Left, true) => {}
               (MouseButton::Left, false) => {}
               _ => {}
           },
           WindowEvent::KeyboardInput {
               event:
                   KeyEvent {
                       physical_key: PhysicalKey::Code(code),
                       state: key_state,
                       ..
                   },
               ..
           } => state.handle_key(event_loop, code, key_state.is_pressed()),
           _ => {}
       }
   }
}



pub fn run() -> Option<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
        log::info!{"initialized log"}
        env_logger::init()
    }

    let event_loop = EventLoop::with_user_event().build().ok()?;
    let mut app = App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );
    event_loop.run_app(&mut app).ok()?;

    Some(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}
