use client_api::ApiAgent;
use std::collections::HashMap;
use wasm_bindgen::JsValue;
use yew::{agent::Bridged, Callback};

use crate::{
    client_api::{self, AgentMessage, ApiBridge},
    extended_text_metrics::ExtendedTextMetrics,
};

use super::{
    controller::{CanvasScalable, View, ViewContext},
    Button,
};
use cosmicverge_shared::{
    euclid::{Point2D, Scale, Size2D, Vector2D},
    protocol::{
        CosmicVergeRequest, PilotLocation, PilotingAction, SolarSystemLocation,
        SolarSystemLocationId,
    },
    ships::{hangar, ShipId},
    solar_systems::{universe, Pixels, Solar, SolarSystem, SolarSystemId},
};
use web_sys::HtmlImageElement;

pub struct SystemRenderer {
    look_at: Point2D<f32, Solar>,
    zoom: f32,
    backdrop: Option<HtmlImageElement>,
    location_images: HashMap<SolarSystemLocationId, HtmlImageElement>,
    ship_images: HashMap<ShipId, HtmlImageElement>,
    solar_system: Option<&'static SolarSystem>,
    api: ApiBridge,
}

impl SystemRenderer {
    pub fn new(solar_system: &'static SolarSystem) -> Self {
        let mut renderer = Self {
            solar_system: Some(solar_system),
            zoom: 1.,
            api: ApiAgent::bridge(Callback::noop()),
            backdrop: None,
            look_at: Default::default(),
            location_images: Default::default(),
            ship_images: Default::default(),
        };
        renderer.load_solar_system_images();
        renderer
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

    fn switch_system(&mut self, system: SolarSystemId) {
        if self.solar_system.is_none() || self.solar_system.unwrap().id != system {
            self.solar_system = Some(universe().get(&system));
            self.load_solar_system_images();
            self.zoom = 1.;
            self.look_at = Default::default();
        }
    }
}

impl View for SystemRenderer {
    fn handle_click(
        &mut self,
        button: super::Button,
        count: i64,
        location: cosmicverge_shared::euclid::Point2D<
            i32,
            cosmicverge_shared::solar_systems::Pixels,
        >,
        view: &ViewContext,
    ) {
        if count == 2 && (button == Button::Left || button == Button::OneFinger) {
            let location = self.convert_canvas_to_world(location.to_f32(), &view.canvas);
            if view.active_pilot.is_some() {
                let request = CosmicVergeRequest::Fly(PilotingAction::NavigateTo(PilotLocation {
                    location: SolarSystemLocation::InSpace(location),
                    system: self.solar_system.unwrap().id,
                }));
                self.api.send(AgentMessage::Request(request));
            }
        }
    }

    fn render(&mut self, view: &ViewContext) {
        // TODO only follow the ship if we
        let mut switch_system_to = None;
        if let Some(ship) = &view.active_ship {
            if Some(ship.physics.system) != self.solar_system.map(|s| s.id) {
                switch_system_to = Some((ship.physics.system, ship.physics.location));
            }
        }

        if let Some((new_system, new_look_at)) = switch_system_to {
            self.switch_system(new_system);
            self.look_at = new_look_at;
        }

        let scale = self.scale();
        let canvas = &view.canvas;
        let context = &view.context;

        let size = Size2D::<_, Pixels>::new(canvas.client_width(), canvas.client_height()).to_f32();
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
                    if let Err(err) =
                        context.draw_image_with_html_image_element(backdrop, x as f64, y as f64)
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
                    let render_center =
                        (center + orbits[&location.id.id()].to_vector().to_f32() * scale).to_f64();

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

            if let Some(simulation_system) = view.simulation_system {
                if simulation_system == solar_system.id {
                    context.save();
                    context.set_font("18px Orbitron, sans-serif");
                    for (ship, location, orientation) in view.pilot_locations.iter() {
                        let ship_spec = hangar().load(&ship.ship.ship);
                        let image = self.ship_images.entry(ship_spec.id).or_insert_with(|| {
                            let image = HtmlImageElement::new().unwrap();
                            image.set_src(ship_spec.image);
                            image
                        });
                        if image.complete() {
                            let render_radius = (image.width() as f64 / 2.) * self.zoom as f64;
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
                                let maximum_ship_size = (render_radius.powf(2.) * 2.).sqrt();
                                let nameplate_top = (render_center.y + maximum_ship_size).ceil();
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

    fn zoom(&mut self, fraction: f32, focus: Point2D<f32, Pixels>, view: &ViewContext) {
        let (new_zoom, new_look_at) = self.calculate_zoom(fraction, focus, &view.canvas);
        self.zoom = new_zoom;
        self.look_at = new_look_at;
    }

    fn pan(&mut self, amount: Vector2D<f32, Pixels>, _: &ViewContext) {
        self.look_at -= amount / self.scale();
    }
}

impl CanvasScalable for SystemRenderer {
    fn scale<Unit>(&self) -> Scale<f32, Unit, Pixels> {
        Scale::new(self.zoom)
    }

    fn look_at<Unit>(&self) -> Point2D<f32, Unit> {
        self.look_at.cast_unit()
    }
}
