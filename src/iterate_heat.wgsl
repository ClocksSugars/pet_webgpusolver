@group(0) @binding(0) var<storage, read> data: array<f32>;
@group(0) @binding(1) var<storage, read> laplacian: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> width: u32;
@group(0) @binding(4) var<uniform> height: u32;
@group(0) @binding(5) var<uniform> kappa: f32;
@group(0) @binding(6) var<uniform> delta_t: f32;

@compute// Entrypoint
@workgroup_size(256,1,1)
fn main(
   //we use these to get which workgroup and where inside workgroup we are
   //@builtin(workgroup_id) wid: vec3<u32>,
   //@builtin(local_invocation_id) lid: vec3<u32>
   //
   // but this will just give us a global id equivalent to
   // wid * workgroup_size + lid
   @builtin(global_invocation_id) gid: vec3<u32>
) {
   // valuable reference: https://www.w3.org/TR/WGSL/#arithmetic-expr

   // // exit if on boundary. might be inefficient but easier to call too many workers
   if (gid.x > (width * height)) {return;}

   output[gid.x] = data[gid.x] + delta_t * kappa * laplacian[gid.x];
}
