use cosmicverge_shared::{
    euclid::{Angle, Point2D},
    protocol::navigation,
    solar_system_simulation::Simulation,
    solar_systems::{Solar, SolarSystemId},
};

#[derive(Default)]
pub struct Simulator {
    pub simulation_system: Option<SolarSystemId>,
    pub simulation: Option<Simulation>,
    pub server_round_trip_avg: Option<f64>,
    last_physics_update: Option<f64>,
}

impl Simulator {
    pub fn update(
        &mut self,
        ships: Vec<navigation::Ship>,
        solar_system: SolarSystemId,
        timestamp: f64,
        now: f64,
    ) {
        self.simulation_system = Some(solar_system);

        let current_simulation_timestamp =
            self.simulation.as_ref().map_or(timestamp, |s| s.timestamp);
        let mut simulation = Simulation::new(solar_system, timestamp);
        simulation.add_ships(ships);

        // Since the simulation keeps track of how much time it thinks has elapsed, we know how much time
        // has elapsed since this calculation was made and can accurately update. We also need to factor
        // in how much time we think it's been since the packet was sent.
        // However, there are situations where we just can't do anything to catch the simulation up. Right now
        // ships just jump to their new location immediately, but we should introduce smoothing somehow eventually
        let estimated_server_oneway = self
            .server_round_trip_avg
            .map(|rtt| rtt / 2.)
            .unwrap_or_default();
        let simulation_catchup = current_simulation_timestamp - estimated_server_oneway - timestamp;
        if simulation_catchup > 0. {
            simulation.step(simulation_catchup as f32);
        }
        self.simulation = Some(simulation);
        self.last_physics_update = Some(now);
    }

    pub fn step(&mut self, now: f64) {
        if let Some(last_physics_timestamp) = self.last_physics_update {
            if let Some(simulation) = &mut self.simulation {
                let elapsed = now - last_physics_timestamp;
                if elapsed >= 0. {
                    simulation.step(elapsed as f32);
                }
            }
        }
        self.last_physics_update = Some(now);
    }

    pub fn pilot_locations(&self) -> Vec<(navigation::Ship, Point2D<f32, Solar>, Angle<f32>)> {
        if let Some(simulation) = &self.simulation {
            simulation
                .all_ships()
                .cloned()
                .map(move |ship| {
                    let location = ship.physics.location;
                    let orientation = ship.physics.rotation;

                    (ship, location, orientation)
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}
