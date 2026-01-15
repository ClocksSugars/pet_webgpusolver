@group(0) @binding(0) var<storage, read> data: array<f32>;
@group(0) @binding(1) var<storage, read_write> rgba_out: array<u32>;
@group(0) @binding(2) var<uniform> minT: f32;
@group(0) @binding(3) var<uniform> maxT: f32;
@group(0) @binding(4) var<uniform> width: u32;
@group(0) @binding(5) var<uniform> height: u32;
@group(0) @binding(6) var<uniform> pad_per_line: u32;



// GOING TO HAVE TO DO SOME EVIL BITWISE MANIPULATION HERE


@compute
@workgroup_size(8,8,1)
fn main(
   @builtin(global_invocation_id) gid: vec3<u32>
) {

   if ((gid.x >= width) | (gid.y >= height)) {return;}

   let range = clamp((data[gid.x + gid.y * width] - minT)/(maxT - minT), 0.0f, 1.0f);

   var red: f32 = 0;
   var green: f32 = 0;
   var blue: f32 = 0;

   // this is obviously not a general hsv to rgb. we are a just aiming for a rough heat color
   //    so considerations of the magenta part of the hue wheel dont matter to us
   switch u32(floor(range * 4)) {
      case 0: {
         blue = 1.0f;
         green = 4.0f * range;
      }
      case 1: {
         green = 1.0f;
         blue = 1.0f - 4.0f * (range - 0.25);
      }
      case 2: {
         green = 1.0f;
         red = 4.0f * (range - 0.5);
      }
      case 3: {
         green = 1.0f - 4.0f * (range - 0.75);
         red = 1.0f;
      }
      default: {
         red = 1.0f;
      }
   }

   // we use floating point multiplication on f32 representatives of the u8 255 numbers we want
   //    then bitwise clamp them so color bits dont leak into each other. due to evil bit reversed
   //    storage shenanigans the RGBA is actually ABGR
   var color: u32 = 0xFF000000 //0xFF000000 // 255 in opacity
      + (u32(0000000255.0f * red    ) & 0x000000FF)   // 0000000255 is 0x000000FF
      + (u32(0000065280.0f * green  ) & 0x0000FF00)   // 0000065280 is 0x0000FF00
      + (u32(0016711680.0f * blue   ) & 0x00FF0000)   // 0016711680 is 0x00FF0000
   ;

   rgba_out[gid.x + gid.y * (pad_per_line + width)] = color;
}
