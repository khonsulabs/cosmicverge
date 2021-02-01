use cosmicverge_shared::{
    euclid::{Point2D, Scale, Size2D, Vector2D},
    solar_systems::{Pixels, Solar, SolarSystem},
};
use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use wasm_bindgen::{JsCast, __rt::std::collections::HashMap};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

use crate::{app::game::check_canvas_size, redraw_loop::Drawable};

pub enum Command {
    Pan(Vector2D<f64, Pixels>),
    /// In percent relative to current zoom
    Zoom(f64, Point2D<f64, Pixels>),
    SetSolarSystem(Option<&'static SolarSystem>),
}

pub struct SpaceView {
    canvas: Option<HtmlCanvasElement>,
    context: Option<CanvasRenderingContext2d>,
    backdrop: Option<HtmlImageElement>,
    location_images: HashMap<i64, HtmlImageElement>,
    solar_system: Option<&'static SolarSystem>,
    look_at: Point2D<f64, Solar>,
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
                look_at: Point2D::new(0., 0.),
                zoom: 1.,
                receiver,
                solar_system: None,
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
                Command::SetSolarSystem(system) => {
                    self.solar_system = system;
                    self.load_solar_system_images();
                }
                Command::Zoom(fraction, focus) => {
                    info!("Zooming from {} by {} at {:?}", self.zoom, fraction, focus);
                    let scale = self.scale();
                    let new_zoom = self.zoom + self.zoom * fraction;
                    let new_zoom = new_zoom.min(10.).max(0.1);
                    let new_scale = Scale::<f64, Solar, Pixels>::new(new_zoom);

                    if let Some(center) = self.canvas_center() {
                        let focus = focus.to_f64();
                        let focus_offset = focus.to_vector() - center.to_vector();
                        let focus_solar = self.look_at + focus_offset / scale;

                        let new_focus_location = self
                            .convert_world_to_canvas_with_scale(focus_solar, new_scale)
                            .unwrap();
                        let pixel_delta = new_focus_location.to_vector() - focus.to_vector();
                        let solar_delta = pixel_delta / new_scale;

                        info!("focus_offset: {:?}, focus_solar: {:?}, new_focus_loc: {:?}, pixel_delta: {:?}, solar_delta: {:?}", focus_offset, focus_solar, new_focus_location, pixel_delta, solar_delta);

                        self.look_at -= solar_delta;
                        // let new_focus_solar = self.look_at + focus_offset / new_scale;
                        // let top_left_solar = Point2D::<f64, Pixels>::zero() / scale;
                        //
                        // info!(
                        //     "focus_offset: {:?}, focus_solar: {:?}, new_focus_solar: {:?}",
                        //     focus_offset, focus_solar, new_focus_solar
                        // );
                        // self.look_at +=
                        //     (new_focus_solar.to_vector() - self.look_at.to_vector()) / 2.;

                        // let world_focus = self.convert_canvas_to_world(focus.to_f64()).unwrap();
                        // let world_focus_in_new_zoom = world_focus + world_focus * fraction;
                        // info!(
                        //     "Focus offset: {:?}, New Zoom: {}, new_zoom_focus: {:?}",
                        //     focus_offset, new_zoom, world_focus_in_new_zoom
                        // );
                        // self.look_at += focus_offset / new_zoom - focus_offset / self.zoom;
                        //- (focus_offset * self.zoom - focus_offset * new_zoom);
                    }
                    self.zoom = new_zoom;
                }
                Command::Pan(amount) => {
                    self.look_at += amount / self.scale();
                }
            }
        }

        Ok(())
    }

    fn scale(&self) -> Scale<f64, Solar, Pixels> {
        Scale::new(self.zoom)
    }

    fn convert_canvas_to_world(
        &mut self,
        canvas_location: Point2D<f64, Pixels>,
    ) -> Option<Point2D<f64, Solar>> {
        self.convert_canvas_to_world_with_scale(canvas_location, self.scale())
    }

    fn convert_canvas_to_world_with_scale(
        &mut self,
        canvas_location: Point2D<f64, Pixels>,
        scale: Scale<f64, Solar, Pixels>,
    ) -> Option<Point2D<f64, Solar>> {
        self.canvas_center().map(move |canvas_center| {
            let relative_location = canvas_location - canvas_center;
            let result = self.look_at + relative_location / scale;

            info!(
                "convert_canvas_to_world({:?}) - {:?} - {:?} - {:?} = {:?}",
                canvas_location, canvas_center, self.look_at, self.zoom, result
            );

            result
        })
    }

    fn convert_world_to_canvas(
        &mut self,
        world_location: Point2D<f64, Solar>,
    ) -> Option<Point2D<f64, Pixels>> {
        self.convert_world_to_canvas_with_scale(world_location, self.scale())
    }

    fn convert_world_to_canvas_with_scale(
        &mut self,
        world_location: Point2D<f64, Solar>,
        scale: Scale<f64, Solar, Pixels>,
    ) -> Option<Point2D<f64, Pixels>> {
        self.canvas_center().map(move |canvas_center| {
            let relative_location = world_location - self.look_at.to_vector();
            let result = canvas_center + relative_location.to_vector() * scale;

            info!(
                "convert_world_to_canvas({:?}) - {:?} - {:?} - {:?} = {:?}",
                world_location, canvas_center, self.look_at, self.zoom, result
            );

            result
        })
    }

    fn canvas_center(&mut self) -> Option<Point2D<f64, Pixels>> {
        self.canvas_size()
            .map(|s| (s.to_f64().to_vector() / 2.).to_point())
    }

    fn canvas_size(&mut self) -> Option<Size2D<i32, Pixels>> {
        self.canvas()
            .map(|canvas| Size2D::new(canvas.client_width(), canvas.client_height()))
    }

    fn load_solar_system_images(&mut self) {
        self.location_images.clear();

        if let Some(solar_system) = &self.solar_system {
            self.backdrop = solar_system.background.map(|url| {
                let image = HtmlImageElement::new().unwrap();
                image.set_src(url);
                image
            });

            for (id, location) in solar_system.locations.iter() {
                let image = HtmlImageElement::new().unwrap();
                image.set_src(location.image);
                self.location_images.insert(*id, image);
            }
        } else {
            self.backdrop = None;
        }
    }
}

impl Drawable for SpaceView {
    fn initialize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_frame(&mut self) -> anyhow::Result<()> {
        if self.receive_commands().is_err() {
            return Ok(());
        }

        if let Some(canvas) = self.canvas() {
            if let Some(context) = self.context() {
                check_canvas_size(&canvas);

                let scale = self.scale();

                let size = Size2D::<_, Pixels>::new(canvas.client_width(), canvas.client_height())
                    .to_f64();
                let canvas_center = (size.to_vector() / 2.).to_point();
                let center = canvas_center + self.look_at.to_vector() * scale;

                context.set_image_smoothing_enabled(false);

                let backdrop = self.backdrop.as_ref().unwrap();
                context.fill_rect(0., 0., size.width, size.height);
                if backdrop.complete() {
                    // The backdrop is tiled and panned based on the look_at unaffected by zoom
                    let backdrop_center = canvas_center + self.look_at.to_vector() * scale * 0.1;
                    let size = size.ceil().to_i32();
                    let backdrop_width = backdrop.width() as i32;
                    let backdrop_height = backdrop.height() as i32;
                    let mut y = (backdrop_center.y) as i32 % backdrop_height;
                    if y > 0 {
                        y -= backdrop_height;
                    }
                    while y < size.height {
                        let mut x = (backdrop_center.x) as i32 % backdrop_width;
                        if x > 0 {
                            x -= backdrop_width;
                        }
                        while x < size.width {
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

                if let Some(solar_system) = &self.solar_system {
                    for (id, location) in solar_system.locations.iter() {
                        let image = &self.location_images[id];
                        if image.complete() {
                            let render_radius = location.size * self.zoom;
                            let render_center = center + location.location.to_vector() * scale;

                            if let Err(err) = context
                                .draw_image_with_html_image_element_and_dw_and_dh(
                                    image,
                                    render_center.x - render_radius,
                                    render_center.y - render_radius,
                                    render_radius * 2.,
                                    render_radius * 2.,
                                )
                            {
                                error!("Error rendering sun: {:#?}", err);
                            }
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
