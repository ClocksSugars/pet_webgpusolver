@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@group(0) @binding(1) var<storage, read_write> laplacian: array<f32>;
@group(0) @binding(2) var<storage, read_write> midpoint: array<f32>;
@group(0) @binding(3) var<storage, read_write> midpoint_laplacian: array<f32>;
@group(0) @binding(4) var<storage, read_write> output: array<f32>;
@group(0) @binding(5) var<uniform> length: u32;
@group(0) @binding(6) var<uniform> kappa: f32;
@group(0) @binding(7) var<uniform> delta_t: f32;

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

   // for ij indices. getting i requires % but we dont need it
   let roughj = gid.x / length;

   // exit if on boundary. might be inefficient but easier to call too many workers
   if (gid.x <= length) {return;}                     // i.e. y=0
   if (gid.x >= length * (length - 1)) {return;}      // i.e. y=1
   //if (gid.x == roughj * length) {return;}            // i.e. x=0
   //if (gid.x == (roughj + 1) * length - 1) {return;}  // i.e. x=1

   // i presume the above is better since more threads exit sooner??
   // if (
   //       gid.x <= length                     // i.e. y=0
   //    |  gid.x >= length * (length - 1)      // i.e. y=1
   //    |  gid.x == roughj * length            // i.e. x=0
   //    |  gid.x == (roughj + 1) * length - 1  // i.e. x=1
   //    ) {
   //    return;
   // }

   let lengthfloatmin1 = f32(length - 1);
   let delta_x_sq = 1.0f / lengthfloatmin1 / lengthfloatmin1;
   let delta_y_sq = 1.0f / lengthfloatmin1 / lengthfloatmin1;

   laplacian[gid.x] =
      (
         data[gid.x + 1]
         -2.0f * data[gid.x]
         + data[gid.x - 1]
      ) / delta_x_sq
      + (
         data[gid.x + length]
         -2.0f * data[gid.x]
         + data[gid.x - length]
      ) / delta_y_sq;

   storageBarrier();

   midpoint[gid.x] = data[gid.x] + laplacian[gid.x] * kappa * delta_t / 2.;

   storageBarrier();

   midpoint_laplacian[gid.x] =
      (
         midpoint[gid.x + 1]
         -2.0f * midpoint[gid.x]
         + midpoint[gid.x - 1]
      ) / delta_x_sq
      + (
         midpoint[gid.x + length]
         -2.0f * midpoint[gid.x]
         + midpoint[gid.x - length]
      ) / delta_y_sq;

   storageBarrier();

   output[gid.x] = data[gid.x] + delta_t * kappa * midpoint_laplacian[gid.x];

   storageBarrier();
   //data[gid.x] = data[gid.x] + laplacian[gid.x] * kappa * delta_t;
}
