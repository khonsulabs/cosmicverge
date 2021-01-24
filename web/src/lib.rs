#![recursion_limit = "8192"]
use wasm_bindgen::prelude::*;

#[allow(unused_imports)]
#[macro_use]
extern crate log;

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
mod routes;
pub mod strings;

#[wasm_bindgen]
pub fn run_app() -> Result<(), JsValue> {
    wasm_logger::init(wasm_logger::Config::new(MAX_LOG_LEVEL));
    yew::start_app::<app::App>();

    Ok(())
}
