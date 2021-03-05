use cosmicverge_shared::{
    euclid::{Angle, Point2D, Scale, Size2D, Vector2D},
    protocol::navigation,
    solar_systems::{Pixels, Solar, SolarSystem, SystemId},
};
use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, Performance};

use super::{simulator::Simulator, system_renderer::SystemRenderer, Button};
use crate::{app::game::check_canvas_size, redraw_loop::Drawable};

pub enum Command {
    SetPilot(navigation::ActivePilot),

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
        solar_system: SystemId,
        ships: Vec<navigation::Ship>,
        timestamp: f64,
    },
}

pub struct GameController {
    hud: Option<HtmlElement>,
    canvas: Option<HtmlCanvasElement>,
    context: Option<CanvasRenderingContext2d>,
    performance: Performance,
    active_pilot: Option<navigation::ActivePilot>,
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

    fn zoom(&mut self, fraction: f32, focus: Point2D<f32, Pixels>, view: &ViewContext);
    fn pan(&mut self, amount: Vector2D<f32, Pixels>, view: &ViewContext);
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
                hud: None,
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
                    let pilot_system = active_pilot.location.system;
                    self.active_pilot = Some(active_pilot);
                    self.view_solar_system(&pilot_system);
                }
                Command::ViewSolarSystem(system) => {
                    self.view_solar_system(&system.id);
                }
                Command::Zoom(fraction, focus) => {
                    let context = self.view_context();
                    if let Some(view) = self.view.as_mut() {
                        view.zoom(fraction, focus, &context);
                    }
                }
                Command::Pan(amount) => {
                    let context = self.view_context();
                    if let Some(view) = self.view.as_mut() {
                        view.pan(amount, &context);
                    }
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
                    self.simulator
                        .update(ships, solar_system, timestamp, self.performance.now());
                }
                Command::UpdateServerRoundtripTime(rtt) => {
                    self.simulator.server_round_trip_avg = Some(rtt);
                }
            }
        }

        Ok(())
    }

    fn view_solar_system(&mut self, solar_system: &SystemId) {
        self.set_view(SystemRenderer::new(solar_system));
    }

    fn set_view<T: View + 'static>(&mut self, view: T) {
        self.view = Some(Box::new(view));

        if let Some(hud) = self.hud() {
            while let Some(child) = hud.first_element_child() {
                child.remove();
            }
        }
    }

    pub fn hud(&mut self) -> Option<HtmlElement> {
        if self.hud.is_none() {
            self.hud = Some(
                web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("hud")
                    .unwrap()
                    .dyn_into::<web_sys::HtmlElement>()
                    .ok()?,
            );
        }

        self.hud.clone()
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
        let active_ship = if let Some(pilot) = self.active_pilot.as_ref() {
            self.simulator
                .simulation
                .as_ref()
                .map(|s| s.lookup_ship(pilot.pilot.id))
                .flatten()
                .cloned()
        } else {
            None
        };
        ViewContext {
            active_ship,
            hud: self.hud().unwrap(),
            canvas: self.canvas().unwrap(),
            context: self.context().unwrap(),
            performance: self.performance.clone(),
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
    pub hud: HtmlElement,
    pub canvas: HtmlCanvasElement,
    pub context: CanvasRenderingContext2d,
    pub performance: Performance,
    pub active_pilot: Option<navigation::ActivePilot>,
    pub active_ship: Option<navigation::Ship>,
    pub simulation_system: Option<SystemId>,
    pub pilot_locations: Vec<(navigation::Ship, Point2D<f32, Solar>, Angle<f32>)>,
}

pub trait CanvasScalable {
    fn scale<Unit>(&self) -> Scale<f32, Unit, Pixels>;
    fn look_at<Unit>(&self) -> Point2D<f32, Unit>;

    fn canvas_size(&self, canvas: &HtmlCanvasElement) -> Size2D<i32, Pixels> {
        Size2D::new(canvas.client_width(), canvas.client_height())
    }

    fn canvas_center(&self, canvas: &HtmlCanvasElement) -> Point2D<f32, Pixels> {
        (self.canvas_size(canvas).to_f32().to_vector() / 2.).to_point()
    }

    fn convert_canvas_to_world_with_scale<Unit>(
        &self,
        canvas_location: Point2D<f32, Pixels>,
        scale: Scale<f32, Unit, Pixels>,
        canvas: &HtmlCanvasElement,
    ) -> Point2D<f32, Unit> {
        let relative_location = canvas_location - self.canvas_center(canvas);
        self.look_at() + relative_location / scale
    }

    fn convert_canvas_to_world<Unit>(
        &self,
        canvas_location: Point2D<f32, Pixels>,
        canvas: &HtmlCanvasElement,
    ) -> Point2D<f32, Unit> {
        self.convert_canvas_to_world_with_scale(canvas_location, self.scale(), canvas)
    }

    fn convert_world_to_canvas_with_scale<Unit>(
        &self,
        world_location: Point2D<f32, Unit>,
        scale: Scale<f32, Unit, Pixels>,
        canvas: &HtmlCanvasElement,
    ) -> Point2D<f32, Pixels> {
        let relative_location = world_location - self.look_at().to_vector();
        self.canvas_center(canvas) + relative_location.to_vector() * scale
    }

    fn calculate_zoom<Unit>(
        &self,
        fraction: f32,
        focus: Point2D<f32, Pixels>,
        canvas: &HtmlCanvasElement,
    ) -> (f32, Point2D<f32, Unit>) {
        let scale = self.scale();
        let new_zoom = scale.get() + scale.get() * fraction;
        let new_zoom = new_zoom.min(10.).max(0.1);
        let new_scale = Scale::<f32, Unit, Pixels>::new(new_zoom);

        let center = self.canvas_center(canvas);
        let focus_offset = focus.to_vector() - center.to_vector();
        let focus_solar = self.look_at() + focus_offset / scale;

        let new_focus_location =
            self.convert_world_to_canvas_with_scale(focus_solar, new_scale, canvas);
        let pixel_delta = new_focus_location.to_vector() - focus.to_vector();
        let solar_delta = pixel_delta / new_scale;

        (new_zoom, self.look_at() + solar_delta)
    }
}
