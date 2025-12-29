use std::primitive::f64;

pub fn sqnum<T>(x:T) -> T where T: std::ops::Mul<Output = T> + Copy{
   x * x
}

pub fn makemiddleRatTinitconds(r: f32, t:f32 ) -> impl Fn(f32,f32) -> f32 {
   let rsquared = sqnum(r);
   move |x:f32,y:f32| -> f32 {
      match (sqnum(x-0.5) + sqnum(y-0.5)) < rsquared {
         true => t,
         false => 0.
      }
   }
}

pub fn makegaussianinitconds(t:f32 ) -> impl Fn(f32,f32) -> f32 {
   move |x:f32,y:f32| -> f32 {
      t * ((-10. * (sqnum(x-0.5) + sqnum(y-0.5))) as f64).exp() as f32
   }
}
