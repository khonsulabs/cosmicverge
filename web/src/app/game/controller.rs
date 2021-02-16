use cosmicverge_shared::{
    euclid::{Angle, Point2D, Scale, Size2D, UnknownUnit, Vector2D},
    protocol::{ActivePilot, PilotedShip},
    solar_systems::{Pixels, Solar, SolarSystem, SolarSystemId},
};
use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, Performance};

use super::{simulator::Simulator, system_renderer::SystemRenderer, Button};
use crate::{app::game::check_canvas_size, redraw_loop::Drawable};

pub const SHIP_TWEEN_DURATION: f64 = 1.0;

pub enum Command {
    SetPilot(ActivePilot),

    HandleClick {
        button: Button,
        count: i64,
        location: Point2D<i32, Pixels>,
    },

    Pan(Vector2D<f32, Pixels>),
    /// In percent relative to current zoom
    Zoom(f32, Point2D<f32, Pixels>),
    ViewSolarSystem(&'static SolarSystem),

    UpdateServerRoundtripTime(f64),

    UpdateSolarSystem {
        solar_system: SolarSystemId,
        ships: Vec<PilotedShip>,
        timestamp: f64,
    },
}

pub struct GameController {
    canvas: Option<HtmlCanvasElement>,
    context: Option<CanvasRenderingContext2d>,
    performance: Performance,
    look_at: Point2D<f32, UnknownUnit>,
    zoom: f32,
    active_pilot: Option<ActivePilot>,
    simulator: Simulator,
    receiver: Receiver<Command>,
    view: Option<Box<dyn View>>,
}

pub trait View {
    fn handle_click(
        &mut self,
        button: Button,
        count: i64,
        location: Point2D<i32, Pixels>,
        context: &ViewContext,
    );

    fn render(&mut self, view: &ViewContext);
}

impl GameController {
    pub fn new() -> (Self, Sender<Command>) {
        let performance = web_sys::window().unwrap().performance().unwrap();
        let (sender, receiver) = channel::unbounded();
        (
            Self {
                performance,
                canvas: None,
                context: None,
                view: None,
                look_at: Point2D::new(0., 0.),
                zoom: 1.,
                receiver,
                active_pilot: None,
                simulator: Simulator::default(),
            },
            sender,
        )
    }

    fn receive_commands(&mut self) -> anyhow::Result<()> {
        while let Some(event) = match self.receiver.try_recv() {
            Ok(command) => Some(command),
            Err(TryRecvError::Empty) => None,
            Err(disconnected) => return Err(disconnected.into()),
        } {
            match event {
                Command::SetPilot(active_pilot) => {
                    self.active_pilot = Some(active_pilot);
                    self.focus_on_pilot();
                }
                Command::ViewSolarSystem(system) => {
                    self.view = Some(Box::new(SystemRenderer::new(system)));
                }
                Command::Zoom(fraction, focus) => {
                    let scale = self.scale();
                    let new_zoom = self.zoom + self.zoom * fraction;
                    let new_zoom = new_zoom.min(10.).max(0.1);
                    let new_scale = Scale::<f32, UnknownUnit, Pixels>::new(new_zoom);

                    let center = self.canvas_center();
                    let focus = focus.to_f32();
                    let focus_offset = focus.to_vector() - center.to_vector();
                    let focus_solar = self.look_at + focus_offset / scale;

                    let new_focus_location =
                        self.convert_world_to_canvas_with_scale(focus_solar, new_scale);
                    let pixel_delta = new_focus_location.to_vector() - focus.to_vector();
                    let solar_delta = pixel_delta / new_scale;

                    self.look_at += solar_delta;
                    self.zoom = new_zoom;
                }
                Command::Pan(amount) => {
                    self.look_at -= amount / self.scale();
                }
                Command::HandleClick {
                    button,
                    count,
                    location,
                } => {
                    let context = self.view_context();
                    if let Some(view) = self.view.as_mut() {
                        view.handle_click(button, count, location, &context);
                    }
                }
                Command::UpdateSolarSystem {
                    ships,
                    solar_system,
                    timestamp,
                } => {
                    self.simulator.update(ships, solar_system, timestamp);

                    // TODO we shouldn't always follow the ship
                    // self.switch_system(solar_system);
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn focus_on_pilot(&mut self) {
        let mut look_at = None;
        if let Some(pilot) = &self.active_pilot {
            if let Some(location) = self.simulator.pilot_location(&pilot.pilot.id) {
                look_at = Some(location);
            }
        }

        // TODO switch to solar system here too
        if let Some(look_at) = look_at {
            self.look_at = look_at.cast_unit();
        }
    }

    pub fn canvas(&mut self) -> Option<HtmlCanvasElement> {
        if self.canvas.is_none() {
            self.canvas = Some(
                web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("layer1")
                    .unwrap()
                    .dyn_into::<web_sys::HtmlCanvasElement>()
                    .ok()?,
            );
        }

        self.canvas.clone()
    }

    pub fn context(&mut self) -> Option<CanvasRenderingContext2d> {
        if self.context.is_none() {
            self.context = Some({
                self.canvas()?
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<CanvasRenderingContext2d>()
                    .ok()?
            });
        }

        self.context.clone()
    }

    pub fn view_context(&mut self) -> ViewContext {
        ViewContext {
            canvas: self.canvas().unwrap(),
            context: self.context().unwrap(),
            performance: self.performance.clone(),
            look_at: self.look_at,
            zoom: self.zoom,
            active_pilot: self.active_pilot.clone(),
            simulation_system: self.simulator.simulation_system,
            pilot_locations: self.simulator.pilot_locations(),
        }
    }
}

impl Drawable for GameController {
    fn initialize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_frame(&mut self) -> anyhow::Result<()> {
        if self.receive_commands().is_err() {
            return Ok(());
        }

        if let Some(canvas) = self.canvas() {
            check_canvas_size(&canvas);
            self.simulator.step(self.performance.now());

            // TODO hook back up
            // TODO only follow the ship if we
            // let mut switch_system_to = None;
            // if let Some(pilot) = &view.active_pilot {
            //     if let Some(ship) = simulation.lookup_ship(&pilot.pilot.id) {
            //         if Some(ship.physics.system) != self.solar_system.map(|s| s.id) {
            //             switch_system_to = Some(ship.physics.system);
            //         }
            //     }
            // }
            // if let Some(new_system) = switch_system_to {
            //     self.switch_system(new_system);
            // }

            let context = self.view_context();
            if let Some(view) = self.view.as_mut() {
                view.render(&context);
            }
        }

        Ok(())
    }

    fn cleanup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct ViewContext {
    pub canvas: HtmlCanvasElement,
    pub context: CanvasRenderingContext2d,
    pub performance: Performance,
    look_at: Point2D<f32, UnknownUnit>,
    pub zoom: f32,
    pub active_pilot: Option<ActivePilot>,
    pub simulation_system: Option<SolarSystemId>,
    pub pilot_locations: Vec<(PilotedShip, Point2D<f32, Solar>, Angle<f32>)>,
}

pub trait CanvasScalable {
    fn canvas(&self) -> &HtmlCanvasElement;
    fn scale<Unit>(&self) -> Scale<f32, Unit, Pixels>;
    fn look_at<Unit>(&self) -> Point2D<f32, Unit>;

    fn canvas_size(&self) -> Size2D<i32, Pixels> {
        Size2D::new(self.canvas().client_width(), self.canvas().client_height())
    }

    fn canvas_center(&self) -> Point2D<f32, Pixels> {
        (self.canvas_size().to_f32().to_vector() / 2.).to_point()
    }

    fn convert_canvas_to_world_with_scale<Unit>(
        &self,
        canvas_location: Point2D<f32, Pixels>,
        scale: Scale<f32, Unit, Pixels>,
    ) -> Point2D<f32, Unit> {
        let relative_location = canvas_location - self.canvas_center();
        self.look_at() + relative_location / scale
    }

    fn convert_canvas_to_world<Unit>(
        &self,
        canvas_location: Point2D<f32, Pixels>,
    ) -> Point2D<f32, Unit> {
        self.convert_canvas_to_world_with_scale(canvas_location, self.scale())
    }

    fn convert_world_to_canvas_with_scale<Unit>(
        &self,
        world_location: Point2D<f32, Unit>,
        scale: Scale<f32, Unit, Pixels>,
    ) -> Point2D<f32, Pixels> {
        let relative_location = world_location - self.look_at().to_vector();
        self.canvas_center() + relative_location.to_vector() * scale
    }
}

impl CanvasScalable for ViewContext {
    fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }

    fn scale<Unit>(&self) -> Scale<f32, Unit, Pixels> {
        Scale::new(self.zoom)
    }

    fn look_at<Unit>(&self) -> Point2D<f32, Unit> {
        self.look_at.cast_unit()
    }
}

impl CanvasScalable for GameController {
    fn canvas(&self) -> &HtmlCanvasElement {
        self.canvas.as_ref().unwrap()
    }

    fn scale<Unit>(&self) -> Scale<f32, Unit, Pixels> {
        Scale::new(self.zoom)
    }

    fn look_at<Unit>(&self) -> Point2D<f32, Unit> {
        self.look_at.cast_unit()
    }
}
