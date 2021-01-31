use glow::*;
use wasm_bindgen::JsCast;

use crate::{app::game::check_canvas_size, redraw_loop::Drawable};

pub struct SpaceView {
    gl: Option<Context>,
}

impl SpaceView {
    pub fn new() -> Self {
        Self { gl: None }
    }

    fn gl(&self) -> &'_ Context {
        self.gl.as_ref().unwrap()
    }
}

impl Drawable for SpaceView {
    fn initialize(&mut self) -> anyhow::Result<()> {
        if self.gl.is_none() {
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

            self.gl = Some(glow::Context::from_webgl2_context(webgl2_context));
        }

        let shader_version = "#version 300 es";
        unsafe {
            let vertex_array = self
                .gl()
                .create_vertex_array()
                .expect("Cannot create vertex array");
            self.gl().bind_vertex_array(Some(vertex_array));

            let program = self.gl().create_program().expect("Cannot create program");

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
                let shader = self
                    .gl()
                    .create_shader(*shader_type)
                    .expect("Cannot create shader");
                self.gl()
                    .shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                self.gl().compile_shader(shader);
                if !self.gl().get_shader_compile_status(shader) {
                    panic!(self.gl().get_shader_info_log(shader));
                }
                self.gl().attach_shader(program, shader);
                shaders.push(shader);
            }

            self.gl().link_program(program);
            if !self.gl().get_program_link_status(program) {
                panic!(self.gl().get_program_info_log(program));
            }

            for shader in shaders {
                self.gl().detach_shader(program, shader);
                self.gl().delete_shader(shader);
            }

            self.gl().use_program(Some(program));
            self.gl().clear_color(0.0, 0.0, 0.0, 1.0);
        }

        Ok(())
    }

    fn render_frame(&mut self) -> anyhow::Result<()> {
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("glcanvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        let changed = check_canvas_size(&canvas);

        unsafe {
            if changed {
                self.gl()
                    .viewport(0, 0, canvas.client_width(), canvas.client_height());
            }

            self.gl().clear(glow::COLOR_BUFFER_BIT);
            self.gl().draw_arrays(glow::TRIANGLES, 0, 3);
        }

        Ok(())
    }

    fn cleanup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
