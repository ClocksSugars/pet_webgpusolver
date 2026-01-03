use std::fmt::Display;
use std::sync::Arc;
use winit::window;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window
};
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
   window: Arc<Window>,
}

impl State {
   pub async fn new(valid_pre_surface: Arc<Window>) -> anyhow::Result<Self>{
      let pony = wgpuworkhorse::WgpuState::new(valid_pre_surface.clone()).await?;
      Ok(Self {
         wgpuworkhorse: pony,
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

   fn handle_key(&mut self, event_loop: &ActiveEventLoop, key: KeyCode, pressed: bool) {
       match (key, pressed) {
           (KeyCode::Escape, true) => event_loop.exit(),
           _ => {}
       }
   }

   fn update(&mut self) {}

   pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
      self.window.request_redraw();

      self.wgpuworkhorse.render()?;

      Ok(())
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
      gen_print("state creation finished");

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
                       state: key_state,
                       ..
                   },
               ..
           } => state.handle_key(event_loop, code, key_state.is_pressed()),
           _ => {}
       }
   }
}
