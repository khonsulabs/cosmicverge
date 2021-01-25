
use glow::*;
use wasm_bindgen::prelude::*;
use crate::{frame_counter, initialize_shared_helpers, space_bridge};
use crossbeam::channel::Receiver;
use crate::space_bridge::BridgeCommand;
use wasm_bindgen::JsCast;
use wasm_bindgen::__rt::core::time::Duration;
use web_sys::Performance;

struct SpaceView {
    performance: Performance,
    last_frame_time: Option<f64>,
    gl: Context,
    command_receiver: Receiver<space_bridge::BridgeCommand>,
    should_render: bool,
}

impl SpaceView {
    fn new(command_receiver: Receiver<space_bridge::BridgeCommand>) -> Self {
        let gl = unsafe {
            let (_window, gl, _events_loop, _render_loop, shader_version) = {
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
            gl
        };
        Self {
            performance: web_sys::window().unwrap().performance().unwrap(),
            command_receiver,
            gl,
            should_render: true,
            last_frame_time: None,
        }
    }

    fn run(mut self) {
        self.request_animation_frame();
    }

    fn receive_commands(&mut self) {
        while let Ok(command) = self.command_receiver.try_recv() {
            self.handle_command(command);
        }
    }

    fn receive_at_least_one_command(&mut self) {
        if let Ok(command) = self.command_receiver.recv() {
            self.handle_command(command);
        }
    }

    fn handle_command(&mut self, command: space_bridge::BridgeCommand) {
        match command {
            BridgeCommand::PauseRendering => {
                self.should_render = false;
            }
            BridgeCommand::ResumeRendering => {
                self.should_render = true;
            }
        }
    }

    fn request_animation_frame(mut self) {
        let closure = Closure::once_into_js(move || {
            self.next_frame()
        });
        web_sys::window()
            .unwrap()
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .unwrap();
    }

    fn sleep_before_frame(mut self, sleep_duration: Duration) {
        info!("Fast machine! Sleeping {}ms", sleep_duration.as_millis());
        let closure = Closure::once_into_js(move || {
            self.request_animation_frame()
        });
        web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), sleep_duration.as_millis() as i32).unwrap();
    }

    fn wait_for_resume(mut self) {
        let closure = Closure::once_into_js(move || {
            self.receive_commands();
            if self.should_render {
                self.request_animation_frame()
            } else {
                self.wait_for_resume()
            }
        });
        web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), 150).unwrap();
    }

    fn next_frame(mut self) {
        let frame_start = self.performance.now();
        self.receive_commands();

        if !self.should_render {
            return self.wait_for_resume()
        }

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

        info!("Rendering!");
        unsafe {
            if changed {
                self.gl.viewport(0, 0, canvas.client_width(), canvas.client_height());
            }

            self.gl.clear(glow::COLOR_BUFFER_BIT);
            self.gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }

        let now = self.performance.now();
        let frame_duration = Duration::from_millis((now - self.last_frame_time.unwrap_or(frame_start)) as u64);
        self.last_frame_time = Some(frame_start);

        let target_duration = Duration::from_secs_f64(1.0 / 60.0);
        match target_duration.checked_sub(frame_duration) {
            Some(sleep_amount) => if sleep_amount.as_millis() < 3 {
                // Only sleep if we know we have a few ms to spare
                return self.sleep_before_frame(sleep_amount)
            },
            None => {}
        }
        self.request_animation_frame()
    }
}

#[wasm_bindgen]
pub fn glcanvas() {
    initialize_shared_helpers();
    let command_receiver = space_bridge::command_receiver();
    SpaceView::new(command_receiver).run()
}
