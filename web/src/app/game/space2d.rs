use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, HtmlElement};
use crossbeam::channel::{Sender, Receiver, TryRecvError, self};

use crate::app::game::check_canvas_size;
use crate::redraw_loop::Drawable;

pub enum Command {
    Zoom(f64),
}

pub struct SpaceView {
    canvas: Option<HtmlCanvasElement>,
    context: Option<CanvasRenderingContext2d>,
    sun: Option<HtmlImageElement>,
    zoom: f64,
    receiver: Receiver<Command>
}

impl SpaceView {
    pub fn new() -> (Self, Sender<Command>) {
        let (sender, receiver) = channel::unbounded();
        (Self {
            canvas: None,
            context: None,
            sun: None,
            zoom: 2.,
            receiver,
        }, sender)
    }

    fn canvas(&mut self) -> Option<HtmlCanvasElement> {
        if self.canvas.is_none() {
            self.canvas = Some(web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("glcanvas")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>().ok()?);
        }

        self.canvas.clone()
    }

    fn context(&mut self) -> Option<CanvasRenderingContext2d> {
        if self.context.is_none() {
            self.context = Some(
                {
                    let context = self.canvas()?
                        .get_context("2d")
                        .unwrap()
                        .unwrap()
                        .dyn_into::<CanvasRenderingContext2d>().ok()?;
                    context
                }
            );
        }

        self.context.clone()
    }

    fn receive_commands(&mut self)-> anyhow::Result<()> {
        while let Some(event) = match self.receiver.try_recv() {
            Ok(command) => Some(command),
            Err(TryRecvError::Empty) => None,
            Err(disconnected) => Err(disconnected)?,
        } {
            match event {
                Command::Zoom(fraction) => {
                    self.zoom += self.zoom * fraction;
                    self.zoom = self.zoom.min(10.).max(0.1);
                }
            }
        }

        Ok(())
    }
}

impl Drawable for SpaceView {
    fn initialize(&mut self) -> anyhow::Result<()> {
        info!("initializing");
        self.sun = Some({
            let image = HtmlImageElement::new().unwrap();
            image.set_src("http://localhost:7879/api/magrathea/sun/d4cf31ea-ab1b-44d4-9d3c-a801892bf1af/6371/64");
            // self.canvas().unwrap().append_child(&image).expect("Error appending child");
            image
        });
        Ok(())
    }

    fn render_frame(&mut self) -> anyhow::Result<()> {
        self.receive_commands()?;

        if let Some(canvas) = self.canvas() {
            if let Some(context) = self.context() {
                check_canvas_size(&canvas);

                let width = canvas.client_width() as f64;
                let height = canvas.client_height() as f64;

                context.set_image_smoothing_enabled(false);
                context.fill_rect(0., 0., width, height);

                let sun = self.sun.as_ref().expect("No sun");
                if sun.complete() {
                    if let Err(err) = context.draw_image_with_html_image_element_and_dw_and_dh(sun, width / 2. - 32. * self.zoom, height / 2. - 32. * self.zoom, 64. * self.zoom,  64. * self.zoom) {
                        error!("Error rendering sun: {:#?}", err);
                    }
                }
            }
        }

        Ok(())
    }

    fn cleanup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}