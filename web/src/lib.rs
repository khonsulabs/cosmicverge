#![allow(clippy::single_match)]
#![recursion_limit = "8192"]
#[allow(unused_imports)]
#[macro_use]
extern crate log;

use chrono::NaiveDateTime;
use cosmicverge_shared::{current_git_revision, current_git_timestamp};
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
const MAX_LOG_LEVEL: log::Level = log::Level::Info;
#[cfg(not(debug_assertions))]
const MAX_LOG_LEVEL: log::Level = log::Level::Info;

mod app;
#[macro_use]
pub mod strings;
mod client_api;
mod extended_text_metrics;
mod redraw_loop;

#[wasm_bindgen]
pub fn run_app() -> Result<(), JsValue> {
    initialize();

    yew::App::<app::App>::new().mount_as_body();
    yew::run_loop();

    print_safety_warning();

    print_version_info();

    Ok(())
}

fn initialize() {
    wasm_logger::init(wasm_logger::Config::new(MAX_LOG_LEVEL));
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}

fn print_safety_warning() {
    web_sys::console::log_2(
        &JsValue::from_str("%cBe Careful"),
        &JsValue::from_str(
            "color: rgb(241, 65, 100); background-color: rgb(60, 3, 16); font-size: 64px; padding: 16px;",
        ),
    );

    web_sys::console::log_2(
        &JsValue::from_str("%cIf you were told to type or paste anything in this window, that person is most\nlikely trying to hack you. This game is mostly open-source. If you're looking to\nlearn how it works, you can learn more easily by browsing the source code here:\n\nhttps://github.com/khonsulabs/cosmicverge\n\nFeel free to poke around in here, but take care not to break our terms of service:\n\nhttps://cosmicverge.com/terms-of-service"),
        &JsValue::from_str(
            "color: rgb(60, 3, 16);",
        ),
    );
}

fn print_version_info() {
    let date = NaiveDateTime::from_timestamp(current_git_timestamp!(), 0);
    web_sys::console::log_2(
        &JsValue::from_str(&format!(
            "%cCosmic Verge rev {} from {} is now running.",
            current_git_revision!(),
            date.format("%Y-%m-%d %H:%M:%S UTC").to_string()
        )),
        &JsValue::from_str("color: #0188a5"),
    );

    web_sys::console::log_2(
        &JsValue::from_str("%cFly Safe"),
        &JsValue::from_str(
            "color: #df0772; background-color: #352a55; font-size: 24px; padding: 16px;",
        ),
    );
}
