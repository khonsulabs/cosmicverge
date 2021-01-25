#![recursion_limit = "8192"]
#[allow(unused_imports)]
#[macro_use]
extern crate log;

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
mod routes;
pub mod strings;

use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
static FRAME_COUNTER: OnceCell<Arc<Mutex<usize>>> = OnceCell::new();

pub fn frame_counter() -> &'static Arc<Mutex<usize>> {
    FRAME_COUNTER.get_or_init(|| Arc::new(Mutex::new(0)))
}

#[wasm_bindgen]
pub fn run_app() -> Result<(), JsValue> {
    wasm_logger::init(wasm_logger::Config::new(MAX_LOG_LEVEL));
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("app")
        .unwrap();
    yew::App::<app::App>::new().mount(root);
    yew::run_loop();

    Ok(())
}

use glow::*;

#[wasm_bindgen]
pub fn glcanvas() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    unsafe {
        let (_window, gl, _events_loop, render_loop, shader_version) = {
            use wasm_bindgen::JsCast;
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("glcanvas")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();
            let webgl2_context = canvas
                .get_context("webgl2")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::WebGl2RenderingContext>()
                .unwrap();
            (
                (),
                glow::Context::from_webgl2_context(webgl2_context),
                (),
                glow::RenderLoop::from_request_animation_frame(),
                "#version 300 es",
            )
        };

        let vertex_array = gl
            .create_vertex_array()
            .expect("Cannot create vertex array");
        gl.bind_vertex_array(Some(vertex_array));

        let program = gl.create_program().expect("Cannot create program");

        let (vertex_shader_source, fragment_shader_source) = (
            r#"const vec2 verts[3] = vec2[3](
                vec2(0.5f, 1.0f),
                vec2(0.0f, 0.0f),
                vec2(1.0f, 0.0f)
            );
            out vec2 vert;
            void main() {
                vert = verts[gl_VertexID];
                gl_Position = vec4(vert - 0.5, 0.0, 1.0);
            }"#,
            r#"precision mediump float;
            in vec2 vert;
            out vec4 color;
            void main() {
                color = vec4(vert, 0.5, 1.0);
            }"#,
        );

        let shader_sources = [
            (glow::VERTEX_SHADER, vertex_shader_source),
            (glow::FRAGMENT_SHADER, fragment_shader_source),
        ];

        let mut shaders = Vec::with_capacity(shader_sources.len());

        for (shader_type, shader_source) in shader_sources.iter() {
            let shader = gl
                .create_shader(*shader_type)
                .expect("Cannot create shader");
            gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                panic!(gl.get_shader_info_log(shader));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }

        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!(gl.get_program_info_log(program));
        }

        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        gl.use_program(Some(program));
        gl.clear_color(0.1, 0.2, 0.3, 1.0);

        let frame_counter = frame_counter().clone();
        render_loop.run(move |running: &mut bool| {
            {
                let canvas = web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("glcanvas")
                    .unwrap();

                let width_attr = canvas.attributes().get_with_name("width");
                let height_attr = canvas.attributes().get_with_name("height");
                let actual_width: Option<i32> = width_attr.as_ref().map(|w| w.value().parse().ok()).flatten();
                let actual_height: Option<i32> = height_attr.as_ref().map(|h| h.value().parse().ok()).flatten();
                let mut changed = false;
                if actual_width.is_none() || actual_width.unwrap() != canvas.client_width() {
                    changed = true;
                    if let Some(attr) = width_attr {
                        attr.set_value(&canvas.client_width().to_string());
                    } else {
                        canvas.set_attribute("width", &canvas.client_width().to_string());
                    }
                }

                if actual_height.is_none() || actual_height.unwrap() != canvas.client_height() {
                    changed = true;
                    if let Some(attr) = height_attr {
                        attr.set_value(&canvas.client_height().to_string());
                    } else {
                        canvas.set_attribute("height", &canvas.client_height().to_string());
                    }
                }

                if changed {
                    gl.viewport(0, 0, canvas.client_width(), canvas.client_height());
                }
            }

            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            {
                let mut frame_counter = frame_counter.lock().unwrap();
                *frame_counter += 1;
            }

            if !*running {
                gl.delete_program(program);
                gl.delete_vertex_array(vertex_array);
            }
        });
    }
}
