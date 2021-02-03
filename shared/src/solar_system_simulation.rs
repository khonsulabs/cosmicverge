use std::collections::HashMap;

use euclid::{Point2D, Vector2D};
use rapier2d_f64::{
    dynamics::{IntegrationParameters, JointSet, RigidBodyBuilder, RigidBodyHandle, RigidBodySet},
    geometry::{BroadPhase, ColliderSet, NarrowPhase},
    na::Vector2,
    pipeline::PhysicsPipeline,
};

use crate::{
    protocol::{PilotPhysics, PilotedShip, PilotingAction, SolarSystemLocation},
    solar_systems::universe,
};

pub struct SolarSystemSimulation {
    pipeline: PhysicsPipeline,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    joints: JointSet,
    ships: HashMap<RigidBodyHandle, PilotedShip>,
}

impl Default for SolarSystemSimulation {
    fn default() -> Self {
        let pipeline = PhysicsPipeline::new();
        let broad_phase = BroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let bodies = RigidBodySet::new();
        let colliders = ColliderSet::new();
        let joints = JointSet::new();
        let ships = HashMap::new();

        Self {
            pipeline,
            broad_phase,
            narrow_phase,
            bodies,
            colliders,
            joints,
            ships,
        }
    }
}

impl SolarSystemSimulation {
    pub fn step(&mut self, duration: f64) {
        for (handle, ship) in self.ships.iter() {
            if let PilotingAction::NavigateTo(location) = &ship.action {
                // TODO once we have multiple systems, this needs to know what the current system is to be able to know what a ship should do

                let destination = match location.location {
                    SolarSystemLocation::InSpace(location) => location,
                    SolarSystemLocation::Docked(object_id) => {
                        let system = universe().get(&location.system);
                        system.locations[&object_id].location
                    }
                };

                let body = self.bodies.get_mut(*handle).unwrap();

                let body_location = body.position().translation;
                let body_location = euclid::Point2D::new(body_location.x, body_location.y);
                let delta = destination.to_vector() - body_location.to_vector();
                let delta = delta.normalize();
                let force = Vector2::new(delta.x, delta.y);

                info!("Applying force {},{}", force.x, force.y);
                body.apply_force(force, true);
            }
        }

        let integration_parameters = IntegrationParameters {
            dt: duration,
            ..Default::default()
        };
        self.pipeline.step(
            &Vector2::default(),
            &integration_parameters,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joints,
            None,
            None,
            &(),
        );
    }

    pub fn add_ships<I: Iterator<Item = PilotedShip>>(&mut self, ships: I) {
        for ship in ships {
            let body = RigidBodyBuilder::new_dynamic()
                .translation(ship.location.x, ship.location.y)
                .rotation(ship.physics.rotation)
                .linvel(
                    ship.physics.linear_velocity.x,
                    ship.physics.linear_velocity.y,
                )
                .angvel(ship.physics.angular_velocity)
                .can_sleep(true)
                .mass(10.)
                .build();
            let handle = self.bodies.insert(body);
            self.ships.insert(handle, ship);
        }
    }

    pub fn get_ship_info(&self) -> Vec<PilotedShip> {
        let mut ships = Vec::new();

        for (handle, original_ship) in self.ships.iter() {
            let body = self.bodies.get(*handle).unwrap();
            let location = body.position().translation;
            let location = Point2D::new(location.x, location.y);
            let linear_velocity = body.linvel();
            let linear_velocity = Vector2D::new(linear_velocity.x, linear_velocity.y);
            let angular_velocity = body.angvel();

            ships.push(PilotedShip {
                pilot_id: original_ship.pilot_id,
                ship: original_ship.ship.clone(),
                location,
                action: original_ship.action.clone(),
                physics: PilotPhysics {
                    rotation: body.position().rotation.angle(),
                    linear_velocity,
                    angular_velocity,
                },
            });
        }

        ships
    }
}
