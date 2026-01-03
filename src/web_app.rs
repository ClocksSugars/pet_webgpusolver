use std::sync::{Mutex, OnceLock};
use std::cell::{RefCell};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

use crate::wgpuworkhorse::*;

// The idea here is to use the replace
pub enum WebApp {
   Uninitialized,
   Idle(WgpuState),
   // something like the following will allow a queue to form
   //Busy(Vec<Box<dyn Future<Output = wgpuworkhorse::WgpuState>>>),
}

thread_local! {
   pub static THE_STATE : RefCell<WebApp> = RefCell::new(WebApp::Uninitialized);
}

// Now expose all wgpu heat equation and rendering functionality to javascript
#[wasm_bindgen]
pub fn update_values(
   n_times: u32,
   kappa: f32,
   delta_t: f32,
   #[allow(non_snake_case)] minT: f32,
   #[allow(non_snake_case)] maxT: f32
) -> Result<(), JsValue> {

   let globalstate = THE_STATE.replace(WebApp::Uninitialized);

   let mut state: WgpuState = match globalstate {
      WebApp::Uninitialized => {
         log::info!("Can not do! Uninitialized");
         return Err(JsValue::from_str("Can not do! Uninitialized"));
      }
      WebApp::Idle(state) => state
   };

   state.heateq.update_values(&state.queue, n_times, kappa, delta_t, minT, maxT);

   THE_STATE.set(WebApp::Idle(state));

   Ok(())
}

#[wasm_bindgen]
pub fn run_a_compute_loop() -> Result<(), JsValue> {

   let globalstate = THE_STATE.replace(WebApp::Uninitialized);

   let mut state: WgpuState = match globalstate {
      WebApp::Uninitialized => {
         log::info!("Can not do! Uninitialized");
         return Err(JsValue::from_str("Can not do! Uninitialized"));
      }
      WebApp::Idle(state) => state
   };

   state.heateq.send_compute_job(&state.queue,&state.device);
   state.heateq.send_color_job(&state.queue,&state.device);
   state.heateq.color_to_texture(&state.queue,&state.device,&state.texture_buffer);

   // let result = state.device.poll(wgpu::PollType::wait_indefinitely())
   //    .map_err(|e| JsValue::from_str(&format!("gpu failure: {}",e)));

   THE_STATE.set(WebApp::Idle(state));

   //result?;

   Ok(())
}

#[wasm_bindgen]
pub fn render_a_frame() -> Result<(), JsValue> {

   let globalstate = THE_STATE.replace(WebApp::Uninitialized);

   let mut state: WgpuState = match globalstate {
      WebApp::Uninitialized => {
         log::info!("Can not do! Uninitialized");
         return Err(JsValue::from_str("Can not do! Uninitialized"));
      }
      WebApp::Idle(state) => state
   };

   let result = state.render().map_err(|e| {
      log::info!("gpu failure: {}",e);
      JsValue::from_str(&format!("gpu failure: {}",e))
   });

   THE_STATE.set(WebApp::Idle(state));

   result?;

   Ok(())
}

// implement this https://donatstudios.com/Read-User-Files-With-Go-WASM
