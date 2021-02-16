use cosmicverge_shared::{
    euclid::{Angle, Point2D},
    protocol::{PilotId, PilotedShip},
    solar_system_simulation::{interpolate_value, InterpolationMode, SolarSystemSimulation},
    solar_systems::{Solar, SolarSystemId},
};

use super::space2d::SHIP_TWEEN_DURATION;

#[derive(Default)]
pub struct Simulator {
    pub simulation_system: Option<SolarSystemId>,
    pub simulation: Option<SolarSystemSimulation>,
    pub tweened_simulation: Option<TweenedSimulation>,
    last_physics_update: Option<f64>,
}

pub struct TweenedSimulation {
    pub simulation: SolarSystemSimulation,
    pub start_timestamp: f64,
}

impl Simulator {
    pub fn update(&mut self, ships: Vec<PilotedShip>, solar_system: SolarSystemId, timestamp: f64) {
        self.simulation_system = Some(solar_system);

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

    pub fn pilot_location(&self, pilot_id: &PilotId) -> Option<Point2D<f32, Solar>> {
        if let Some(simulation) = &self.simulation {
            if let Some(ship) = simulation.lookup_ship(pilot_id) {
                return Some(ship.physics.location);
            }
        }

        None
    }

    pub fn step(&mut self, now: f64) {
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
            }
        }
        self.last_physics_update = Some(now);
    }

    pub fn pilot_locations(&self) -> Vec<(PilotedShip, Point2D<f32, Solar>, Angle<f32>)> {
        if let Some(simulation) = &self.simulation {
            simulation
                .all_ships()
                .cloned()
                .map(move |ship| {
                    let mut location = ship.physics.location;
                    let mut orientation = ship.physics.rotation;
                    if let Some(tweened) = &self.tweened_simulation {
                        if let Some(tweened_ship) = tweened.simulation.lookup_ship(&ship.pilot_id) {
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

                    (ship, location, orientation)
                })
                .collect()
        } else {
            Default::default()
        }
    }
}
