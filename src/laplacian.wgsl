@group(0) @binding(0) var<storage, read> data: array<f32>;
@group(0) @binding(1) var<storage, read_write> laplacian: array<f32>;
@group(0) @binding(2) var<uniform> width: u32;
@group(0) @binding(3) var<uniform> height: u32;

@compute// Entrypoint
@workgroup_size(8,8,1)
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

   if ((gid.x == 0) | (gid.x >= width - 1)) {return;}
   if ((gid.y == 0) | (gid.y >= height - 1) ) {return;}

   // i presume the above is better since more threads exit sooner??
   // if (
   //       gid.x <= length                     // i.e. y=0
   //    |  gid.x >= length * (length - 1)      // i.e. y=1
   //    |  gid.x == roughj * length            // i.e. x=0
   //    |  gid.x == (roughj + 1) * length - 1  // i.e. x=1
   //    ) {
   //    return;
   // }

   let widthfloatmin1 = f32(width);
   let heightfloatmin1 = f32(height);
   let delta_x_sq = 1.0f / widthfloatmin1 / widthfloatmin1;
   let delta_y_sq = 1.0f / heightfloatmin1 / heightfloatmin1;

   let indexwecareabout = gid.x + gid.y * width;

   laplacian[indexwecareabout] =
      (
         data[indexwecareabout + 1]
         -2.0f * data[indexwecareabout]
         + data[indexwecareabout - 1]
      ) / delta_x_sq
      + (
         data[indexwecareabout + width]
         -2.0f * data[indexwecareabout]
         + data[indexwecareabout - width]
      ) / delta_y_sq;
}
