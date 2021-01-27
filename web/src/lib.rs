#![recursion_limit = "8192"]
#[allow(unused_imports)]
#[macro_use]
extern crate log;

use std::sync::{Arc, Mutex};

use once_cell::sync::OnceCell;
use wasm_bindgen::prelude::*;

#[macro_use]
mod internal_macros {
    #[allow(dead_code)]
    #[macro_export]
    macro_rules! todo {
        () => { error!("not yet implemented {}:{}", file!(), line!()) };
        ($($arg:tt)+) => { error!( "not yet implemented {}:{}: {}", file!(), line!(), std::format_args!($($arg)+))};
    }
}

#[cfg(debug_assertions)]
const MAX_LOG_LEVEL: log::Level = log::Level::Trace;
#[cfg(not(debug_assertions))]
const MAX_LOG_LEVEL: log::Level = log::Level::Info;

mod app;
#[macro_use]
pub mod strings;
mod redraw_loop;

static FRAME_COUNTER: OnceCell<Arc<Mutex<bool>>> = OnceCell::new();

pub fn frame_counter() -> &'static Arc<Mutex<bool>> {
    FRAME_COUNTER.get_or_init(|| Arc::new(Mutex::new(true)))
}

static APP_INITIALIZED: OnceCell<()> = OnceCell::new();
fn initialize_shared_helpers() {
    APP_INITIALIZED.get_or_init(|| {
        wasm_logger::init(wasm_logger::Config::new(MAX_LOG_LEVEL));
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    });
}

#[wasm_bindgen]
pub fn run_app() -> Result<(), JsValue> {
    initialize_shared_helpers();
    yew::App::<app::App>::new().mount_to_body();
    yew::run_loop();

    Ok(())
}
