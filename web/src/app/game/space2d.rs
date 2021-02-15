use std::collections::HashMap;

use cosmicverge_shared::{
    euclid::{Point2D, Scale, Size2D, Vector2D},
    protocol::{
        ActivePilot, CosmicVergeRequest, PilotLocation, PilotedShip, PilotingAction,
        SolarSystemLocation, SolarSystemLocationId,
    },
    ships::{hangar, ShipId},
    solar_system_simulation::{interpolate_value, InterpolationMode, SolarSystemSimulation},
    solar_systems::{universe, Pixels, Solar, SolarSystem, SolarSystemId},
};
use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, Performance};
use yew::{Bridged, Callback};

use super::Button;
use crate::{
    app::game::check_canvas_size,
    client_api::{self, AgentMessage, ApiAgent, ApiBridge},
    extended_text_metrics::ExtendedTextMetrics,
    redraw_loop::Drawable,
};

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
    SetSolarSystem(Option<&'static SolarSystem>),

    UpdateServerRoundtripTime(f64),

    UpdateSolarSystem {
        solar_system: SolarSystemId,
        ships: Vec<PilotedShip>,
        timestamp: f64,
    },
}

const SHIP_TWEEN_DURATION: f64 = 1.0;

pub struct SpaceView {
    performance: Performance,
    canvas: Option<HtmlCanvasElement>,
    context: Option<CanvasRenderingContext2d>,
    backdrop: Option<HtmlImageElement>,
    location_images: HashMap<SolarSystemLocationId, HtmlImageElement>,
    ship_images: HashMap<ShipId, HtmlImageElement>,
    solar_system: Option<&'static SolarSystem>,
    look_at: Point2D<f32, Solar>,
    zoom: f32,
    receiver: Receiver<Command>,
    active_pilot: Option<ActivePilot>,
    api: ApiBridge,
    simulation_system: Option<SolarSystemId>,
    simulation: Option<SolarSystemSimulation>,
    tweened_simulation: Option<TweenedSimulation>,
    last_physics_update: Option<f64>,
    server_roundtrip_time: Option<f64>,
}

struct TweenedSimulation {
    simulation: SolarSystemSimulation,
    start_timestamp: f64,
}

impl SpaceView {
    pub fn new() -> (Self, Sender<Command>) {
        let performance = web_sys::window().unwrap().performance().unwrap();
        let api = ApiAgent::bridge(Callback::noop());
        let (sender, receiver) = channel::unbounded();
        (
            Self {
                performance,
                canvas: None,
                context: None,
                backdrop: None,
                location_images: Default::default(),
                ship_images: Default::default(),
                look_at: Point2D::new(0., 0.),
                zoom: 1.,
                receiver,
                solar_system: None,
                active_pilot: None,
                api,
                simulation_system: None,
                simulation: None,
                tweened_simulation: None,
                last_physics_update: None,
                server_roundtrip_time: None,
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
                Command::SetSolarSystem(system) => {
                    self.solar_system = system;
                    self.load_solar_system_images();
                }
                Command::Zoom(fraction, focus) => {
                    let scale = self.scale();
                    let new_zoom = self.zoom + self.zoom * fraction;
                    let new_zoom = new_zoom.min(10.).max(0.1);
                    let new_scale = Scale::<f32, Solar, Pixels>::new(new_zoom);

                    if let Some(center) = self.canvas_center() {
                        let focus = focus.to_f32();
                        let focus_offset = focus.to_vector() - center.to_vector();
                        let focus_solar = self.look_at + focus_offset / scale;

                        let new_focus_location = self
                            .convert_world_to_canvas_with_scale(focus_solar, new_scale)
                            .unwrap();
                        let pixel_delta = new_focus_location.to_vector() - focus.to_vector();
                        let solar_delta = pixel_delta / new_scale;

                        self.look_at += solar_delta;
                    }
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
                    if count == 2 && (button == Button::Left || button == Button::OneFinger) {
                        if let Some(location) = self.convert_canvas_to_world(location.to_f32()) {
                            if self.active_pilot.is_some() {
                                let request = CosmicVergeRequest::Fly(PilotingAction::NavigateTo(
                                    PilotLocation {
                                        location: SolarSystemLocation::InSpace(location),
                                        system: self.solar_system.unwrap().id,
                                    },
                                ));
                                self.api.send(AgentMessage::Request(request));
                            }
                        }
                    }
                }
                Command::UpdateSolarSystem {
                    ships,
                    solar_system,
                    timestamp,
                } => {
                    self.simulation_system = Some(solar_system);
                    // TODO we shouldn't always follow the ship
                    self.switch_system(solar_system);

                    let current_simulation_timestamp = self
                        .simulation
                        .as_ref()
                        .map(|s| s.timestamp)
                        .unwrap_or(timestamp);
                    let mut simulation = SolarSystemSimulation::new(solar_system, timestamp);
                    simulation.add_ships(ships);

                    self.last_physics_update = None;

                    // Since the simulation keeps track of how much time it thinks has elapsed, we know how much time
                    // has elapsed since this calculation was made and can accurately update. However, in the case of
                    // a negative duration, our only resort is to tween.
                    let simulation_catchup = current_simulation_timestamp - timestamp;
                    if simulation_catchup > 0. {
                        simulation.step(simulation_catchup as f32);
                    }

                    self.tweened_simulation =
                        self.simulation
                            .replace(simulation)
                            .map(|simulation| TweenedSimulation {
                                simulation,
                                start_timestamp: current_simulation_timestamp,
                            });
                }
                Command::UpdateServerRoundtripTime(server_roundtrip_time) => {
                    self.server_roundtrip_time = Some(server_roundtrip_time)
                }
            }
        }

        Ok(())
    }

    fn scale(&self) -> Scale<f32, Solar, Pixels> {
        Scale::new(self.zoom)
    }

    fn convert_canvas_to_world(
        &mut self,
        canvas_location: Point2D<f32, Pixels>,
    ) -> Option<Point2D<f32, Solar>> {
        self.convert_canvas_to_world_with_scale(canvas_location, self.scale())
    }

    fn convert_canvas_to_world_with_scale(
        &mut self,
        canvas_location: Point2D<f32, Pixels>,
        scale: Scale<f32, Solar, Pixels>,
    ) -> Option<Point2D<f32, Solar>> {
        self.canvas_center().map(move |canvas_center| {
            let relative_location = canvas_location - canvas_center;
            self.look_at + relative_location / scale
        })
    }

    fn switch_system(&mut self, system: SolarSystemId) {
        if self.solar_system.is_none() || self.solar_system.unwrap().id != system {
            self.solar_system = Some(universe().get(&system));
            self.load_solar_system_images();

            self.focus_on_pilot();
        }
    }

    fn focus_on_pilot(&mut self) {
        let mut look_at = None;
        if let Some(simulation) = &self.simulation {
            if let Some(pilot) = &self.active_pilot {
                if let Some(ship) = simulation.lookup_ship(&pilot.pilot.id) {
                    look_at = Some(ship.physics.location);
                }
            }
        }

        if let Some(look_at) = look_at {
            self.look_at = look_at;
        }
    }

    // fn convert_world_to_canvas(
    //     &mut self,
    //     world_location: Point2D<f32, Solar>,
    // ) -> Option<Point2D<f32, Pixels>> {
    //     self.convert_world_to_canvas_with_scale(world_location, self.scale())
    // }

    fn convert_world_to_canvas_with_scale(
        &mut self,
        world_location: Point2D<f32, Solar>,
        scale: Scale<f32, Solar, Pixels>,
    ) -> Option<Point2D<f32, Pixels>> {
        self.canvas_center().map(move |canvas_center| {
            let relative_location = world_location - self.look_at.to_vector();
            canvas_center + relative_location.to_vector() * scale
        })
    }

    fn canvas_center(&mut self) -> Option<Point2D<f32, Pixels>> {
        self.canvas_size()
            .map(|s| (s.to_f32().to_vector() / 2.).to_point())
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
                image.set_src(&location.image_url());
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

                let now = self.performance.now();
                if let Some(last_physics_timestamp_ms) = self.last_physics_update {
                    if let Some(simulation) = &mut self.simulation {
                        let elapsed = (now - last_physics_timestamp_ms) / 1000.;
                        simulation.step(elapsed as f32);
                        if let Some(tweened_simulation) = self.tweened_simulation.as_mut() {
                            if simulation.timestamp - tweened_simulation.start_timestamp
                                > SHIP_TWEEN_DURATION
                            {
                                self.tweened_simulation = None;
                            } else {
                                tweened_simulation.simulation.step(elapsed as f32);
                            }
                        }

                        // TODO only follow the ship if we
                        let mut switch_system_to = None;
                        if let Some(pilot) = &self.active_pilot {
                            if let Some(ship) = simulation.lookup_ship(&pilot.pilot.id) {
                                if Some(ship.physics.system) != self.solar_system.map(|s| s.id) {
                                    switch_system_to = Some(ship.physics.system);
                                }
                            }
                        }
                        if let Some(new_system) = switch_system_to {
                            self.switch_system(new_system);
                        }
                    }
                }
                self.last_physics_update = Some(now);

                let scale = self.scale();

                let size = Size2D::<_, Pixels>::new(canvas.client_width(), canvas.client_height())
                    .to_f32();
                let canvas_center = (size.to_vector() / 2.).to_point();
                let center = canvas_center - self.look_at.to_vector() * scale;

                context.set_image_smoothing_enabled(false);

                context.set_fill_style(&JsValue::from_str("#000"));

                let backdrop = self.backdrop.as_ref().unwrap();
                context.fill_rect(0., 0., size.width as f64, size.height as f64);
                if backdrop.complete() {
                    // The backdrop is tiled and panned based on the look_at unaffected by zoom
                    let backdrop_center = canvas_center - self.look_at.to_vector() * scale * 0.1;
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
                    let orbits = universe().orbits_for(&solar_system.id);
                    for (id, location) in solar_system.locations.iter() {
                        let image = &self.location_images[id];
                        if image.complete() {
                            let render_radius = (location.size * self.zoom) as f64;
                            let render_center = (center
                                + orbits[&location.id.id()].to_vector().to_f32() * scale)
                                .to_f64();

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

                    if let Some(simulation_system) = self.simulation_system {
                        if simulation_system == solar_system.id {
                            context.save();
                            context.set_font("18px Orbitron, sans-serif");
                            for ship in self.simulation.as_ref().unwrap().all_ships() {
                                let mut location = ship.physics.location;
                                let mut orientation = ship.physics.rotation;
                                if let Some(tweened) = &self.tweened_simulation {
                                    if let Some(tweened_ship) =
                                        tweened.simulation.lookup_ship(&ship.pilot_id)
                                    {
                                        let amount = (self.simulation.as_ref().unwrap().timestamp
                                            - tweened.start_timestamp)
                                            / SHIP_TWEEN_DURATION;
                                        let amount = amount.clamp(0.0, 1.0) as f32;
                                        location = interpolate_value(
                                            tweened_ship.physics.location.to_vector(),
                                            location.to_vector(),
                                            amount,
                                            InterpolationMode::Linear,
                                        )
                                        .to_point();
                                        orientation = interpolate_value(
                                            tweened_ship.physics.rotation,
                                            orientation,
                                            amount,
                                            InterpolationMode::Linear,
                                        );
                                    }
                                }

                                let ship_spec = hangar().load(&ship.ship.ship);
                                let image =
                                    self.ship_images.entry(ship_spec.id).or_insert_with(|| {
                                        let image = HtmlImageElement::new().unwrap();
                                        image.set_src(ship_spec.image);
                                        image
                                    });
                                if image.complete() {
                                    let render_radius =
                                        (image.width() as f64 / 2.) * self.zoom as f64;
                                    let render_center =
                                        center.to_f64() + (location.to_vector() * scale).to_f64();
                                    context.save();
                                    context.translate(render_center.x, render_center.y).unwrap();
                                    context.rotate(orientation.signed().get() as f64).unwrap();
                                    if let Err(err) = context
                                        .draw_image_with_html_image_element_and_dw_and_dh(
                                            image,
                                            -render_radius,
                                            -render_radius,
                                            render_radius * 2.,
                                            render_radius * 2.,
                                        )
                                    {
                                        error!("Error rendering ship: {:#?}", err);
                                    }
                                    context.restore();

                                    if let Some(pilot) =
                                        client_api::pilot_information(ship.pilot_id, &mut self.api)
                                    {
                                        let text_metrics = ExtendedTextMetrics::from(
                                            context.measure_text(&pilot.name).unwrap(),
                                        );

                                        const NAMEPLATE_PADDING: f64 = 5.;
                                        context.set_fill_style(&JsValue::from_str("#df0772"));
                                        let text_left =
                                            (render_center.x - text_metrics.width() / 2.).floor();
                                        // Since it's a square, this is the simplified version of a^2 + b^2 = c^2
                                        let maximum_ship_size =
                                            (render_radius.powf(2.) * 2.).sqrt();
                                        let nameplate_top =
                                            (render_center.y + maximum_ship_size).ceil();
                                        let text_top = nameplate_top + NAMEPLATE_PADDING;
                                        context.fill_rect(
                                            text_left - NAMEPLATE_PADDING,
                                            nameplate_top,
                                            text_metrics.width() + 2. * NAMEPLATE_PADDING,
                                            text_metrics.height() + 2. * NAMEPLATE_PADDING,
                                        );
                                        context.set_fill_style(&JsValue::from_str("#FFF"));
                                        let _ = context.fill_text(
                                            &pilot.name,
                                            text_left,
                                            (text_top + text_metrics.height()).ceil(),
                                        );
                                    }
                                }
                            }
                            context.restore();
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
