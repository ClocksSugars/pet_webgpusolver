use std::fmt::Display;
use std::sync::Arc;
use winit::window;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, KeyCode, PhysicalKey},
    window::Window
};
use tokio::sync::*;
use wgpu::util::{DeviceExt};
use crate::wgpuworkhorse;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use console_log::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowAttributesExtWebSys;

fn gen_print<T>(s: T) where T: Display {
   #[cfg(target_arch = "wasm32")]
   log::info!("{}",s);
   #[cfg(not(target_arch = "wasm32"))]
   println!("{}",s)
}

pub struct State {
   wgpuworkhorse: wgpuworkhorse::WgpuState,
   cli_state: std::string::String,
   compute_on_render: bool,
   //end_cli_sender: oneshot::Sender<()>,
   //cli_receiver: mpsc::Receiver<Vec<char>>,
   window: Arc<Window>,
}

impl State {
   pub async fn new(valid_pre_surface: Arc<Window>) -> anyhow::Result<Self>{
      let pony = wgpuworkhorse::WgpuState::new(valid_pre_surface.clone()).await?;

      // end_cli_receiver: oneshot::Receiver<()>,
      // cli_sender: mpsc::Sender<Vec<char>>

      print!("Input: ");

      Ok(Self {
         wgpuworkhorse: pony,
         cli_state: std::string::String::new(),
         compute_on_render: false,
         window: valid_pre_surface
      })
   }

   pub fn resize(&mut self, width: u32, height: u32) {
       if width > 0 && height > 0 {
           self.wgpuworkhorse.config.width = width;//256;
           self.wgpuworkhorse.config.height = height;//256;
           self.wgpuworkhorse.surface.configure(
              &self.wgpuworkhorse.device,
              &self.wgpuworkhorse.config
           );
           self.wgpuworkhorse.is_surface_configured = true;
       }
   }

   fn handle_key(&mut self, event_loop: &ActiveEventLoop, key: Result<Key,KeyCode>, pressed: bool) {
       match (key, pressed) {
           (Err(KeyCode::Escape), true) => event_loop.exit(),
           (Err(KeyCode::Backspace), true) => {
               self.cli_state = std::string::String::new();
               },
           (Err(KeyCode::Enter), true) => {
              self.do_instruction();
              self.cli_state = std::string::String::new();
           },
           (Err(KeyCode::Space), true) => {
              self.cli_state.push(' ');
              println!("Input: {}",self.cli_state);
           },
           (Ok(Key::Character(presumably_letter)), true) => {
              let temp: String = presumably_letter.chars().collect();
              self.cli_state.push_str(&temp);
              //println!("{}",temp);
              println!("Input: {}",self.cli_state);
           }
           _ => {}
       }
   }

   fn update(&mut self) {}

   pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
      self.window.request_redraw();

      let mut pending_queue = self.wgpuworkhorse.pending_queue.replace(vec![]);

      if self.compute_on_render {
         self.wgpuworkhorse.heateq.send_compute_job(&mut pending_queue,&self.wgpuworkhorse.device);
         self.wgpuworkhorse.heateq.send_color_job(&mut pending_queue,&self.wgpuworkhorse.device);
         self.wgpuworkhorse.heateq.color_to_texture(&mut pending_queue,&self.wgpuworkhorse.device,&self.wgpuworkhorse.texture_buffer);
      }

      _ = self.wgpuworkhorse.pending_queue.replace(pending_queue);

      self.wgpuworkhorse.render()?;

      Ok(())
   }

   fn do_instruction(&mut self) {
      let mut instruction = self.cli_state.split(' ');
      println!("");

      let first_word = match instruction.next() {
         Some("start") => {self.compute_on_render = true; return}
         Some("stop") => {self.compute_on_render = false; return}
         Some(x) => x,
         None => {println!("received empty command"); return}
      };

      if (first_word == "set") {
         match (instruction.next(), instruction.next()) {
            (Some("max_T"), Some(x)) => {
                  if let Ok(max_T) = x.parse::<f32>() {
                     self.wgpuworkhorse.queue.write_buffer(
                        &self.wgpuworkhorse.heateq.vis_maxT_buffer, 0, bytemuck::cast_slice(&[max_T]));
                     self.wgpuworkhorse.queue.submit([]);
                  }
               }
            (Some("kappa"), Some(x)) => {
               if let Ok(kappa) = x.parse::<f32>() {
                  self.wgpuworkhorse.queue.write_buffer(
                     &self.wgpuworkhorse.heateq.kappa_buffer, 0, bytemuck::cast_slice(&[kappa]));
                  self.wgpuworkhorse.queue.submit([]);
               }
            }
            (Some("iter_quant"), Some(x)) => {
               if let Ok(iter_quant) = x.parse::<u32>() {
                  self.wgpuworkhorse.heateq.iteration_quantity = iter_quant;
               }
            }
            _ => {return;}
         }
      }
   }
}



pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl App {
   pub fn new()
   -> Self {


      Self {
         state: None,

         #[cfg(target_arch = "wasm32")]
         proxy: proxy,
      }
   }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
      #[allow(unused_mut)]
      let mut window_attributes = Window::default_attributes();

      let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
      let temprt = tokio::runtime::Runtime::new()
         .expect("tokio runtime creation failed");
      self.state = temprt.block_on(State::new(window)).ok();
      //gen_print("state creation finished");

   }

   #[allow(unused_mut)]
   fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
       #[cfg(target_arch = "wasm32")]
       {
           event.window.request_redraw();
           event.resize(
               event.window.inner_size().width,
               event.window.inner_size().height,
           );
       }
       self.state = Some(event);
   }

   fn window_event(
       &mut self,
       event_loop: &ActiveEventLoop,
       _window_id: winit::window::WindowId,
       event: WindowEvent,
   ) {
       let state = match &mut self.state {
           Some(canvas) => canvas,
           None => return,
       };

       match event {
           WindowEvent::CloseRequested => event_loop.exit(),
           WindowEvent::Resized(size) => state.resize(size.width, size.height),
           WindowEvent::RedrawRequested => {
               state.update();
               match state.render() {
                   Ok(_) => {}
                   // Reconfigure the surface if it's lost or outdated
                   Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                       let size = state.window.inner_size();
                       state.resize(size.width, size.height);
                   }
                   Err(e) => {
                       log::error!("Unable to render {}", e);
                   }
               }
           }
           WindowEvent::MouseInput { state, button, .. } => match (button, state.is_pressed()) {
               (MouseButton::Left, true) => {}
               (MouseButton::Left, false) => {}
               _ => {}
           },
           WindowEvent::KeyboardInput {
               event:
                   KeyEvent {
                       physical_key: PhysicalKey::Code(code),
                       logical_key: logical_key,
                       state: key_state,
                       ..
                   },
               ..
           } => match code {
              KeyCode::Escape => {state.handle_key(event_loop, Err(code), key_state.is_pressed());}
              KeyCode::Enter =>  {state.handle_key(event_loop, Err(code), key_state.is_pressed());}
              KeyCode::Backspace =>  {state.handle_key(event_loop, Err(code), key_state.is_pressed());}
              KeyCode::Space =>  {state.handle_key(event_loop, Err(code), key_state.is_pressed());}
              _ =>  {state.handle_key(event_loop, Ok(logical_key), key_state.is_pressed());}
           },
           _ => {}
       }
   }
}
