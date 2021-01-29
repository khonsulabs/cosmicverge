use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use glam::f64::DVec2;
use wasm_bindgen::{JsCast, __rt::std::collections::HashMap};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

use crate::{app::game::check_canvas_size, redraw_loop::Drawable};

pub enum Command {
    /// In pixels
    Pan(DVec2),
    /// In percent relative to current zoom
    Zoom(f64),
}

pub struct SpaceView {
    canvas: Option<HtmlCanvasElement>,
    context: Option<CanvasRenderingContext2d>,
    backdrop: Option<HtmlImageElement>,
    location_images: HashMap<i64, HtmlImageElement>,
    solar_system: SolarSystem,
    look_at: DVec2,
    zoom: f64,
    receiver: Receiver<Command>,
}

impl SpaceView {
    pub fn new() -> (Self, Sender<Command>) {
        let (sender, receiver) = channel::unbounded();
        (
            Self {
                canvas: None,
                context: None,
                backdrop: None,
                location_images: Default::default(),
                look_at: DVec2::new(0., 0.),
                zoom: 1.,
                receiver,
                solar_system: fake_solar_system(),
            },
            sender,
        )
    }

    fn canvas(&mut self) -> Option<HtmlCanvasElement> {
        if self.canvas.is_none() {
            self.canvas = Some(
                web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("glcanvas")
                    .unwrap()
                    .dyn_into::<web_sys::HtmlCanvasElement>()
                    .ok()?,
            );
        }

        self.canvas.clone()
    }

    fn context(&mut self) -> Option<CanvasRenderingContext2d> {
        if self.context.is_none() {
            self.context = Some({
                let context = self
                    .canvas()?
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<CanvasRenderingContext2d>()
                    .ok()?;
                context
            });
        }

        self.context.clone()
    }

    fn receive_commands(&mut self) -> anyhow::Result<()> {
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
        self.backdrop = Some({
            let image = HtmlImageElement::new().unwrap();
            image.set_src(&self.solar_system.background);
            image
        });

        for location in self.solar_system.locations.iter() {
            let image = HtmlImageElement::new().unwrap();
            image.set_src(&location.image);
            self.location_images.insert(location.id, image);
        }

        Ok(())
    }

    fn render_frame(&mut self) -> anyhow::Result<()> {
        self.receive_commands()?;

        if let Some(canvas) = self.canvas() {
            if let Some(context) = self.context() {
                check_canvas_size(&canvas);

                let size = DVec2::new(canvas.client_width() as f64, canvas.client_height() as f64);
                let canvas_center = size / 2.;
                let center = canvas_center + self.look_at * self.zoom;

                context.set_image_smoothing_enabled(false);

                let backdrop = self.backdrop.as_ref().unwrap();
                context.fill_rect(0., 0., size.x, size.y);
                if backdrop.complete() {
                    // The backdrop is tiled and panned based on the look_at unaffected by zoom
                    let backdrop_center = canvas_center + self.look_at * (self.zoom * 0.1);
                    let size = size.ceil().as_i32();
                    let backdrop_width = backdrop.width() as i32;
                    let backdrop_height = backdrop.height() as i32;
                    let mut y = (backdrop_center.y) as i32 % backdrop_height;
                    if y > 0 {
                        y -= backdrop_height;
                    }
                    while y < size.y {
                        let mut x = (backdrop_center.x) as i32 % backdrop_width;
                        if x > 0 {
                            x -= backdrop_width;
                        }
                        while x < size.x {
                            if let Err(err) = context
                                .draw_image_with_html_image_element(backdrop, x as f64, y as f64)
                            {
                                error!("Error rendering backdrop: {:#?}", err);
                            }
                            x += backdrop_width;
                        }
                        y += backdrop_height;
                    }
                }

                for location in self.solar_system.locations.iter() {
                    let image = &self.location_images[&location.id];
                    if image.complete() {
                        let render_radius = location.size * self.zoom;
                        let render_center = center + location.location * self.zoom;

                        if let Err(err) = context.draw_image_with_html_image_element_and_dw_and_dh(
                            image,
                            render_center.x - render_radius,
                            render_center.y - render_radius,
                            render_radius * 2.,
                            render_radius * 2.,
                        ) {
                            error!("Error rendering sun: {:#?}", err);
                        }
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

pub struct SolarSystem {
    pub name: String,
    pub background: String,
    pub locations: Vec<SolarSystemLocation>,
}

pub struct SolarSystemLocation {
    pub id: i64,
    pub name: String,
    pub image: String,
    pub size: f64,
    pub location: DVec2,
    pub owned_by: Option<i64>,
}

pub fn fake_solar_system() -> SolarSystem {
    SolarSystem {
        name: String::from("SM-0-A9F4"),
        background: String::from("/helianthusgames/Backgrounds/BlueStars.png"),
        locations: vec![
            SolarSystemLocation {
                id: 1,
                name: String::from("Sun"),
                image: String::from("/helianthusgames/Suns/2.png"),
                size: 128.,
                location: DVec2::zero(),
                owned_by: None,
            },
            SolarSystemLocation {
                id: 2,
                name: String::from("Earth"),
                image: String::from("/helianthusgames/Terran_or_Earth-like/1.png"),
                size: 32.,
                location: DVec2::new(600., 0.),
                owned_by: Some(1),
            },
            SolarSystemLocation {
                id: 3,
                name: String::from("Earth"),
                image: String::from("/helianthusgames/Rocky/1.png"),
                size: 24.,
                location: DVec2::new(200., 200.),
                owned_by: Some(1),
            },
        ],
    }
}
