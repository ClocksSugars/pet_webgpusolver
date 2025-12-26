use std::primitive::f64;
use std::path::Path;

mod squaregrid;
mod aspng;
mod webgpuheat;

use crate::aspng::*;
use crate::squaregrid::*;
use crate::webgpuheat::*;



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

#[tokio::main]
async fn main() {
   env_logger::init();
   let delta_t = 0.000001;
   let kappa = 1.;

   println!("Hello, world!");

   let initialconditions: SquareGrid =
      //SquareGrid::newbyfunc(255, makemiddleRatTinitconds(0.2, 400.));
      SquareGrid::newbyfunc(1023, makegaussianinitconds(400.));

   let bufferoutput: Vec<f32> = heatrun(
      initialconditions.getarray(),
      initialconditions.length() as u32,
      kappa,
      delta_t
   ).await.unwrap();
   let bufferoutput: SquareGrid =
      initialconditions
         .newbytemplate(bufferoutput);

   PngConfig::default().writeDataAtPath(
      &bufferoutput.outasheatmap(100.,400.),
      bufferoutput.length() as u32,
      Path::new("image0.png")
   );

   let bufferoutput: Vec<f32> = heatrun(
      bufferoutput.getarray(),
      bufferoutput.length() as u32,
      kappa,
      delta_t
   ).await.unwrap();

   let bufferoutput: SquareGrid =
      initialconditions
         .newbytemplate(bufferoutput);


   PngConfig::default().writeDataAtPath(
      &bufferoutput.outasheatmap(100.,400.),
      bufferoutput.length() as u32,
      Path::new("image1.png")
   );

   let bufferoutput: Vec<f32> = heatrun(
      bufferoutput.getarray(),
      bufferoutput.length() as u32,
      kappa,
      delta_t
   ).await.unwrap();
   let bufferoutput: SquareGrid =
      initialconditions
         .newbytemplate(bufferoutput);

   PngConfig::default().writeDataAtPath(
      &bufferoutput.outasheatmap(100.,400.),
      bufferoutput.length() as u32,
      Path::new("image2.png")
   );
}
