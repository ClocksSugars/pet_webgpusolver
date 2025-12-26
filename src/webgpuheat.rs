use tokio::sync::oneshot::channel;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use bytemuck::cast_slice;

pub async fn heatrun(
   data: &Vec<f32>,
   length: u32,
   kappa: f32,
   delta_t: f32
) -> Option<Vec<f32>> {
   let instance = wgpu::Instance::new(&Default::default());
   let adapter = instance.request_adapter(&Default::default()).await.ok()?;
   let (device, queue) = adapter.request_device(&Default::default()).await.ok()?;

   let shader = device.create_shader_module(wgpu::include_wgsl!("heat.wgsl"));

   let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
       label: Some("Compute Pipeline"),
       layout: None,
       module: &shader,
       entry_point: None,
       compilation_options: Default::default(),
       cache: Default::default(),
   });

   let data_buffer = device.create_buffer_init(&BufferInitDescriptor {
      label: Some("data"),
      contents: bytemuck::cast_slice(&data),
      usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
   });

   let laplacian_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("laplacian"),
      size: data_buffer.size(),
      usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
      mapped_at_creation: false,
   });

   let midpoint_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("midpoint"),
      size: data_buffer.size(),
      usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
      mapped_at_creation: false,
   });

   let midpoint_laplacian_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("midpoint laplacian"),
      size: data_buffer.size(),
      usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
      mapped_at_creation: false,
   });

   let output_buffer = device.create_buffer_init(&BufferInitDescriptor {
      label: Some("output"),
      contents: bytemuck::cast_slice(&data),
      usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
   });

   let export_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("export"),
      size: data_buffer.size(),
      usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
      mapped_at_creation: false,
   });

   let length_buffer = device.create_buffer_init(&BufferInitDescriptor {
       label: Some("length"),
       contents: bytemuck::cast_slice(&[length]),
       usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
   });

   let kappa_buffer = device.create_buffer_init(&BufferInitDescriptor {
       label: Some("kappa"),
       contents: bytemuck::cast_slice(&[kappa]),
       usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
   });

   let delta_t_buffer = device.create_buffer_init(&BufferInitDescriptor {
       label: Some("delta_t"),
       contents: bytemuck::cast_slice(&[delta_t]),
       usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
   });

   // THIS DEFINES WHAT IS AVAILABLE TO THE SHADER
   //    RESOURCES ARE IDENTIFIED BY THEIR BIND ID IN @binding(id)
   // note that we can also freely relabel buffers to different bindings
   //    and this effectively relabels them at the wgsl file
   let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &pipeline.get_bind_group_layout(0),
      entries: &[
         wgpu::BindGroupEntry {
               binding: 0,
               resource: data_buffer.as_entire_binding(),
         },
         wgpu::BindGroupEntry {
               binding: 1,
               resource: laplacian_buffer.as_entire_binding(),
         },
         wgpu::BindGroupEntry {
               binding: 2,
               resource: midpoint_buffer.as_entire_binding(),
         },
         wgpu::BindGroupEntry {
               binding: 3,
               resource: midpoint_laplacian_buffer.as_entire_binding(),
         },
         wgpu::BindGroupEntry {
               binding: 4,
               resource: output_buffer.as_entire_binding(),
         },
         wgpu::BindGroupEntry {
               binding: 5,
               resource: length_buffer.as_entire_binding(),
         },
         wgpu::BindGroupEntry {
               binding: 6,
               resource: kappa_buffer.as_entire_binding(),
         },
         wgpu::BindGroupEntry {
               binding: 7,
               resource: delta_t_buffer.as_entire_binding(),
         },
      ],
   });

   // encoder in the sense that this encodes the job to the gpu's job queue
   let mut encoder = device.create_command_encoder(&Default::default());

   // need this to match @workgroup_size in the wgsl file
   let workgroup_size = 256;
   let workgroup_quantity: u32 = data.len().div_ceil(workgroup_size) as u32;

   // these braces are to make sure we return any refs borrowed in them.
   //    principly, encoder must be returned since it is borrowed by
   //    begin_compute_pass, and is needed so we can call encoder.finish().
   {
      let mut gputodo = encoder.begin_compute_pass(&Default::default());

      // this is also where we define the steps the gpu should take. so far as
      //    i can tell, this is similar to sending an io monad to the gpu

      gputodo.set_pipeline(&pipeline);
      gputodo.set_bind_group(0, &bind_group, &[]);
      gputodo.dispatch_workgroups(workgroup_quantity, 1, 1);
   }

   // one must assume this is also ostensibly thrown into the gpu's todo list monad
   encoder.copy_buffer_to_buffer(&output_buffer, 0, &export_buffer, 0, data_buffer.size());

   // we submit the todo list
   queue.submit([encoder.finish()]);

   let final_output: Vec<f32> = {
      // Some async shenanigans mean we have to do something here.
      //    The tutorial I followed suggests that export_buffer.get_mapped_range
      //    only works after some async mapping process is complete. So we
      //    pass a function to export_buffer.map_async which can send a signal
      //    to receiver when done; we then await on the receiver to be certain
      //    that mapping the range is finished.
      let (sender,receiver) = channel();

      export_buffer.map_async(wgpu::MapMode::Read, ..,
         move |result| sender.send(result).unwrap());

      device.poll(wgpu::PollType::wait_indefinitely()).ok()?;

      _ = receiver.await.ok()?;

      let output_data = export_buffer.get_mapped_range(..);

      bytemuck::cast_slice(&output_data).to_vec()
      };

   Some(final_output)
}
