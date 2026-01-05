@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@group(0) @binding(1) var<uniform> length: u32;

@compute// Entrypoint
@workgroup_size(64,1,1)
fn main(
   //we use these to get which workgroup and where inside workgroup we are
   //@builtin(workgroup_id) wid: vec3<u32>,
   //@builtin(local_invocation_id) lid: vec3<u32>
   //
   // but this will just give us a global id equivalent to
   // wid * workgroup_size + lid
   @builtin(global_invocation_id) gid: vec3<u32>
) {

   // we need 4 * length with different behaviours for each side
   // regard mention of length as ostensibly y+=1


   if (gid.x < length) {                                               // side y=0 line
      if (gid.x == 0) {
         data[0] = data[length + 1]; //corner
      } else if (gid.x == length){
         data[length] = data[2 * length - 2]; //corner
      } else {
         data[gid.x] = data[gid.x + length];
      }
      return;
   } else if (gid.x < 2 * length ){                                      // side 2
      if ((gid.x == length) | (gid.x == ((2 * length) - 1))) {
         return; // corners, handled by lines 1 and 3
      }
      let indexwecareabout = (gid.x - length) * length; // y axis
      data[indexwecareabout] = data[indexwecareabout + 1];
      return;
   } else if (gid.x < 3 * length ){                                       // side 3
      if (gid.x == 2 * length) {
         data[(length - 1) * length] = data[(length - 2) * length + 1]; //corner
      } else if (gid.x == (3 * length) - 1){
         data[length * length] = data[((length - 1) * length) - 1]; //corner
      } else {
         let indexwecareabout = (gid.x - (2*length) ) + ((length - 1) * length);
         data[indexwecareabout] = data[indexwecareabout - length];
      }
      return;
   } else if (gid.x < 4 * length ){                                       // side 4
      if ((gid.x == (3 * length)) | (gid.x == ((4 * length) - 1))) {
         return; // corners, handled by lines 1 and 3
      } else {
         let indexwecareabout = (gid.x - (3*length) + 1) * length - 1;
         data[indexwecareabout] = data[indexwecareabout - 1];
         return;
      }
   } else { return; }



}
