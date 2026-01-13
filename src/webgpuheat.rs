use wgpu::util::{BufferInitDescriptor, DeviceExt};
use bytemuck::cast_slice;


#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::oneshot::{Receiver, channel};

#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};

fn gen_print<T>(s: T) where T: core::fmt::Display {
   #[cfg(target_arch = "wasm32")]
   log::info!("{}",s);
   #[cfg(not(target_arch = "wasm32"))]
   println!("{}",s)
}

#[derive(Debug)]
pub enum ComputeRelevantEvent {
   ComputeDoneNowColor,
   ComputeIsWorking,
   ColorIsDone,
   ColorIsWorking,
   ColorIsCopying,
   ColorCopyDone
}


#[allow(non_snake_case)]
pub struct HeatComputer {
   pub iteration_quantity: u32,
   pub width: u32,
   pub height: u32,
   pub pad_per_line: u32,
   pub workgroup_size: u32,
   pub fix_boundary_conditions_shdr: wgpu::ShaderModule,
   pub laplacian_shader: wgpu::ShaderModule,
   pub iterate_shader: wgpu::ShaderModule,
   pub buffer_move_shader: wgpu::ShaderModule,
   pub fix_boundary_conditions_ppln: wgpu::ComputePipeline,
   pub laplacian_pipeline: wgpu::ComputePipeline,
   pub iterate_pipeline: wgpu::ComputePipeline,
   pub buffer_move_pipeline: wgpu::ComputePipeline,
   pub data_buffer: wgpu::Buffer,
   pub laplacian_buffer: wgpu::Buffer,
   pub midpoint_buffer: wgpu::Buffer,
   pub midpoint_laplacian_buffer: wgpu::Buffer,
   pub output_buffer: wgpu::Buffer,
   pub export_buffer: wgpu::Buffer,
   pub width_buffer: wgpu::Buffer,
   pub height_buffer: wgpu::Buffer,
   pub kappa_buffer: wgpu::Buffer,
   pub delta_t_buffer: wgpu::Buffer,
   pub delta_t_2_buffer: wgpu::Buffer,
   pub pad_buffer: wgpu::Buffer,

   // When i have more confidence, these should be an vec of 'steps'
   //    although that may require the above pipelines to be changed to
   //    Arc<wgpu::Buffer> types.
   pub fix_boundary_conditions_bg: wgpu::BindGroup,
   pub stage_one_bind_group: wgpu::BindGroup,
   pub stage_two_bind_group: wgpu::BindGroup,
   pub stage_three_bind_group: wgpu::BindGroup,
   pub stage_four_bind_group: wgpu::BindGroup,
   pub stage_five_bind_group: wgpu::BindGroup,

   pub vis_minT_buffer: wgpu::Buffer,
   pub vis_maxT_buffer: wgpu::Buffer,
   pub heat_map_buffer: wgpu::Buffer,
   pub heat_hue_shader: wgpu::ShaderModule,
   pub heat_hue_pipeline: wgpu::ComputePipeline,
   pub heat_hue_bind_group: wgpu::BindGroup,

   pub workgroup_quantity: u32,


   // NOTE: my understanding is that wasm will not start threads in the same way rust will.
   //    for this reason it makes sense to use a sender-receiver in rust to avoid thread stuff
   //    and an arc-mutex in wasm which ostensibly just lets us send a message that doesnt anger
   //    the rust-compiler borrow checker.

   #[cfg(not(target_arch = "wasm32"))]
   pub progress: Option<
      (ComputeRelevantEvent, Receiver<ComputeRelevantEvent>)
   >,

   #[cfg(target_arch = "wasm32")]
   pub progress: Arc<
      Mutex<
         Option<ComputeRelevantEvent>
      >
   >
}





fn helper_basic_compute_shader(
   device: &wgpu::Device,
   label: Option<&str>,
   shader_module: &wgpu::ShaderModule
) -> wgpu::ComputePipeline {
   device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
       label: label,
       layout: None,
       module: shader_module,
       entry_point: None,
       compilation_options: Default::default(),
       cache: Default::default(),
   })
}

fn helper_compute_interim_data_buffer(
   device: &wgpu::Device,
   label: Option<&str>,
   size: u64,
) -> wgpu::Buffer {
   device.create_buffer(&wgpu::BufferDescriptor {
      label: label,
      size: size,
      usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
      mapped_at_creation: false,
   })
}

fn helper_param_buffer(
   device: &wgpu::Device,
   label: Option<&str>,
   size: u64,
) -> wgpu::Buffer {
   device.create_buffer(&wgpu::BufferDescriptor {
      label: label,
      size: size,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
   })
}

fn helper_compute_bind_group(
   device: &wgpu::Device,
   label: Option<&str>,
   pipeline: &wgpu::ComputePipeline,
   buffer_sequence: &[&wgpu::Buffer]
) -> wgpu::BindGroup {
   let mut temp: Vec<wgpu::BindGroupEntry> = Vec::new();

   for (n,val) in buffer_sequence.iter().enumerate() {
      temp.push(
         wgpu::BindGroupEntry {
               binding: n as u32,
               resource: val.as_entire_binding()
         },
      )
   }

   device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: label,
      layout: &pipeline.get_bind_group_layout(0),
      entries: &temp
   })
}


impl HeatComputer {
   pub fn new(
      initial_data: &Vec<f32>,
      width: u32,
      height: u32,
      device: &wgpu::Device,
      //queue: &wgpu::Queue,
   ) -> Self {
      assert_eq!(initial_data.len() as u32, width*height);

      let pad_per_line: u32 = width.div_ceil(64) * 64 - width;

      let laplacian_shader = device.create_shader_module(wgpu::include_wgsl!("laplacian.wgsl"));
      let iterate_shader = device.create_shader_module(wgpu::include_wgsl!("iterate_heat.wgsl"));
      let buffer_move_shader = device.create_shader_module(wgpu::include_wgsl!("buffer_move.wgsl"));
      let fix_boundary_conditions_shdr = device.create_shader_module(wgpu::include_wgsl!("boundary_cond.wgsl"));

      let laplacian_pipeline = helper_basic_compute_shader(device, Some("Laplacian Pipeline"), &laplacian_shader);
      let iterate_pipeline = helper_basic_compute_shader(device, Some("Iteration Pipeline"), &iterate_shader);
      let buffer_move_pipeline = helper_basic_compute_shader(device, Some("Relocation Pipeline"), &buffer_move_shader);
      let fix_boundary_conditions_ppln = helper_basic_compute_shader(device, Some("Boundary Conds Pipeline"), &fix_boundary_conditions_shdr);

      let data_buffer = device.create_buffer_init(&BufferInitDescriptor {
         label: Some("data"),
         contents: bytemuck::cast_slice(&initial_data),
         usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
      });

      let export_buffer = device.create_buffer(&wgpu::BufferDescriptor {
         label: Some("export"),
         size: data_buffer.size(),
         usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
         mapped_at_creation: false,
      });

      let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
         label: Some("output"),
         size: data_buffer.size(),
         usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
         mapped_at_creation: false
      });
      let laplacian_buffer = helper_compute_interim_data_buffer(
         device, Some("laplacian buffer"), data_buffer.size()
      );
      let midpoint_buffer = helper_compute_interim_data_buffer(
         device, Some("midpoint buffer"), data_buffer.size()
      );
      let midpoint_laplacian_buffer = helper_compute_interim_data_buffer(
         device, Some("midpoint laplacian buffer"), data_buffer.size()
      );


      let width_buffer = device.create_buffer_init(&BufferInitDescriptor {
          label: Some("width"),
          contents: bytemuck::cast_slice(&[width]),
          usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      });
      let height_buffer = device.create_buffer_init(&BufferInitDescriptor {
          label: Some("height"),
          contents: bytemuck::cast_slice(&[height]),
          usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      });
      let kappa_buffer = helper_param_buffer(device,Some("kappa"),4);
      let delta_t_buffer = helper_param_buffer(device,Some("delta_t"),4);
      // this is just the same number divided by two so we can reuse the pipeline
      let delta_t_2_buffer = helper_param_buffer(device,Some("delta_t_2"),4);
      let pad_buffer = device.create_buffer_init(&BufferInitDescriptor {
          label: Some("pad_per_line"),
          contents: bytemuck::cast_slice(&[pad_per_line]),
          usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      });

      // if pipelines are like gpu function calls, this is where we identify our variables in address space.
      // in that sense we may freely put in different buffers like swapping arguments to a function

      // shader that fixes the insulating boundary conditions. we want to apply this before we compute
      //    the laplacian since it effectively fixes the laplacian equal to zero on the boundary.
      let fix_boundary_conditions_bg = helper_compute_bind_group(
         device, None, &fix_boundary_conditions_ppln,
         &[&data_buffer, &width_buffer, &height_buffer]
      );

      // compute laplacian of data
      let stage_one_bind_group = helper_compute_bind_group(
         device, None, &laplacian_pipeline,
         &[&data_buffer, &laplacian_buffer, &width_buffer, &height_buffer]
      );
      // compute RK2 midpoint using laplacian
      let stage_two_bind_group = helper_compute_bind_group(
         device, None, &iterate_pipeline,
         &[&data_buffer, &laplacian_buffer, &midpoint_buffer, &width_buffer, &height_buffer, &kappa_buffer, &delta_t_2_buffer]
      );
      // reuse laplacian pipeline to compute laplacian using midpoint buffer
      let stage_three_bind_group = helper_compute_bind_group(
         device, None, &laplacian_pipeline,
         &[&midpoint_buffer, &midpoint_laplacian_buffer, &width_buffer, &height_buffer]
      );
      // reuse RK2 midpoint pipeline but with delta_t instead of delta_t/2 to compute RK2 result
      let stage_four_bind_group = helper_compute_bind_group(
         device, None, &iterate_pipeline,
         &[&data_buffer, &midpoint_laplacian_buffer, &output_buffer, &width_buffer, &height_buffer, &kappa_buffer, &delta_t_buffer]
      );
      // send output buffer to data buffer so we can repeat this
      let stage_five_bind_group = helper_compute_bind_group(
         device, None, &buffer_move_pipeline,
         &[&output_buffer, &data_buffer, &width_buffer, &height_buffer]
      );

      #[allow(non_snake_case)]
      let vis_minT_buffer = helper_param_buffer(device,Some("minT"),4);
      #[allow(non_snake_case)]
      let vis_maxT_buffer = helper_param_buffer(device,Some("maxT"),4);
      let heat_hue_shader = device.create_shader_module(wgpu::include_wgsl!("heatcolor.wgsl"));
      let heat_hue_pipeline = helper_basic_compute_shader(device, Some("Heatmap Pipeline"), &heat_hue_shader);
      let heat_hue_buffer = device.create_buffer(&wgpu::BufferDescriptor {
         label: Some("heatmap"),
         size: (256 * width.div_ceil(64) * height) as u64,
         usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
         mapped_at_creation: false
      });
      let heat_hue_bind_group = helper_compute_bind_group(
         device, None, &heat_hue_pipeline,
         &[&data_buffer, &heat_hue_buffer, &vis_minT_buffer, &vis_maxT_buffer, &width_buffer, &height_buffer, &pad_buffer]
      );

      Self {
         width,
         height,
         pad_per_line,
         workgroup_size: 64,
         fix_boundary_conditions_shdr,
         laplacian_shader,
         iterate_shader,
         buffer_move_shader,
         fix_boundary_conditions_ppln,
         laplacian_pipeline,
         iterate_pipeline,
         buffer_move_pipeline,
         data_buffer,
         laplacian_buffer,
         midpoint_buffer,
         midpoint_laplacian_buffer,
         output_buffer,
         export_buffer,
         width_buffer,
         height_buffer,
         kappa_buffer,
         delta_t_buffer,
         delta_t_2_buffer,
         pad_buffer,

         fix_boundary_conditions_bg,
         stage_one_bind_group,
         stage_two_bind_group,
         stage_three_bind_group,
         stage_four_bind_group,
         stage_five_bind_group,

         vis_minT_buffer,
         vis_maxT_buffer,
         heat_map_buffer: heat_hue_buffer,
         heat_hue_shader,
         heat_hue_pipeline,
         heat_hue_bind_group,

         iteration_quantity: 100,

         workgroup_quantity: initial_data.len().div_ceil(64) as u32,

         #[cfg(not(target_arch = "wasm32"))]
         progress: None,

         #[cfg(target_arch = "wasm32")]
         progress: Arc::new(Mutex::new(None))
      }
   }

   pub fn update_values(
      &mut self,
      queue: &wgpu::Queue,
      n_times: u32,
      kappa: f32,
      delta_t: f32,
      #[allow(non_snake_case)] minT: f32,
      #[allow(non_snake_case)] maxT: f32
   ) {
      self.iteration_quantity = n_times;
      queue.write_buffer(&self.kappa_buffer, 0, cast_slice(&[kappa]));
      queue.write_buffer(&self.delta_t_buffer, 0, cast_slice(&[delta_t.clone()]));
      queue.write_buffer(&self.delta_t_2_buffer, 0, cast_slice(&[delta_t / 2.0]));
      queue.write_buffer(&self.vis_minT_buffer, 0, cast_slice(&[minT]));
      queue.write_buffer(&self.vis_maxT_buffer, 0, cast_slice(&[maxT]));
      queue.submit([]);


   }

   pub fn send_compute_job(
      &mut self,
      pending_queue: &mut Vec<wgpu::CommandBuffer>,
      device: &wgpu::Device,
   ) {
      let mut encoder = device.create_command_encoder(&Default::default());

      // these braces are to make sure we return any refs borrowed in them.
      //    principly, encoder must be returned since it is borrowed by
      //    begin_compute_pass, and is needed so we can call encoder.finish().
      {
         let mut gputodo = encoder.begin_compute_pass(&Default::default());

         // this is also where we define the steps the gpu should take. so far as
         //    i can tell, this is similar to sending an io monad to the gpu

         let workgroup_quantity = (self.width * self.height).div_ceil(64) as u32;
         let x_workgroup_quantity = self.width.div_ceil(8) as u32;
         let y_workgroup_quantity = self.height.div_ceil(8) as u32;
         let boundary_conds_wg_quant = (self.width*2 + self.height*2).div_ceil(self.workgroup_size);

         for _ in 0..self.iteration_quantity {
         gputodo.set_pipeline(&self.fix_boundary_conditions_ppln);
         gputodo.set_bind_group(0, &self.fix_boundary_conditions_bg, &[]);
         gputodo.dispatch_workgroups(boundary_conds_wg_quant, 1 , 1);

         gputodo.set_pipeline(&self.laplacian_pipeline);
         gputodo.set_bind_group(0, &self.stage_one_bind_group, &[]);
         gputodo.dispatch_workgroups(x_workgroup_quantity, y_workgroup_quantity, 1);

         gputodo.set_pipeline(&self.iterate_pipeline);
         gputodo.set_bind_group(0, &self.stage_two_bind_group, &[]);
         gputodo.dispatch_workgroups(workgroup_quantity, 1, 1);

         gputodo.set_pipeline(&self.laplacian_pipeline);
         gputodo.set_bind_group(0, &self.stage_three_bind_group, &[]);
         gputodo.dispatch_workgroups(x_workgroup_quantity, y_workgroup_quantity, 1);

         gputodo.set_pipeline(&self.iterate_pipeline);
         gputodo.set_bind_group(0, &self.stage_four_bind_group, &[]);
         gputodo.dispatch_workgroups(workgroup_quantity, 1, 1);

         gputodo.set_pipeline(&self.buffer_move_pipeline);
         gputodo.set_bind_group(0, &self.stage_five_bind_group, &[]);
         gputodo.dispatch_workgroups(workgroup_quantity, 1, 1);
         }
      }

      // #[cfg(target_arch = "wasm32")]
      // let progress_ref = {
      //    let temp_ref = self.progress.clone();
      //    {
      //       let mut progress = temp_ref.lock().unwrap();
      //       *progress = Some(ComputeRelevantEvent::ComputeIsWorking);
      //    }
      //    temp_ref
      // };

      pending_queue.push(encoder.finish())
      //queue.submit([encoder.finish()]);

      // #[cfg(not(target_arch = "wasm32"))]
      // {
      //    let (sender,receiver) = channel();

      //    queue.on_submitted_work_done(move || sender.send( ComputeRelevantEvent::ComputeDoneNowColor ).unwrap());
      //    self.progress = Some((ComputeRelevantEvent::ComputeIsWorking, receiver));
      // }

      // #[cfg(target_arch = "wasm32")]
      // queue.on_submitted_work_done(move || {
      //    let mut progress = progress_ref.lock().unwrap();
      //    *progress = Some(ComputeRelevantEvent::ComputeDoneNowColor);
      // });
   }

   pub fn send_color_job(
      &mut self,
      pending_queue: &mut Vec<wgpu::CommandBuffer>,
      device: &wgpu::Device,
   ) {
      let mut encoder = device.create_command_encoder(&Default::default());

      let x_workgroup_quantity = self.width.div_ceil(8) as u32;
      let y_workgroup_quantity = self.height.div_ceil(8) as u32;

      {
         let mut gputodo = encoder.begin_compute_pass(&Default::default());

         gputodo.set_pipeline(&self.heat_hue_pipeline);
         gputodo.set_bind_group(0, &self.heat_hue_bind_group, &[]);
         gputodo.dispatch_workgroups(x_workgroup_quantity, y_workgroup_quantity, 1);
      }

      #[cfg(target_arch = "wasm32")]
      let progress_ref = {
         let temp_ref = self.progress.clone();
         {
            let mut progress = temp_ref.lock().unwrap();
            *progress = Some(ComputeRelevantEvent::ColorIsWorking);
         }
         temp_ref
      };

      pending_queue.push(encoder.finish())
      //queue.submit([encoder.finish()]);

      // #[cfg(not(target_arch = "wasm32"))]
      // {
      //    let (sender,receiver) = channel();

      //    queue.on_submitted_work_done(move || sender.send( ComputeRelevantEvent::ColorIsDone ).unwrap());
      //    self.progress = Some((ComputeRelevantEvent::ColorIsWorking, receiver));
      // }

      // #[cfg(target_arch = "wasm32")]
      // queue.on_submitted_work_done(move || {
      //    let mut progress = progress_ref.lock().unwrap();
      //    *progress = Some(ComputeRelevantEvent::ColorIsDone);
      // });
   }

   pub fn unsafe_queue_color_job(
      &self,
      encoder: &mut wgpu::CommandEncoder
   ) {
      let mut gputodo = encoder.begin_compute_pass(&Default::default());

      let x_workgroup_quantity = self.width.div_ceil(8) as u32;
      let y_workgroup_quantity = self.height.div_ceil(8) as u32;

      gputodo.set_pipeline(&self.heat_hue_pipeline);
      gputodo.set_bind_group(0, &self.heat_hue_bind_group, &[]);
      gputodo.dispatch_workgroups(x_workgroup_quantity, y_workgroup_quantity, 1);
   }

   pub fn color_to_texture(
      &mut self,
      pending_queue: &mut Vec<wgpu::CommandBuffer>,
      device: &wgpu::Device,
      textureBuffer: &wgpu::Texture,
   ) {
      let mut encoder = device.create_command_encoder(&Default::default());
      encoder.copy_buffer_to_texture(
         wgpu::TexelCopyBufferInfo {
            buffer: &self.heat_map_buffer,
            layout: wgpu::TexelCopyBufferLayout {
               offset: 0,
               bytes_per_row: Some(256 * self.width.div_ceil(64)),
               rows_per_image: Some(self.height)
            }
         },
         wgpu::TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            texture: textureBuffer,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO
         },
         textureBuffer.size()
      );

      #[cfg(target_arch = "wasm32")]
      let progress_ref = {
         let temp_ref = self.progress.clone();
         {
            let mut progress = temp_ref.lock().unwrap();
            *progress = Some(ComputeRelevantEvent::ColorIsCopying);
         }
         temp_ref
      };

      pending_queue.push(encoder.finish());

      // #[cfg(not(target_arch = "wasm32"))]
      // {
      //    let (sender,receiver) = channel();

      //    queue.on_submitted_work_done(move || sender.send( ComputeRelevantEvent::ComputeDoneNowColor ).unwrap());
      //    self.progress = Some((ComputeRelevantEvent::ColorIsCopying ,receiver));
      // }

      // #[cfg(target_arch = "wasm32")]
      // queue.on_submitted_work_done(move || {
      //    let mut progress = progress_ref.lock().unwrap();
      //    *progress = None;
      // });
   }

   pub fn unsafe_color_to_texture_queue(
      &self,
      encoder: &mut wgpu::CommandEncoder,
      textureBuffer: &wgpu::Texture,
   ) {
      encoder.copy_buffer_to_texture(
         wgpu::TexelCopyBufferInfo {
            buffer: &self.heat_map_buffer,
            layout: wgpu::TexelCopyBufferLayout {
               offset: 0,
               bytes_per_row: Some(256 * self.width.div_ceil(64)),
               rows_per_image: Some(self.height)
            }
         },
         wgpu::TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            texture: textureBuffer,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO
         },
         textureBuffer.size()
      );
   }

   #[cfg(not(target_arch = "wasm32"))]
   pub async fn export_heatmap_buffer(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Option<()> {
    use crate::aspng::PngConfig;

      let mut encoder = device.create_command_encoder(&Default::default());

      encoder.copy_buffer_to_buffer(&self.heat_map_buffer, 0, &self.export_buffer, 0, self.heat_map_buffer.size());

      // we submit the todo list
      queue.submit([encoder.finish()]);

      let colordata: Vec<u8> = {
         let (sender,receiver) = channel();

         self.export_buffer.map_async(wgpu::MapMode::Read, ..,
            move |result| sender.send(result).unwrap());

         device.poll(wgpu::PollType::wait_indefinitely()).ok()?;

         _ = receiver.await.ok()?;

         let output_data = self.export_buffer.get_mapped_range(..);

         bytemuck::cast_slice(&output_data).to_vec()
      };

      PngConfig::default().writeDataAtPath(
         &colordata,
         self.width,
         self.height,
         std::path::Path::new("checkthis.png")
      );

      Some(())
   }
}
