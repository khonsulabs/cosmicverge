use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};
use crossbeam::channel::{Sender, Receiver, TryRecvError, self};

use crate::app::game::check_canvas_size;
use crate::redraw_loop::Drawable;

use glam::f64::DVec2;

pub enum Command {
    /// In pixels
    Pan(DVec2),
    /// In percent relative to current zoom
    Zoom(f64),
}

pub struct SpaceView {
    canvas: Option<HtmlCanvasElement>,
    context: Option<CanvasRenderingContext2d>,
    sun: Option<HtmlImageElement>,
    planet: Option<HtmlImageElement>,
    look_at: DVec2,
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
            planet: None,
            look_at: DVec2::new(0., 0.),
            zoom: 1.,
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
                Command::Pan(amount) => {
                    self.look_at += amount / self.zoom;
                }
            }
        }

        Ok(())
    }
}

impl Drawable for SpaceView {
    fn initialize(&mut self) -> anyhow::Result<()> {
        self.sun = Some({
            let image = HtmlImageElement::new().unwrap();
            image.set_src("/api/magrathea/sun/d4cf31ea-ab1b-44d4-9d3c-a801892bf1af/6371/128");
            image
        });
        self.planet = Some({
            let image = HtmlImageElement::new().unwrap();
            image.set_src("/api/magrathea/world/d4cf31ea-ab1b-44d4-9d3c-a801892bf1af/150200000/0/6371/32");
            image
        });
        Ok(())
    }

    fn render_frame(&mut self) -> anyhow::Result<()> {
        self.receive_commands()?;

        if let Some(canvas) = self.canvas() {
            if let Some(context) = self.context() {
                check_canvas_size(&canvas);

                let size = DVec2::new(canvas.client_width() as f64, canvas.client_height() as f64);
                let center = size / 2. + self.look_at * self.zoom;

                context.set_image_smoothing_enabled(false);
                context.fill_rect(0., 0., size.x, size.y);

                let sun = self.sun.as_ref().expect("No sun");
                if sun.complete() {
                    if let Err(err) = context.draw_image_with_html_image_element_and_dw_and_dh(sun, center.x - 32. * self.zoom, center.y - 32. * self.zoom, 64. * self.zoom,  64. * self.zoom) {
                        error!("Error rendering sun: {:#?}", err);
                    }
                }

                let planet = self.planet.as_ref().expect("No planet");
                if planet.complete() {
                    if let Err(err) = context.draw_image_with_html_image_element_and_dw_and_dh(planet, center.x + (500. - 16.) * self.zoom, center.y - 16. * self.zoom, 32. * self.zoom,  32. * self.zoom) {
                        error!("Error rendering planet: {:#?}", err);
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