use std::primitive::f64;
use std::path::Path;

mod squaregrid;
mod aspng;

use crate::aspng::*;
use crate::squaregrid::*;



fn sqnum<T>(x:T) -> T where T: std::ops::Mul<Output = T> + Copy{
   x * x
}

fn makemiddleRatTinitconds(r: f32, t:f32 ) -> impl Fn(f32,f32) -> f32 {
   let rsquared = sqnum(r);
   move |x:f32,y:f32| -> f32 {
      match (sqnum(x-0.5) + sqnum(y-0.5)) < rsquared {
         true => t,
         false => 0.
      }
   }
}

fn makegaussianinitconds(t:f32 ) -> impl Fn(f32,f32) -> f32 {
   move |x:f32,y:f32| -> f32 {
      t * ((-10. * (sqnum(x-0.5) + sqnum(y-0.5))) as f64).exp() as f32
   }
}

fn main() {
   println!("Hello, world!");
   let initialconditions: SquareGrid =
      //SquareGrid::newbyfunc(255, makemiddleRatTinitconds(0.2, 100.));
      SquareGrid::newbyfunc(255, makegaussianinitconds(100.));
   PngConfig::default().writeDataAtPath(
      &initialconditions.outasheatmap(0.,100.),
      255,
      Path::new("image.png")
   )
}
