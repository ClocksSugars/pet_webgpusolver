mod mathutils;
mod rectgrid;
mod aspng;
mod webgpuheat;
mod wgpuworkhorse;

#[cfg(target_arch = "wasm32")]
mod web_app;

#[cfg(not(target_arch = "wasm32"))]
mod desktop_app;

use crate::aspng::*;
use crate::mathutils::*;
use crate::rectgrid::*;
use crate::webgpuheat::*;

use std::fmt::Display;
#[cfg(target_arch = "wasm32")]
use std::process::Output;


#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;
use winit::window;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window
};
use wgpu::util::{DeviceExt};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use console_log::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowAttributesExtWebSys;


// #[cfg(target_arch = "wasm32")]
// static mut THE_STATE : OnceLock<wgpuworkhorse::WgpuState> = OnceLock::new();


fn gen_print<T>(s: T) where T: Display {
   #[cfg(target_arch = "wasm32")]
   log::info!("{}",s);
   #[cfg(not(target_arch = "wasm32"))]
   println!("{}",s)
}

#[cfg(target_arch = "wasm32")]
pub async fn junkresult(thing: HtmlCanvasElement) -> () {
   _ = wgpuworkhorse::WgpuState::new(thing).await;
   ()
}


pub fn run_desktop() -> Option<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        let event_loop = EventLoop::with_user_event().build().ok()?;
        let mut app = crate::desktop_app::App::new(
            #[cfg(target_arch = "wasm32")]
            &event_loop,
        );
        event_loop.run_app(&mut app).ok()?;
    }

    Some(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn run_web() -> Result<(), wasm_bindgen::JsValue> {

    console_log::init_with_level(log::Level::Info).unwrap_throw();
    log::info!{"initialized log"};

    let (sender, receiver) = tokio::sync::oneshot::channel::<Result<(), wgpu::BufferAsyncError>>();
    _ = web_app::INTERNAL_MESSAGE.replace(Some(receiver));

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

    let mut webstate = wgpuworkhorse::WgpuState::new(html_canvas_element)
        .await
        .map_err(|e| JsValue::from_str(&format!("error {}",e)))?;

    webstate.render().map_err(|e| JsValue::from_str(&format!("error {}",e)));

    console_error_panic_hook::set_once();
    web_app::THE_STATE.set(web_app::WebApp::Idle(webstate));

    sender.send(Ok(()));

    Ok(())
}
