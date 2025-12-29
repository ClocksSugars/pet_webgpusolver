use wgpu::util::{BufferInitDescriptor, DeviceExt};
use bytemuck::cast_slice;


#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::oneshot::{Receiver, channel};

#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};

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
   length: u32,
   workgroup_size: u32,
   laplacian_shader: wgpu::ShaderModule,
   iterate_shader: wgpu::ShaderModule,
   buffer_move_shader: wgpu::ShaderModule,
   laplacian_pipeline: wgpu::ComputePipeline,
   iterate_pipeline: wgpu::ComputePipeline,
   buffer_move_pipeline: wgpu::ComputePipeline,
   data_buffer: wgpu::Buffer,
   laplacian_buffer: wgpu::Buffer,
   midpoint_buffer: wgpu::Buffer,
   midpoint_laplacian_buffer: wgpu::Buffer,
   output_buffer: wgpu::Buffer,
   export_buffer: wgpu::Buffer,
   length_buffer: wgpu::Buffer,
   kappa_buffer: wgpu::Buffer,
   delta_t_buffer: wgpu::Buffer,
   delta_t_2_buffer: wgpu::Buffer,

   // When i have more confidence, these should be an vec of 'steps'
   //    although that may require the above pipelines to be changed to
   //    Arc<wgpu::Buffer> types.
   stage_one_bind_group: wgpu::BindGroup,
   stage_two_bind_group: wgpu::BindGroup,
   stage_three_bind_group: wgpu::BindGroup,
   stage_four_bind_group: wgpu::BindGroup,
   stage_five_bind_group: wgpu::BindGroup,

   vis_minT_buffer: wgpu::Buffer,
   vis_maxT_buffer: wgpu::Buffer,
   heat_map_buffer: wgpu::Buffer,
   heat_hue_shader: wgpu::ShaderModule,
   heat_hue_pipeline: wgpu::ComputePipeline,
   heat_hue_bind_group: wgpu::BindGroup,

   workgroup_quantity: u32,


   // NOTE: my understanding is that wasm will not start threads in the same way rust will.
   //    for this reason it makes sense to use a sender-receiver in rust to avoid thread stuff
   //    and an arc-mutex in wasm which ostensibly just lets us send a message that doesnt anger
   //    the rust-compiler borrow checker.

   #[cfg(not(target_arch = "wasm32"))]
   progress: Option<
      (ComputeRelevantEvent, Receiver<ComputeRelevantEvent>)
   >,

   #[cfg(target_arch = "wasm32")]
   progress: Arc<
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
      length: u32,
      device: &wgpu::Device,
      //queue: &wgpu::Queue,
   ) -> Self {
      assert_eq!(initial_data.len() as u32,length*length);

      let laplacian_shader = device.create_shader_module(wgpu::include_wgsl!("laplacian.wgsl"));
      let iterate_shader = device.create_shader_module(wgpu::include_wgsl!("iterate_heat.wgsl"));
      let buffer_move_shader = device.create_shader_module(wgpu::include_wgsl!("buffer_move.wgsl"));

      let laplacian_pipeline = helper_basic_compute_shader(device, Some("Laplacian Pipeline"), &laplacian_shader);
      let iterate_pipeline = helper_basic_compute_shader(device, Some("Iteration Pipeline"), &iterate_shader);
      let buffer_move_pipeline = helper_basic_compute_shader(device, Some("Relocation Pipeline"), &buffer_move_shader);

      let data_buffer = device.create_buffer_init(&BufferInitDescriptor {
         label: Some("data"),
         contents: bytemuck::cast_slice(&initial_data),
         usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
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


      let length_buffer = device.create_buffer_init(&BufferInitDescriptor {
          label: Some("length"),
          contents: bytemuck::cast_slice(&[length]),
          usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      });
      let kappa_buffer = helper_param_buffer(device,Some("kappa"),4);
      let delta_t_buffer = helper_param_buffer(device,Some("delta_t"),4);
      // this is just the same number divided by two so we can reuse the pipeline
      let delta_t_2_buffer = helper_param_buffer(device,Some("delta_t_2"),4);

      // if pipelines are like gpu function calls, this is where we identify our variables in address space.
      // in that sense we may freely put in different buffers like swapping arguments to a function
      let stage_one_bind_group = helper_compute_bind_group(
         device, None, &laplacian_pipeline,
         &[&data_buffer, &laplacian_buffer, &length_buffer]
      );
      let stage_two_bind_group = helper_compute_bind_group(
         device, None, &iterate_pipeline,
         &[&data_buffer, &laplacian_buffer, &midpoint_buffer, &length_buffer, &kappa_buffer, &delta_t_2_buffer]
      );
      let stage_three_bind_group = helper_compute_bind_group(
         device, None, &laplacian_pipeline,
         &[&midpoint_buffer, &midpoint_laplacian_buffer, &length_buffer]
      );
      let stage_four_bind_group = helper_compute_bind_group(
         device, None, &iterate_pipeline,
         &[&data_buffer, &midpoint_laplacian_buffer, &output_buffer, &length_buffer, &kappa_buffer, &delta_t_buffer]
      );
      let stage_five_bind_group = helper_compute_bind_group(
         device, None, &buffer_move_pipeline,
         &[&output_buffer, &data_buffer, &length_buffer]
      );

      #[allow(non_snake_case)]
      let vis_minT_buffer = helper_param_buffer(device,Some("minT"),4);
      #[allow(non_snake_case)]
      let vis_maxT_buffer = helper_param_buffer(device,Some("maxT"),4);
      let heat_hue_shader = device.create_shader_module(wgpu::include_wgsl!("heatcolor.wgsl"));
      let heat_hue_pipeline = helper_basic_compute_shader(device, Some("Heatmap Pipeline"), &heat_hue_shader);
      let heat_hue_buffer = device.create_buffer(&wgpu::BufferDescriptor {
         label: Some("heatmap"),
         size: data_buffer.size(), // this is only bc f32 has same size as 4 * u8, which is what we need for RGBA
         usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
         mapped_at_creation: false
      });
      let heat_hue_bind_group = helper_compute_bind_group(
         device, None, &heat_hue_pipeline,
         &[&data_buffer, &heat_hue_buffer, &vis_minT_buffer, &vis_maxT_buffer]
      );



      Self {
         length,
         workgroup_size: 64,
         laplacian_shader,
         iterate_shader,
         buffer_move_shader,
         laplacian_pipeline,
         iterate_pipeline,
         buffer_move_pipeline,
         data_buffer,
         laplacian_buffer,
         midpoint_buffer,
         midpoint_laplacian_buffer,
         output_buffer,
         export_buffer,
         length_buffer,
         kappa_buffer,
         delta_t_buffer,
         delta_t_2_buffer,

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

         workgroup_quantity: initial_data.len().div_ceil(64) as u32,

         #[cfg(not(target_arch = "wasm32"))]
         progress: None,

         #[cfg(target_arch = "wasm32")]
         progress: Arc::new(Mutex::new(None))
      }
   }

   pub fn update_values(
      &self,
      queue: &wgpu::Queue,
      kappa: f32,
      delta_t: f32,
      #[allow(non_snake_case)] minT: f32,
      #[allow(non_snake_case)] maxT: f32
   ) {
      queue.write_buffer(&self.kappa_buffer, 0, cast_slice(&[kappa]));
      queue.write_buffer(&self.delta_t_buffer, 0, cast_slice(&[delta_t.clone()]));
      queue.write_buffer(&self.delta_t_2_buffer, 0, cast_slice(&[delta_t / 2.0]));
      queue.write_buffer(&self.vis_minT_buffer, 0, cast_slice(&[minT]));
      queue.write_buffer(&self.vis_maxT_buffer, 0, cast_slice(&[maxT]));
   }

   pub fn send_compute_job(
      &mut self,
      queue: &wgpu::Queue,
      device: &wgpu::Device,
      n_times: usize,
   ) {
      let mut encoder = device.create_command_encoder(&Default::default());

      // these braces are to make sure we return any refs borrowed in them.
      //    principly, encoder must be returned since it is borrowed by
      //    begin_compute_pass, and is needed so we can call encoder.finish().
      {
         let mut gputodo = encoder.begin_compute_pass(&Default::default());

         // this is also where we define the steps the gpu should take. so far as
         //    i can tell, this is similar to sending an io monad to the gpu


         for _ in 0..n_times {
         gputodo.set_pipeline(&self.laplacian_pipeline);
         gputodo.set_bind_group(0, &self.stage_one_bind_group, &[]);
         gputodo.dispatch_workgroups(self.workgroup_quantity, 1, 1);

         gputodo.set_pipeline(&self.iterate_pipeline);
         gputodo.set_bind_group(0, &self.stage_two_bind_group, &[]);
         gputodo.dispatch_workgroups(self.workgroup_quantity, 1, 1);

         gputodo.set_pipeline(&self.laplacian_pipeline);
         gputodo.set_bind_group(0, &self.stage_three_bind_group, &[]);
         gputodo.dispatch_workgroups(self.workgroup_quantity, 1, 1);

         gputodo.set_pipeline(&self.iterate_pipeline);
         gputodo.set_bind_group(0, &self.stage_four_bind_group, &[]);
         gputodo.dispatch_workgroups(self.workgroup_quantity, 1, 1);

         gputodo.set_pipeline(&self.buffer_move_pipeline);
         gputodo.set_bind_group(0, &self.stage_five_bind_group, &[]);
         gputodo.dispatch_workgroups(self.workgroup_quantity, 1, 1);
         }
      }

      #[cfg(target_arch = "wasm32")]
      let progress_ref = {
         let temp_ref = self.progress.clone();
         {
            let mut progress = temp_ref.lock().unwrap();
            *progress = Some(ComputeRelevantEvent::ComputeIsWorking);
         }
         temp_ref
      };

      queue.submit([encoder.finish()]);

      #[cfg(not(target_arch = "wasm32"))]
      {
         let (sender,receiver) = channel();

         queue.on_submitted_work_done(move || sender.send( ComputeRelevantEvent::ComputeDoneNowColor ).unwrap());
         self.progress = Some((ComputeRelevantEvent::ComputeIsWorking, receiver));
      }

      #[cfg(target_arch = "wasm32")]
      queue.on_submitted_work_done(move || {
         let mut progress = progress_ref.lock().unwrap();
         *progress = Some(ComputeRelevantEvent::ComputeDoneNowColor);
      });
   }

   pub fn send_color_job(
      &mut self,
      queue: &wgpu::Queue,
      device: &wgpu::Device,
   ) {
      let mut encoder = device.create_command_encoder(&Default::default());

      {
         let mut gputodo = encoder.begin_compute_pass(&Default::default());

         gputodo.set_pipeline(&self.heat_hue_pipeline);
         gputodo.set_bind_group(0, &self.heat_hue_bind_group, &[]);
         gputodo.dispatch_workgroups(self.workgroup_quantity, 1, 1);
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

      queue.submit([encoder.finish()]);

      #[cfg(not(target_arch = "wasm32"))]
      {
         let (sender,receiver) = channel();

         queue.on_submitted_work_done(move || sender.send( ComputeRelevantEvent::ColorIsDone ).unwrap());
         self.progress = Some((ComputeRelevantEvent::ColorIsWorking, receiver));
      }

      #[cfg(target_arch = "wasm32")]
      queue.on_submitted_work_done(move || {
         let mut progress = progress_ref.lock().unwrap();
         *progress = Some(ComputeRelevantEvent::ColorIsDone);
      });
   }

   pub fn unsafe_queue_color_job(
      &self,
      encoder: &mut wgpu::CommandEncoder
   ) {
      let mut gputodo = encoder.begin_compute_pass(&Default::default());

      gputodo.set_pipeline(&self.heat_hue_pipeline);
      gputodo.set_bind_group(0, &self.heat_hue_bind_group, &[]);
      gputodo.dispatch_workgroups(self.workgroup_quantity, 1, 1);
   }

   pub fn color_to_texture(
      &mut self,
      queue: &wgpu::Queue,
      device: &wgpu::Device,
      textureBuffer: &wgpu::Texture,
   ) {
      let mut encoder = device.create_command_encoder(&Default::default());
      encoder.copy_buffer_to_texture(
         wgpu::TexelCopyBufferInfo {
            buffer: &self.heat_map_buffer,
            layout: wgpu::TexelCopyBufferLayout {
               offset: 0,
               bytes_per_row: Some(4 * self.length),
               rows_per_image: Some(self.length)
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

      queue.submit([encoder.finish()]);

      #[cfg(not(target_arch = "wasm32"))]
      {
         let (sender,receiver) = channel();

         queue.on_submitted_work_done(move || sender.send( ComputeRelevantEvent::ComputeDoneNowColor ).unwrap());
         self.progress = Some((ComputeRelevantEvent::ColorIsCopying ,receiver));
      }

      #[cfg(target_arch = "wasm32")]
      queue.on_submitted_work_done(move || {
         let mut progress = progress_ref.lock().unwrap();
         *progress = None;
      });
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
               bytes_per_row: Some(4 * self.length),
               rows_per_image: Some(self.length)
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
         self.length,
         std::path::Path::new("checkthis.png")
      );

      Some(())
   }
}
