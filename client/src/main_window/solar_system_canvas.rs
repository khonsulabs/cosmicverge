use std::{collections::HashMap, sync::Arc, time::Duration};

use cosmicverge_shared::{
    euclid::Vector2D,
    protocol::navigation,
    ships::{hangar, ShipId},
    solar_systems::{universe, Solar, SolarSystem, SystemId},
};
use kludgine::prelude::*;

use self::simulator::Simulator;
use crate::{cache::CachedImage, CosmicVergeClient};

mod simulator;
use chrono::Utc;

pub struct SolarSystemCanvas {
    api_client: CosmicVergeClient,
    solar_system: Option<SolarSystemCache>,
    look_at: Point<f32, Solar>,
    zoom: f32,
    simulator: Simulator,
    ship_images: HashMap<ShipId, Arc<CachedImage>>,
}

struct SolarSystemCache {
    solar_system: &'static SolarSystem,
    backdrop: Option<Arc<CachedImage>>,
    object_images: HashMap<navigation::SolarSystemId, Arc<CachedImage>>,
}

#[async_trait]
impl Component for SolarSystemCanvas {
    async fn update(&mut self, context: &mut Context) -> KludgineResult<()> {
        let orbit_timestamp = if let Some(simulation) = &self.simulator.simulation {
            simulation.timestamp
        } else {
            current_timestamp()
        };
        universe().update_orbits(orbit_timestamp);

        self.simulator.step(current_timestamp());

        // TODO the framerate here should be limited if we aren't simulating, but this is a simple implementation for now
        context
            .estimate_next_frame(Duration::from_nanos(1_000_000_000 / 60))
            .await;

        Ok(())
    }

    async fn render(&mut self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        if let Some(cache) = &self.solar_system {
            let scene_size = context.scene().size().await;
            let canvas_center = (scene_size / 2.).to_vector().to_point();
            let scale = self.scale();
            if let Some(backdrop) = cache.backdrop.as_ref() {
                if let Some(texture) = backdrop.texture().await? {
                    // The backdrop is tiled and panned based on the look_at unaffected by zoom
                    let backdrop_size = texture.size().to_i32();
                    let sprite = SpriteSource::entire_texture(texture);
                    let backdrop_center =
                        canvas_center.to_vector() - self.look_at.to_vector() * scale * 0.1;
                    let size = scene_size.ceil().to_i32();
                    let mut y = (backdrop_center.y) as i32 % backdrop_size.height;
                    if y > 0 {
                        y -= backdrop_size.height;
                    }
                    while y < size.height {
                        let mut x = (backdrop_center.x) as i32 % backdrop_size.width;
                        if x > 0 {
                            x -= backdrop_size.width;
                        }
                        while x < size.width {
                            sprite
                                .render_at(
                                    context.scene(),
                                    Point::new(x, y).to_f32(),
                                    SpriteRotation::default(),
                                )
                                .await;

                            x += backdrop_size.width;
                        }
                        y += backdrop_size.height;
                    }
                }
            } else {
                Shape::rect(layout.inner_bounds())
                    .fill(Fill::new(Color::BLACK))
                    .render_at(Default::default(), context.scene())
                    .await;
            }

            let orbits = universe().orbits_for(cache.solar_system.id);
            for (id, location) in cache.solar_system.locations.iter() {
                if let Some(image) = cache.object_images.get(id) {
                    if let Some(texture) = image.texture().await? {
                        let render_radius = (location.size * self.zoom) as f32;
                        let render_center = (canvas_center
                            + orbits[&location.id.id()].to_vector() * scale)
                            .to_f32();

                        let sprite = SpriteSource::entire_texture(texture);
                        sprite
                            .render_within(
                                context.scene(),
                                Rect::new(
                                    render_center - Vector2D::new(render_radius, render_radius),
                                    Size::new(render_radius * 2., render_radius * 2.),
                                ),
                                SpriteRotation::default(),
                            )
                            .await;
                    }
                }
            }

            if let Some(simulation_system) = self.simulator.simulation_system {
                if simulation_system == cache.solar_system.id {
                    for (ship, location, orientation) in self.simulator.pilot_locations() {
                        let ship_spec = hangar().load(&ship.ship.ship);
                        let texture = match self.ship_images.get(&ship_spec.id) {
                            Some(image) => image.texture().await?,
                            None => {
                                let image = CachedImage::new(ship_spec.image)
                                    .await
                                    .map_err(anyhow::Error::from)?;
                                self.ship_images.insert(ship_spec.id, image.clone());
                                image.texture().await?
                            }
                        };
                        if let Some(texture) = texture {
                            let sprite = SpriteSource::entire_texture(texture);
                            let render_center = canvas_center + location.to_vector() * scale;
                            let render_location = render_center
                                - sprite.texture.size().to_f32().cast_unit().to_vector() / 2.;
                            sprite
                                .render_at(
                                    context.scene(),
                                    render_location,
                                    SpriteRotation::around_center(orientation),
                                )
                                .await;

                            if let Some(pilot) = self
                                .api_client
                                .pilot_information(&ship.pilot_id, &self.api_client)
                                .await
                            {
                                let text = Text::span(
                                    &pilot.name,
                                    Style::new()
                                        .with(FontFamily::from("Orbitron"))
                                        .with(FontSize::new(18.))
                                        .with(ForegroundColor(
                                            Color::new(1.0, 1.0, 1.0, 1.0).into(),
                                        ))
                                        .effective_style(context.scene())
                                        .await,
                                );
                                let prepared = text.wrap(context.scene(), TextWrap::NoWrap).await?;
                                let text_size = prepared.size().await;
                                let text_size = text_size / context.scene().scale_factor().await;
                                let text_left = render_center.x - text_size.width / 2.;
                                let ship_longest_side = sprite
                                    .texture
                                    .size()
                                    .width
                                    .max(sprite.texture.size().height)
                                    as f32;
                                let max_ship_size = ((ship_longest_side / 2.).powf(2.) * 2.).sqrt();
                                let nameplate_top = (render_center.y + max_ship_size).ceil();
                                const NAMEPLATE_PADDING: f32 = 5.;
                                Shape::rect(Rect::new(
                                    Point::new(text_left - NAMEPLATE_PADDING, nameplate_top),
                                    Size::new(
                                        text_size.width + NAMEPLATE_PADDING * 2.,
                                        text_size.height + NAMEPLATE_PADDING * 2.,
                                    ),
                                ))
                                .fill(Fill::new(Color::new(0.87, 0.03, 0.45, 1.)))
                                .render_at(Point::zero(), context.scene())
                                .await;

                                let text_top = nameplate_top + NAMEPLATE_PADDING;
                                prepared
                                    .render(context.scene(), Point::new(text_left, text_top), true)
                                    .await?;
                            }
                        }
                    }
                }
            }
        } else {
            Shape::rect(layout.inner_bounds())
                .fill(Fill::new(Color::BLACK))
                .render_at(Default::default(), context.scene())
                .await;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    ViewSolarSystem(SystemId),
    SpaceUpdate {
        timestamp: f64,
        location: navigation::Pilot,
        action: navigation::Action,
        ships: Vec<navigation::Ship>,
    },
}

#[async_trait]
impl InteractiveComponent for SolarSystemCanvas {
    type Message = ();

    type Command = Command;

    type Event = ();

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        match command {
            Command::ViewSolarSystem(id) => self.view_solar_system(id, context).await,
            Command::SpaceUpdate {
                timestamp,
                location,
                ships,
                ..
            } => {
                self.simulator
                    .update(ships, location.system, timestamp, current_timestamp());
                Ok(())
            }
        }
    }
}

impl SolarSystemCanvas {
    pub fn new(api_client: CosmicVergeClient) -> Self {
        Self {
            api_client,
            solar_system: None,
            look_at: Default::default(),
            zoom: 1.,
            simulator: Default::default(),
            ship_images: Default::default(),
        }
    }

    async fn view_solar_system(
        &mut self,
        id: SystemId,
        context: &mut Context,
    ) -> KludgineResult<()> {
        if let Some(cache) = &self.solar_system {
            if cache.solar_system.id == id {
                // Same system
                return Ok(());
            }
        }

        self.zoom = 1.;
        self.look_at = Point::default();
        let solar_system = universe().get(&id);
        let backdrop = match solar_system.background {
            Some(url) => Some(CachedImage::new(url).await.map_err(anyhow::Error::from)?),
            None => None,
        };
        let mut cache = SolarSystemCache {
            backdrop,
            solar_system,
            object_images: Default::default(),
        };
        for object in solar_system.locations.values() {
            let image = CachedImage::new(object.image_url())
                .await
                // TODO this is ugly, Kludgine should offer this conversion automatically if possible
                .map_err(anyhow::Error::from)?;
            cache.object_images.insert(object.id.id(), image);
        }
        self.solar_system = Some(cache);

        context.set_needs_redraw().await;

        Ok(())
    }
}

impl CanvasScalable for SolarSystemCanvas {
    fn scale<Unit>(&self) -> Scale<f32, Unit, Scaled> {
        Scale::new(self.zoom)
    }

    fn look_at<Unit>(&self) -> Point<f32, Unit> {
        self.look_at.cast_unit()
    }
}

pub trait CanvasScalable {
    fn scale<Unit>(&self) -> Scale<f32, Unit, Scaled>;
    fn look_at<Unit>(&self) -> Point<f32, Unit>;

    fn canvas_center(&self, size: Size<f32, Scaled>) -> Point<f32, Scaled> {
        (size.to_vector() / 2.).to_point()
    }

    fn convert_canvas_to_world_with_scale<Unit>(
        &self,
        canvas_location: Point<f32, Scaled>,
        scale: Scale<f32, Unit, Scaled>,
        size: Size<f32, Scaled>,
    ) -> Point<f32, Unit> {
        let relative_location = canvas_location - self.canvas_center(size);
        self.look_at() + relative_location / scale
    }

    fn convert_canvas_to_world<Unit>(
        &self,
        canvas_location: Point<f32, Scaled>,
        size: Size<f32, Scaled>,
    ) -> Point<f32, Unit> {
        self.convert_canvas_to_world_with_scale(canvas_location, self.scale(), size)
    }

    fn convert_world_to_canvas_with_scale<Unit>(
        &self,
        world_location: Point<f32, Unit>,
        scale: Scale<f32, Unit, Scaled>,
        size: Size<f32, Scaled>,
    ) -> Point<f32, Scaled> {
        let relative_location = world_location - self.look_at().to_vector();
        self.canvas_center(size) + relative_location.to_vector() * scale
    }

    fn calculate_zoom<Unit>(
        &self,
        fraction: f32,
        focus: Point<f32, Scaled>,
        size: Size<f32, Scaled>,
    ) -> (f32, Point<f32, Unit>) {
        let scale = self.scale();
        let new_zoom = scale.get() + scale.get() * fraction;
        let new_zoom = new_zoom.min(10.).max(0.1);
        let new_scale = Scale::<f32, Unit, Scaled>::new(new_zoom);

        let center = self.canvas_center(size);
        let focus_offset = focus.to_vector() - center.to_vector();
        let focus_solar = self.look_at() + focus_offset / scale;

        let new_focus_location =
            self.convert_world_to_canvas_with_scale(focus_solar, new_scale, size);
        let pixel_delta = new_focus_location.to_vector() - focus.to_vector();
        let solar_delta = pixel_delta / new_scale;

        (new_zoom, self.look_at() + solar_delta)
    }
}

fn current_timestamp() -> f64 {
    let now = Utc::now();
    now.timestamp() as f64 + now.timestamp_subsec_millis() as f64 / 1000.
}
