use std::path::Path;
use std::fs::File;
use std::io::BufWriter;


use png;

pub struct PngConfig {
   color: png::ColorType,
   depth: png::BitDepth,
   gamma: png::ScaledFloat,
   chromaticities: png::SourceChromaticities
}

impl Default for PngConfig {
   fn default() -> Self {
      PngConfig {
         color: png::ColorType::Rgba,
         depth: png::BitDepth::Eight,
         gamma: png::ScaledFloat::new(1.0/2.2),
         chromaticities: png::SourceChromaticities::new(
            (0.31270, 0.32900),
            (0.64000, 0.33000),
            (0.30000, 0.60000),
            (0.15000, 0.06000)
         )
      }
   }
}

impl PngConfig {
   pub fn new(
      color: png::ColorType,
      depth: png::BitDepth,
      gamma: png::ScaledFloat,
      chromaticities: png::SourceChromaticities
   ) -> Self {
      Self {
         color: color,
         depth: depth,
         gamma: gamma,
         chromaticities: chromaticities
      }
   }

   pub fn writeDataAtPath(&self, data: &[u8], width: u32, height: u32, path: &Path) {
      let file = File::create(path).unwrap();
      let ref mut bufwriter = BufWriter::new(file);
      let mut encoder = png::Encoder::new(bufwriter, width, height);
      encoder.set_color(self.color);
      encoder.set_depth(self.depth);
      encoder.set_source_gamma(self.gamma);
      encoder.set_source_chromaticities(self.chromaticities);
      let mut writer = encoder.write_header().unwrap();
      writer.write_image_data(data).unwrap();
   }
}
