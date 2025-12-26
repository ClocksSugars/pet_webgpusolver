use core::f64;

use bytemuck::{AnyBitPattern, NoUninit};
use color_space::{Hsv, Rgb, ToRgb};




pub struct SquareGrid {
   array: Vec<f32>,
   ijmax: usize
}

/// This is a grid assumed to model [0,1]x[0,1]. BEWARE THE INCLUSION OF (1,1) FOR COUNTING
/// Precision is left open ended with type T.
impl SquareGrid {
   pub fn new(length: usize) -> Self
   {
      Self {
         array: [0.].repeat(length * length),
         ijmax: length,
      }
   }
   /// not the same as array length, length as in side of a square
   pub fn length(&self) -> usize {self.ijmax}
   pub fn getarray(&self) -> &Vec<f32> {&self.array}
   pub fn arrayasslice<S>(&self) -> &[S]
   where
      S: AnyBitPattern
   {
      bytemuck::cast_slice(&(self.array))
   }


   /// for convenience in other methods. there should not be a good reason
   ///   to edit elements of a grid otherwise, since they are either initial
   ///   conditions or solutions.
   fn setelement(&mut self, i: usize, j: usize, newval: f32) {
      self.array[i + j * self.ijmax] = newval
   }

   pub fn setbyfunc(&mut self, f: impl Fn(f32,f32) -> f32) {
      for n in 0..(self.ijmax*self.ijmax) {
         let i: usize = n % self.ijmax;
         let j: usize = n / self.ijmax;
         // SINCE (1,1) IS INCLUDED WE MUST SUBTRAC 1 FROM IJMAX
         //   OR ELSE X=1 AND Y=1 WILL NOT OCCUR
         let x: f32 = (i as f32) / ((self.ijmax -1) as f32);
         let y: f32 = (j as f32) / ((self.ijmax -1) as f32);
         self.setelement(i,j, f(x,y))
      }
   }

   pub fn newbyfunc(length: usize, f: impl Fn(f32,f32) -> f32) -> Self
   {
      let mut temp: SquareGrid = SquareGrid::new(length);
      temp.setbyfunc(f);
      temp
   }

   pub fn outasheatmap(&self, minT: f64, maxT: f64) -> Vec<u8>
   {
      let diff: f64 = maxT - minT;
      let mut result: Vec<u8> = vec![0; self.array.len() * 4];
      let mut rgb: Rgb = Rgb::new(0.,0.,0.);
      let mut zone: f64 = 0.;
      for n in 0..(self.ijmax*self.ijmax) {
         zone = (((self.array[n] as f64) - minT)/diff);
         // BEWARE HSV HUE IS FROM 0 TO 360
         rgb = Hsv::new(
            match (zone < 0., zone > 1.) {
               (false,false) => 240. + (zone * -240.),
               (true,_) =>  240.,
               (_, true) => 0.
            },
            1.,
            1.
         ).to_rgb();
         //println!("{}",zone * 255.);
         result[ 4 * n     ] = rgb.r as u8;
         result[(4 * n)+1  ] = rgb.g as u8;
         result[(4 * n)+2  ] = rgb.b as u8;
         result[(4 * n)+3  ] = 255 as u8;
      }
      result
   }

   pub fn newbytemplate(&self, newdata: Vec<f32>) -> SquareGrid {
      assert_eq!(self.array.len(),newdata.len());

      SquareGrid { array: newdata, ijmax: self.ijmax }
   }
}
