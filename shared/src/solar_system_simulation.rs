use std::{collections::HashMap, f32::consts::PI};

use euclid::{approxeq::ApproxEq, Angle, Point2D, Vector2D};
use rand::{thread_rng, Rng};

use crate::{
    protocol::{
        Action, FlightPlan, Id, Maneuver, ManeuverKind, PilotedShip, ShipEffect,
        SolarSystemLocation,
    },
    ships::{hangar, ShipSpecification},
    solar_systems::{universe, Solar, SolarSystemId},
};

pub struct SolarSystemSimulation {
    pub timestamp: f64,
    system: SolarSystemId,
    ships: HashMap<Id, PilotedShip>,
}

impl SolarSystemSimulation {
    #[must_use]
    pub fn new(system: SolarSystemId, timestamp: f64) -> Self {
        Self {
            system,
            timestamp,
            ships: HashMap::new(),
        }
    }

    pub fn step(&mut self, duration: f32) {
        for ship in self.ships.values_mut() {
            // carry out the existing plan first, because the ship's action shouldn't take effect until the next iteration
            ship.execute_flight_plan_if_needed(duration);

            let create_plan = match &ship.physics.flight_plan {
                Some(plan) => plan.made_for != ship.action,
                None => true,
            };

            if create_plan {
                ship.create_flight_plan(self.system);
            }
        }

        self.timestamp += f64::from(duration);
    }

    pub fn add_ships<I: IntoIterator<Item = PilotedShip>>(&mut self, ships: I) {
        for ship in ships {
            self.ships.insert(ship.pilot_id, ship);
        }
    }

    pub fn all_ships(&self) -> impl Iterator<Item = &PilotedShip> {
        self.ships.values()
    }

    #[must_use]
    pub fn lookup_ship(&self, pilot_id: Id) -> Option<&PilotedShip> {
        self.ships.get(&pilot_id)
    }
}

fn vector_to_angle<T>(vec: Vector2D<f32, T>) -> Angle<f32> {
    Angle::radians(vec.y.atan2(vec.x))
}

impl crate::protocol::PilotedShip {
    fn create_flight_plan(&mut self, current_system: SolarSystemId) {
        let mut plan = FlightPlan::new(self, current_system);
        match &self.action {
            Action::NavigateTo(destination) => {
                let system = destination.system;
                let location = match destination.location {
                    SolarSystemLocation::InSpace(location) => location,
                    SolarSystemLocation::Docked(object_id) => {
                        let system = universe().orbits_for(destination.system);

                        system[&object_id]
                    }
                };

                self.plan_flight_to(&mut plan, system, location);
            }
            Action::Idle => {
                self.append_stop_plan_if_needed(&mut plan);
            }
        }
        self.physics.flight_plan = Some(plan);
    }

    fn append_stop_plan_if_needed(&self, plan: &mut FlightPlan) {
        if let Some(normalized_velocity) = plan.last_velocity_for(self).try_normalize() {
            let current_system = plan.last_system();
            // Turn the ship around
            let negative_velocity_angle =
                (vector_to_angle(normalized_velocity) + Angle::pi()).signed();
            let amount_to_turn = self.physics.rotation.angle_to(negative_velocity_angle);
            let rotation_time = amount_to_turn.radians.abs() / self.max_turning_radians_per_sec();
            let position_after_rotation =
                plan.last_location_for(self) + self.physics.linear_velocity * rotation_time;
            plan.maneuvers.push(Maneuver {
                kind: ManeuverKind::Movement,
                system: current_system,
                duration: rotation_time,
                target: position_after_rotation,
                target_rotation: negative_velocity_angle,
                target_velocity: self.physics.linear_velocity,
            });

            // Decelerate
            let velocity_magnitude = self.physics.linear_velocity.length();
            let time_to_stop = velocity_magnitude / self.max_acceleration();
            let traveled_distance = velocity_magnitude.mul_add(
                time_to_stop,
                0.5 * -self.max_acceleration() * time_to_stop * time_to_stop,
            );
            let final_location = position_after_rotation + normalized_velocity * traveled_distance;
            plan.maneuvers.push(Maneuver {
                kind: ManeuverKind::Movement,
                system: current_system,
                duration: time_to_stop,
                target: final_location,
                target_rotation: negative_velocity_angle,
                target_velocity: Vector2D::zero(),
            });
        }
    }

    fn time_to_turn_around(&self) -> f32 {
        PI / self.max_turning_radians_per_sec()
    }

    fn append_alignment_towards(&mut self, plan: &mut FlightPlan, angle: Angle<f32>) {
        let time_to_align = plan
            .last_rotation_for(self)
            .angle_to(angle)
            .signed()
            .radians
            .abs()
            / self.max_turning_radians_per_sec();
        let velocity = plan.last_velocity_for(self);
        let location_after_drifting = plan.last_location_for(self) + velocity * time_to_align;
        plan.maneuvers.push(Maneuver {
            kind: ManeuverKind::Movement,
            system: plan.last_system(),
            duration: time_to_align,
            target: location_after_drifting,
            target_rotation: angle,
            target_velocity: plan.last_velocity_for(self),
        });
    }

    fn plan_flight_to(
        &mut self,
        plan: &mut FlightPlan,
        destination_system: SolarSystemId,
        location: Point2D<f32, Solar>,
    ) {
        if plan.initial_system == destination_system {
            self.plan_interstellar_flight(plan, location)
        } else {
            self.append_stop_plan_if_needed(plan);

            let current_system = universe().get(&plan.initial_system);
            let destination = universe().get(&destination_system);

            let distance_to_destination = destination.galaxy_location.to_vector()
                - current_system.galaxy_location.to_vector();
            let angle_to_destination = vector_to_angle(distance_to_destination);
            self.append_alignment_towards(plan, angle_to_destination);

            let sun_id = destination.locations_by_owners[&None][0];
            let sun = &destination.locations[&sun_id];

            let mut rng = thread_rng();
            plan.maneuvers.push(Maneuver {
                kind: ManeuverKind::Jump,
                system: destination_system,
                duration: 1.0,
                target: Point2D::new(
                    rng.gen::<f32>() * sun.size * 2.,
                    rng.gen::<f32>() * sun.size * 2.,
                ),
                target_rotation: Angle::default(),
                target_velocity: Vector2D::default(),
            });

            self.plan_interstellar_flight(plan, location);
        }
    }

    fn plan_interstellar_flight(&mut self, plan: &mut FlightPlan, location: Point2D<f32, Solar>) {
        // We have to turn around to stop whatever motion we make
        let time_to_turn_around = self.time_to_turn_around();

        // For now, let's just stop motion, then go to the destination
        self.append_stop_plan_if_needed(plan);

        let current_system = plan.last_system();
        let start = plan.last_location_for(self);
        let destination_delta = location.to_vector() - start.to_vector();

        // If we can't normalize, it means the distance is 0., so our goal is already met
        if let Some(normalized_travel_direction) = destination_delta.try_normalize() {
            let distance_to_destination = destination_delta.length();
            let angle_to_destination = vector_to_angle(destination_delta);

            self.append_alignment_towards(plan, angle_to_destination);

            let time_to_midpoint =
                time_to_travel_distance(0., self.max_acceleration(), distance_to_destination / 2.);
            let acceleration_time = time_to_midpoint - time_to_turn_around / 2.;
            let final_velocity =
                normalized_travel_direction * acceleration_time * self.max_acceleration();
            // This is a shortcut for 1/2 at^2, because we already calculated at in the previous line, the remaining values are 0.5 and t
            let location_after_acceleration =
                plan.last_location_for(self) + final_velocity * 0.5 * acceleration_time;
            plan.maneuvers.push(Maneuver {
                kind: ManeuverKind::Movement,
                system: current_system,
                duration: acceleration_time,
                target: location_after_acceleration,
                target_rotation: plan.last_rotation_for(self),
                target_velocity: final_velocity,
            });

            // Turn around
            let deceleration_angle = (angle_to_destination + Angle::pi()).signed();
            self.append_alignment_towards(plan, deceleration_angle);

            // We can cheat on this last one, because we know the destination and can just assume our math was good enough at this point
            plan.maneuvers.push(Maneuver {
                kind: ManeuverKind::Movement,
                system: current_system,
                duration: acceleration_time,
                target: location,
                target_rotation: deceleration_angle,
                target_velocity: Vector2D::zero(),
            });
        }
    }

    fn max_acceleration(&self) -> f32 {
        self.specification().acceleration()
    }

    /// Executes the plight plan. If the execution switches solar systems, the new system is returned
    fn execute_flight_plan_if_needed(&mut self, duration: f32) {
        if let Some(plan) = &mut self.physics.flight_plan {
            if let Some(update) = execute_flight_plan(plan, duration) {
                self.physics.location = update.location;
                self.physics.rotation = update.orientation;
                self.physics.linear_velocity = update.velocity;
                self.physics.system = update.system;
            }
        }
    }

    fn max_turning_radians_per_sec(&self) -> f32 {
        self.specification().rotation
    }

    fn specification(&self) -> &'static ShipSpecification {
        hangar().load(&self.ship.ship)
    }
}

//     -b +/- sqrt(b^2 - 4ac)
// t = ----------------------
//               2a
// where
//   a = 1/2 acceleration
//   b = initial_velocity
//   c = -distance
fn time_to_travel_distance(initial_velocity: f32, acceleration: f32, distance: f32) -> f32 {
    if approx::relative_eq!(distance, 0.) {
        return 0.;
    }

    let a = acceleration / 2.;
    let b = initial_velocity;
    let c = -distance;

    let negative_b = -b;
    let sqrt_term = (b * b - 4. * a * c).sqrt();
    let numerator_a = negative_b + sqrt_term;
    let numerator_b = negative_b - sqrt_term;

    let denominator = 2. * a;

    let solution_a = numerator_a / denominator;
    let solution_b = numerator_b / denominator;

    if solution_a > 0. {
        solution_a
    } else if solution_b > 0. {
        solution_b
    } else {
        log::error!(
            "invalid solution found for time_to_travel_distance({}, {}, {}) -> {} or {}",
            initial_velocity,
            acceleration,
            distance,
            solution_a,
            solution_b
        );
        0.
    }
}

fn calculate_flight_update(plan: &FlightPlan) -> Option<FlightUpdate> {
    plan.maneuvers.first().map(|maneuver| {
        let percent_complete = plan.elapsed_in_current_maneuver / maneuver.duration;
        match maneuver.kind {
            // For jumps, we don't want to alter the values, just pass the effect of "jumping"
            ManeuverKind::Jump => FlightUpdate {
                system: plan.initial_system,
                location: plan.initial_position,
                orientation: plan.initial_orientation,
                velocity: plan.initial_velocity,
                effect: Some(ShipEffect::Jumping),
            },

            // Movement we interpolate
            ManeuverKind::Movement => {
                let (effect, movement_interpolation_mode) =
                    if plan.initial_velocity.approx_eq(&maneuver.target_velocity) {
                        (None, InterpolationMode::Linear)
                    } else if plan.initial_velocity.square_length()
                        > maneuver.target_velocity.square_length()
                    {
                        (
                            Some(ShipEffect::Thrusting),
                            InterpolationMode::ExponentialOut,
                        )
                    } else {
                        (
                            Some(ShipEffect::Thrusting),
                            InterpolationMode::ExponentialIn,
                        )
                    };
                FlightUpdate {
                    system: maneuver.system,
                    effect,
                    location: interpolate_value(
                        plan.initial_position.to_vector(),
                        maneuver.target.to_vector(),
                        percent_complete,
                        movement_interpolation_mode,
                    )
                    .to_point(),
                    orientation: interpolate_value(
                        plan.initial_orientation,
                        maneuver.target_rotation,
                        percent_complete,
                        InterpolationMode::Linear,
                    ),
                    velocity: interpolate_value(
                        plan.initial_velocity,
                        maneuver.target_velocity,
                        percent_complete,
                        movement_interpolation_mode,
                    ),
                }
            }
        }
    })
}

fn execute_flight_plan(plan: &mut FlightPlan, mut duration: f32) -> Option<FlightUpdate> {
    let mut last_update = None;
    while let Some(maneuver) = plan.maneuvers.first_mut() {
        let total_elapsed = plan.elapsed_in_current_maneuver + duration;
        if plan.elapsed_in_current_maneuver < maneuver.duration {
            // Partial maneuver
            plan.elapsed_in_current_maneuver = total_elapsed;
            return calculate_flight_update(plan);
        }
        // Completed this maneuver
        duration -= maneuver.duration - plan.elapsed_in_current_maneuver;
        let update = FlightUpdate {
            system: maneuver.system,
            location: maneuver.target,
            velocity: maneuver.target_velocity,
            orientation: maneuver.target_rotation,
            effect: None,
        };
        last_update = Some(update.clone());

        let jump = matches!(maneuver.kind, ManeuverKind::Jump);
        plan.maneuvers.remove(0);

        plan.initial_system = update.system;
        plan.initial_orientation = update.orientation;
        plan.initial_velocity = update.velocity;
        plan.initial_position = update.location;
        plan.elapsed_in_current_maneuver = 0.;

        // Jumps should be returned right away after they're removed
        // On the server, the new solar system will process the next flight plan steps
        // On the client, the simulation will remove ships that are no longer present.
        if jump {
            break;
        }
    }

    last_update
}

#[derive(Clone)]
struct FlightUpdate {
    system: SolarSystemId,
    location: Point2D<f32, Solar>,
    orientation: Angle<f32>,
    velocity: Vector2D<f32, Solar>,
    effect: Option<ShipEffect>,
}

#[derive(Clone, Copy, Debug)]
pub enum InterpolationMode {
    Linear,
    ExponentialIn,
    ExponentialOut,
}

pub fn interpolate_value<T>(original: T, target: T, percent: f32, mode: InterpolationMode) -> T
where
    T: std::ops::Add<T, Output = T>
        + std::ops::Sub<T, Output = T>
        + std::ops::Mul<f32, Output = T>
        + Copy,
{
    match mode {
        InterpolationMode::Linear => original + (target - original) * percent,
        InterpolationMode::ExponentialIn => original + (target - original) * percent * percent,
        InterpolationMode::ExponentialOut => {
            let one_minus_percent = 1. - percent;
            let difference = original - target;
            target + difference * one_minus_percent * one_minus_percent
        }
    }
}
