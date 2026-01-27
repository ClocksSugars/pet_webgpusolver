use std::future::pending;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use std::cell::{RefCell};

use tokio::sync::watch::Ref;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::{HtmlCanvasElement};

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

thread_local! {
   pub static INTERNAL_MESSAGE : RefCell<
      Option<
         tokio::sync::oneshot::Receiver<
            Result<(), wgpu::BufferAsyncError>
         >
      >
   > = RefCell::new(None);
}

thread_local! {
   pub static CSV_BUFFER : RefCell<Option<(Vec<f32>, u32, u32)>> = RefCell::new(None);
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

   // log::info!("received values, n_times looks like {:?}", bytemuck::cast_slice::<u32,u8>(&[n_times]));
   // log::info!("and n_times is {:?}", &n_times);
   // log::info!("received values, kappa looks like {:?}", bytemuck::cast_slice::<f32,u8>(&[kappa]));
   // log::info!("received values, delta_t looks like {:?}", bytemuck::cast_slice::<f32,u8>(&[delta_t]));
   // log::info!("received values, min_T looks like {:?}", bytemuck::cast_slice::<f32,u8>(&[minT]));
   // log::info!("received values, max_T looks like {:?}", bytemuck::cast_slice::<f32,u8>(&[maxT]));

   state.heateq.update_values(&state.queue, n_times, kappa, delta_t, minT, maxT);

   let result = state.device.poll(wgpu::PollType::wait_indefinitely()).map_err( |e|
      {
         log::info!("failed to update values with {}", e);
         JsValue::from_str(&format!("failed to update values with {}", e))
      }
   );

   THE_STATE.set(WebApp::Idle(state));

   result?;
   log::info!("write succeeded");

   Ok(())
}

#[wasm_bindgen]
pub fn run_a_compute_iter() -> Result<(), JsValue> {

   let globalstate = THE_STATE.replace(WebApp::Uninitialized);

   let mut state: WgpuState = match globalstate {
      WebApp::Uninitialized => {
         log::info!("Can not do! Uninitialized");
         return Err(JsValue::from_str("Can not do! Uninitialized"));
      }
      WebApp::Idle(state) => state
   };

   let mut pending_queue = state.pending_queue.replace(vec![]);

   state.heateq.send_compute_job(&mut pending_queue,&state.device);
   state.heateq.send_color_job(&mut pending_queue,&state.device);
   state.heateq.color_to_texture(&mut pending_queue,&state.device,&state.texture_buffer);

   _ = state.pending_queue.replace(pending_queue);

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

#[wasm_bindgen]
pub fn send_output_to_export() -> Result<(), JsValue> {
   let globalstate = THE_STATE.replace(WebApp::Uninitialized);
   let mut state: WgpuState = match globalstate {
      WebApp::Uninitialized => {
         log::info!("Can not do! Uninitialized");
         return Err(JsValue::from_str("Can not do! Uninitialized"));
      }
      WebApp::Idle(state) => state
   };

   let mut pending_queue = state.pending_queue.replace(vec![]);
   let mut encoder = state.device.create_command_encoder(&Default::default());

   state.heateq.export_buffer.unmap();
   let (sender,receiver) = tokio::sync::oneshot::channel();
   encoder.copy_buffer_to_buffer(&state.heateq.output_buffer, 0, &state.heateq.export_buffer, 0, state.heateq.output_buffer.size());
   encoder.map_buffer_on_submit(&state.heateq.export_buffer, wgpu::MapMode::Read, ..,
      move |result| {
         match sender.send(result) {
            Ok(()) => {}
            Err(x) => log::info!("sender failed to send, with message {:?}",x)
         }
      });
   pending_queue.push(encoder.finish());
   INTERNAL_MESSAGE.set(Some(receiver));

   _ = state.pending_queue.replace(pending_queue);
   THE_STATE.set(WebApp::Idle(state));
   Ok(())
}

// #[wasm_bindgen]
// pub fn setup_temp_receiver() -> Result<(), JsValue> {
//    let globalstate = THE_STATE.replace(WebApp::Uninitialized);
//    let mut state: WgpuState = match globalstate {
//       WebApp::Uninitialized => {
//          log::info!("Can not do! Uninitialized");
//          return Err(JsValue::from_str("Can not do! Uninitialized"));
//       }
//       WebApp::Idle(state) => state
//    };

//    let (sender,receiver) = tokio::sync::oneshot::channel();

//    state.heateq.export_buffer.map_async(wgpu::MapMode::Read, ..,
//       move |result| sender.send(result).unwrap());

//    state.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

//    THE_STATE.set(WebApp::Idle(state));
//    INTERNAL_MESSAGE.set(Some(receiver));
//    Ok(())
// }

#[wasm_bindgen]
pub fn is_receiver_ready() -> Result<bool, JsValue> {
   let Some(mut receiver) = INTERNAL_MESSAGE.replace(None) else {return Ok(false)};
   let is_val = receiver.try_recv();
   match is_val {
      Ok(Ok(())) => {return Ok(true);}
      Ok(Err(_)) => {log::info!("gpu export cooked")}
      _ => {INTERNAL_MESSAGE.replace(Some(receiver));}
   };

   Ok(false)
}

#[wasm_bindgen]
pub fn get_export_to_num() -> Result<f32, JsValue> {
   let globalstate = THE_STATE.replace(WebApp::Uninitialized);
   let mut state: WgpuState = match globalstate {
      WebApp::Uninitialized => {
         log::info!("Can not do! Uninitialized");
         return Err(JsValue::from_str("Can not do! Uninitialized"));
      }
      WebApp::Idle(state) => state
   };

   let thedata: Vec<f32> = {
      let output_data = state.heateq.export_buffer.get_mapped_range(..);
      bytemuck::cast_slice(&output_data).to_vec()
   };

   //let length = state.heateq.length.clone();

   THE_STATE.set(WebApp::Idle(state));

   let mut the_sum: f64 = 0.0;
   // for i in 0..(length as usize) {
   //    the_sum += thedata[i] as f64;
   // }
   for i in thedata.iter() {the_sum += *i as f64}
   return Ok(the_sum as f32)
}

// we need this separate so that rust knows to add instructions to drop the gpu memory
#[wasm_bindgen]
pub fn junk_current_state() -> Result<(), wasm_bindgen::JsValue> {
   _ = THE_STATE.replace(WebApp::Uninitialized);
   _ = INTERNAL_MESSAGE.replace(None);

   Ok(())
}

#[wasm_bindgen]
pub async fn rinit_with_xy(width: u32, height: u32) -> Result<(), wasm_bindgen::JsValue> {
    let window = wgpu::web_sys::window().unwrap_throw();
    let document = window.document().unwrap_throw();
    //let canvas = body.get_element_by_id(CANVAS_ID).unwrap_throw();
    let canvas: web_sys::Element = document
       .query_selector("canvas")
       .expect("could not find canvas")
       .expect("canvas query returned empty");
    let html_canvas_element: HtmlCanvasElement = canvas
       .dyn_into()
       .expect("man your canvas is bonked or somethin");

    let mut webstate = WgpuState::new_with(html_canvas_element, width, height)
        .await
        .map_err(|e| JsValue::from_str(&format!("error {}",e)))?;

    webstate.render().map_err(|e| JsValue::from_str(&format!("error {}",e)));

    console_error_panic_hook::set_once();
    THE_STATE.set(WebApp::Idle(webstate));

    Ok(())
}

#[wasm_bindgen]
pub fn give_current_width() -> Result<u32, JsValue> {
   let globalstate = THE_STATE.replace(WebApp::Uninitialized);
   let state: WgpuState = match globalstate {
      WebApp::Uninitialized => {
         log::info!("Can not do! Uninitialized");
         return Err(JsValue::from_str("Can not do! Uninitialized"));
      }
      WebApp::Idle(state) => state
   };

   let answer: u32 = state.heateq.width.clone();
   THE_STATE.replace(WebApp::Idle(state));
   Ok(answer)
}

#[wasm_bindgen]
pub fn give_current_height() -> Result<u32, JsValue> {
   let globalstate = THE_STATE.replace(WebApp::Uninitialized);
   let state: WgpuState = match globalstate {
      WebApp::Uninitialized => {
         log::info!("Can not do! Uninitialized");
         return Err(JsValue::from_str("Can not do! Uninitialized"));
      }
      WebApp::Idle(state) => state
   };

   let answer: u32 = state.heateq.height.clone();
   THE_STATE.replace(WebApp::Idle(state));
   Ok(answer)
}

#[wasm_bindgen]
pub fn parse_csv(csv_as_string: String) -> Result<String, JsValue> {
   let mut csvrdr = csv::ReaderBuilder::new()
      .delimiter(b',')
      .has_headers(false)
      .from_reader(csv_as_string.as_bytes());

   let mut width: u32 = 0;
   let mut height: u32 = 0;
   let mut newbuffer: Vec<f32> = Vec::new();

   if let Some(result) = csvrdr.records().next() {
      let first_line = match result {
         Ok(is_ok) => is_ok,
         Err(_) => {return Ok(String::from_str("failed to read first line of csv").unwrap());}
      };
      for i in first_line.iter() {
         match i.parse::<f32>() {
            Ok(num) => { newbuffer.push(num); width += 1; }
            Err(_) => {return Ok(String::from_str(&format!(
               "could not read csv entry (0,{}) as float32",
               width)).unwrap());}
         }
      }
      height += 1;
   } else {
      return Ok(String::from_str("couldnt find first row").unwrap());
   }

   loop {
      if let Some(result) = csvrdr.records().next() {
         let next_line = match result {
            Ok(is_ok) => is_ok,
            Err(_) => {return Ok(String::from_str(&format!(
               "failed to read {}'th line of csv, was it longer or shorter than other lines?",
               height + 1)).unwrap());}
         };
         let mut x_coord = 0;
         for i in next_line.iter() {
            match i.parse::<f32>() {
               Ok(num) => {newbuffer.push(num); x_coord += 1;}
               Err(_) => {return Ok(String::from_str(&format!(
                  "failed to read element ({},{}) of csv as float32?",
                  x_coord, height)).unwrap());
               }
            };
         }
         if !(x_coord == width) {return Ok(String::from_str(&format!(
            "{}'th line of csv was {} long instead of {}", height, x_coord, width)).unwrap());}
         height += 1;
      } else {
         break;
      }
   }

   if !(newbuffer.len() == (width * height) as usize) {
      return Ok(String::from_str(&format!(
         "csv failed data-length is width times height test (width {} and height {})", width, height)).unwrap());
   }

   _ = CSV_BUFFER.replace(Some((newbuffer, width, height)));

   Ok(String::from_str("success!").unwrap())
}

#[wasm_bindgen]
pub async fn init_from_csv_buffer() -> Result<(), JsValue> {
   let Some((newbuffer, width, height)) = CSV_BUFFER.replace(None) else {
      return Err(JsValue::from_str("tried to load from csv buffer but it was empty"));
   };

   let (sender, receiver) = tokio::sync::oneshot::channel::<Result<(), wgpu::BufferAsyncError>>();
   _ = INTERNAL_MESSAGE.replace(Some(receiver));

   let window = wgpu::web_sys::window().unwrap_throw();
   let document = window.document().unwrap_throw();
   let canvas: web_sys::Element = document
      .query_selector("canvas")
      .expect("could not find canvas")
      .expect("canvas query returned empty");
   let html_canvas_element: HtmlCanvasElement = canvas
      .dyn_into()
      .expect("man your canvas is bonked or somethin");
   let mut state = WgpuState::new_with(html_canvas_element, width, height)
      .await
      .map_err(|e| JsValue::from_str(&format!("error {}",e)))?;

   state.queue.write_buffer(
      &state.heateq.data_buffer,
      0,
      bytemuck::cast_slice(newbuffer.as_slice())
   );

   let mut pending_queue: Vec<wgpu::CommandBuffer> = Vec::new();
   state.heateq.send_color_job(&mut pending_queue, &state.device);
   state.heateq.color_to_texture(&mut pending_queue, &state.device, &state.texture_buffer);
   _ = state.pending_queue.replace(pending_queue);
   state.render();

   THE_STATE.replace(WebApp::Idle(state));

   sender.send(Ok(()));

   Ok(())
}

#[wasm_bindgen]
pub async fn get_total_energy_in_one() -> Result<f32, JsValue> {
   if let Some(receiver) = INTERNAL_MESSAGE.replace(None) {
      let receiver_result = receiver.await;
      match (receiver_result) {
         Ok(Ok(())) => {},
         Err(err) => {return Err(JsValue::from_str(
            &format!("error at receiver in get_total_energy_in_one: {:?}", err)
         ));},
         _ => {return Err(JsValue::from_str("wgpu export failure"));}
      };
   } else {log::info!("no receiver to block on in get_total_energy_in_one, continuing")};

   let globalstate = THE_STATE.replace(WebApp::Uninitialized);
   let state: WgpuState = match globalstate {
      WebApp::Uninitialized => {
         log::info!("Can not do! Uninitialized");
         return Err(JsValue::from_str("Can not do! Uninitialized"));
      }
      WebApp::Idle(state) => state
   };

   let mut pending_queue = state.pending_queue.replace(Vec::new());
   let mut encoder = state.device.create_command_encoder(&Default::default());

   state.heateq.export_buffer.unmap();
   let (sender,receiver) = tokio::sync::oneshot::channel();
   encoder.copy_buffer_to_buffer(
      &state.heateq.data_buffer,
      0,
      &state.heateq.export_buffer,
      0,
      state.heateq.data_buffer.size());
   encoder.map_buffer_on_submit(&state.heateq.export_buffer, wgpu::MapMode::Read, ..,
      move |result| {
         match sender.send(result) {
            Ok(()) => {}
            Err(x) => log::info!("sender failed to send, with message {:?}",x)
         }
      });
   pending_queue.push(encoder.finish());
   state.queue.submit(pending_queue.into_boxed_slice());

   let thedata: Vec<f32> = {
      match state.device.poll(wgpu::PollType::wait_indefinitely()) {
         Err(_) => {return Err(JsValue::from_str(&format!("poll failed in get_total_energy_in_one"))); }
         Ok(_) => {}
      };
      _ = match receiver.await {
         Err(_) => {return Err(JsValue::from_str(&format!("receiver failed in get_total_energy_in_one"))); }
         Ok(_) => ()
      };
      let output_data = state.heateq.export_buffer.get_mapped_range(..);
      bytemuck::cast_slice(&output_data).to_vec()
   };

   let mut the_sum: f64 = 0.0;
   for i in thedata.iter() {the_sum += *i as f64}

   _ = THE_STATE.replace(WebApp::Idle(state));

   Ok(the_sum as f32)
}

#[wasm_bindgen]
pub async fn writeStateAsCSV() -> Result<String,JsValue> {
   if let Some(receiver) = INTERNAL_MESSAGE.replace(None) {
      let receiver_result = receiver.await;
      match (receiver_result) {
         Ok(Ok(())) => {},
         Err(err) => {return Err(JsValue::from_str(
            &format!("error at receiver in get_total_energy_in_one: {:?}", err)
         ));},
         _ => {return Err(JsValue::from_str("wgpu export failure"));}
      };
   } else {log::info!("no receiver to block on in get_total_energy_in_one, continuing")};

   let globalstate = THE_STATE.replace(WebApp::Uninitialized);
   let state: WgpuState = match globalstate {
      WebApp::Uninitialized => {
         log::info!("Can not do! Uninitialized");
         return Err(JsValue::from_str("Can not do! Uninitialized"));
      }
      WebApp::Idle(state) => state
   };

   let mut pending_queue = state.pending_queue.replace(Vec::new());
   let mut encoder = state.device.create_command_encoder(&Default::default());

   state.heateq.export_buffer.unmap();
   let (sender,receiver) = tokio::sync::oneshot::channel();
   encoder.copy_buffer_to_buffer(
      &state.heateq.data_buffer,
      0,
      &state.heateq.export_buffer,
      0,
      state.heateq.data_buffer.size());
   encoder.map_buffer_on_submit(&state.heateq.export_buffer, wgpu::MapMode::Read, ..,
      move |result| {
         match sender.send(result) {
            Ok(()) => {}
            Err(x) => log::info!("sender failed to send, with message {:?}",x)
         }
      });
   pending_queue.push(encoder.finish());
   state.queue.submit(pending_queue.into_boxed_slice());

   let mut thedata: Vec<f32> = {
      match state.device.poll(wgpu::PollType::wait_indefinitely()) {
         Err(_) => {return Err(JsValue::from_str(&format!("poll failed in get_total_energy_in_one"))); }
         Ok(_) => {}
      };
      _ = match receiver.await {
         Err(_) => {return Err(JsValue::from_str(&format!("receiver failed in get_total_energy_in_one"))); }
         Ok(_) => ()
      };
      let output_data = state.heateq.export_buffer.get_mapped_range(..);
      bytemuck::cast_slice(&output_data).to_vec()
   };
   let width = state.heateq.width;
   let height = state.heateq.height;
   _ = THE_STATE.replace(WebApp::Idle(state));

   let mut thewriter = csv::WriterBuilder::new()
      .delimiter(b',')
      .has_headers(false)
      .from_writer(vec![]);
   for j in 0..height {
      thewriter.write_record(
         thedata.drain(0..(width as usize)).map(|f| format!("{}",f))
      ).map_err(|e| {JsValue::from_str(&format!("writer failed on line {}", j))})?
   }
   let innerwriter = thewriter.into_inner();
   let finalstr = match innerwriter {
      Ok(thevec) => String::from_utf8(thevec).map_err(|e| JsValue::from_str("Could not convert bytestring to utf8 string")),
      _ => {return Err(JsValue::from_str("Could not convert csv-writer into bytestring"));}
   };

   finalstr
}
