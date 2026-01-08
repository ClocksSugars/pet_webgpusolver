@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@group(0) @binding(1) var<uniform> width: u32;
@group(0) @binding(2) var<uniform> height: u32;

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

   // we need 2 * width + 2 * height with different behaviours for each side
   // regard adding width as ostensibly y+=delta_y since each y coordinate position is separated by a width


   if (gid.x < width) {                                               // side y=0 line
      if (gid.x == 0) {
         //corner, set (0,0) value to (delta_x,delta_y) value
         data[0] = data[width + 1];
      } else if (gid.x == width){
         //corner, set (1,0) value to (1 - delta_x,delta_y) value
         data[width] = data[2 * width - 2];
      } else {
         // set (x,0) values to (x,delta_y) values
         data[gid.x] = data[gid.x + width];
      }
      return;
   } else if (gid.x < width + height ){                                     // side 2
      // in these cases we must regard gid.x as y+width since we havent subtracted that
      if ((gid.x == width) | (gid.x == ((width + height) - 1))) {
         return; // corners, handled by sides 1 and 3
      }
      // set (0,y) values to (delta_x, y) values
      let indexwecareabout = (gid.x - width) * width; // y axis
      data[indexwecareabout] = data[indexwecareabout + 1];
      return;
   } else if (gid.x < (2*width) + height ){                                       // side 3
      // in these cases we must regard gid.x as x+width+height since we havent subtracted that
      if (gid.x == width + height) {
         // corner, set (0,1) value to (delta_x, 1 - delta_y) value
         data[width * (height - 1)] = data[width * (height - 2) + 1];
      } else if (gid.x == (2*width) + height - 1) {
         // corner, set (1,1) value to (1 - delta_x, 1 - delta_y) value
         data[width * height] = data[width * (height - 1) - 2];
      } else {
         // set (x,1) values to (x, 1 - delta_y) values
         let indexwecareabout = (gid.x - width - height ) + (width * (height - 1));
         data[indexwecareabout] = data[indexwecareabout - width];
      }
      return;
   } else if (gid.x < 2 * width + 2 * height ){                                       // side 4
      if ((gid.x == (2*width) + height) | (gid.x == 2 * width + 2 * height - 1)) {
         return; // corners, handled by sides 1 and 3
      } else {
         // the +1 before we multiply by width is so we are one more row than we want, then the -1 takes us to the y=1 side of the previous row
         let indexwecareabout = (gid.x - ((2*width) + height) + 1) * width - 1;
         data[indexwecareabout] = data[indexwecareabout - 1];
         return;
      }
   } else { return; }



}
