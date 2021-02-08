use std::collections::HashMap;
use std::f32::consts::PI;

use euclid::{Point2D, Vector2D, Angle, approxeq::ApproxEq};

use crate::protocol::{FlightPlan, PilotedShip, PilotingAction, SolarSystemLocation, FlightPlanManeuver};
use crate::solar_systems::{Solar, universe};
use crate::ships::{ShipSpecification, hangar};

#[derive(Default)]
pub struct SolarSystemSimulation {
    ships: HashMap<i64, PilotedShip>,
}

impl SolarSystemSimulation {
    pub fn step(&mut self, duration: f32) {
        for (_, ship) in self.ships.iter_mut() {
            // carry out the existing plan first, because the ship's action shouldn't take effect until the next iteration
            ship.execute_flight_plan_if_needed(duration);

            let create_plan = match &ship.physics.flight_plan {
                Some(plan) => plan.made_for != ship.action,
                None => true,
            };

            if create_plan {
                ship.create_flight_plan();
            }
        }
    }

    pub fn add_ships<I: Iterator<Item=PilotedShip>>(&mut self, ships: I) {
        for ship in ships {
            self.ships.insert(ship.pilot_id, ship);
        }
    }

    pub fn get_ship_info(&self) -> Vec<PilotedShip> {
        self.ships.values().cloned().collect()
    }
}

fn vector_to_angle(vec: Vector2D<f32, Solar>) -> Angle<f32> {
    Angle::radians(vec.y.atan2(vec.x))
}

impl crate::protocol::PilotedShip {
    fn create_flight_plan(&mut self) {
        let mut plan = FlightPlan::new(self);
        match &self.action {
            PilotingAction::NavigateTo(location) => {
                let destination = match location.location {
                    SolarSystemLocation::InSpace(location) => location,
                    SolarSystemLocation::Docked(object_id) => {
                        let system = universe().get(&location.system);
                        system.locations[&object_id].location
                    }
                };

                self.plan_flight_to(&mut plan, destination);
            }
            PilotingAction::Idle => {
                self.append_stop_plan_if_needed(&mut plan);
            }
        }
        self.physics.flight_plan = Some(plan);
    }

    fn append_stop_plan_if_needed(&self, plan: &mut FlightPlan) {
        if let Some(normalized_velocity) = self.physics.linear_velocity.try_normalize() {
            // Turn the ship around
            let negative_velocity_angle = (vector_to_angle(normalized_velocity) + Angle::pi()).signed();
            let amount_to_turn = self.physics.rotation.angle_to(negative_velocity_angle);
            let rotation_time = amount_to_turn.radians.abs() / self.max_turning_radians_per_sec();
            let position_after_rotation = plan.last_location_for(self) + self.physics.linear_velocity * rotation_time;
            plan.maneuvers.push(FlightPlanManeuver {
                duration: rotation_time,
                target: position_after_rotation,
                target_rotation: negative_velocity_angle,
                target_velocity: self.physics.linear_velocity,
            });

            // Decelerate
            let velocity_magnitude = self.physics.linear_velocity.length();
            let time_to_stop = velocity_magnitude / self.max_acceleration();
            let traveled_distance = velocity_magnitude * time_to_stop + 0.5 * -self.max_acceleration() * time_to_stop * time_to_stop;
            let final_location = position_after_rotation + normalized_velocity * traveled_distance;
            plan.maneuvers.push(FlightPlanManeuver {
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

    fn plan_flight_to(&mut self, plan: &mut FlightPlan, destination: Point2D<f32, Solar>) {
        // We have to turn around to stop whatever motion we make
        let time_to_turn_around = self.time_to_turn_around();

        // For now, let's just stop motion, then go to the destination
        self.append_stop_plan_if_needed(plan);

        let start = plan.last_location_for(self);
        let destination_delta = destination.to_vector() - start.to_vector();

        // If we can't normalize, it means the distance is 0., so our goal is already met
        if let Some(normalized_travel_direction) = destination_delta.try_normalize()
        {
            let distance_to_destination = destination_delta.length();
            let angle_to_destination = vector_to_angle(destination_delta);

            let time_to_align = plan.last_rotation_for(self).angle_to(angle_to_destination).signed().radians.abs() / self.max_turning_radians_per_sec();
            plan.maneuvers.push(FlightPlanManeuver {
                duration: time_to_align,
                target: start,
                target_rotation: angle_to_destination,
                target_velocity: Vector2D::zero(),
            });

            let time_to_midpoint = time_to_travel_distance(0., self.max_acceleration(), distance_to_destination / 2.);
            let acceleration_time = time_to_midpoint - time_to_turn_around / 2.;
            let final_velocity = normalized_travel_direction * acceleration_time * self.max_acceleration();
            // This is a shortcut for 1/2 at^2, because we already calculated at in the previous line, the remaining values are 0.5 and t
            let location_after_acceleration = plan.last_location_for(self) + final_velocity * 0.5 * acceleration_time;
            plan.maneuvers.push(FlightPlanManeuver {
                duration: acceleration_time,
                target: location_after_acceleration,
                target_rotation: plan.last_rotation_for(self),
                target_velocity: final_velocity,
            });

            // Turn around
            let deceleration_angle = (angle_to_destination + Angle::pi()).signed();
            let location_after_drifting = location_after_acceleration + final_velocity * time_to_turn_around;
            plan.maneuvers.push(FlightPlanManeuver {
                duration: time_to_turn_around,
                target: location_after_drifting,
                target_rotation: deceleration_angle,
                target_velocity: final_velocity,
            });

            // We can cheat on this last one, because we know the destination and can just assume our math was good enough at this point
            plan.maneuvers.push(FlightPlanManeuver {
                duration: acceleration_time,
                target: destination,
                target_rotation: deceleration_angle,
                target_velocity: Vector2D::zero(),
            });
        }
    }

    fn max_acceleration(&self) -> f32 {
        self.specification().acceleration()
    }

    fn execute_flight_plan_if_needed(&mut self, duration: f32) {
        if let Some(plan) = &mut self.physics.flight_plan {
            if let Some(update) = execute_flight_plan(plan, duration) {
                self.physics.location = update.location;
                self.physics.rotation = update.orientation;
                self.physics.linear_velocity = update.velocity;
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
        error!("invalid solution found for time_to_travel_distance({}, {}, {}) -> {} or {}", initial_velocity, acceleration, distance, solution_a, solution_b);
        0.
    }
}

fn execute_flight_plan(plan: &mut FlightPlan, mut duration: f32) -> Option<FlightUpdate> {
    let mut last_update = None;
    while let Some(maneuver) = plan.maneuvers.first_mut() {
        let total_elapsed = plan.elapsed_in_current_maneuver + duration;
        if plan.elapsed_in_current_maneuver < maneuver.duration {
            // Partial maneuver
            plan.elapsed_in_current_maneuver = total_elapsed;
            let percent_complete = plan.elapsed_in_current_maneuver / maneuver.duration;
            let movement_interpolation_mode= if plan.initial_velocity.approx_eq(&maneuver.target_velocity) {
                InterpolationMode::Linear
            } else if plan.initial_velocity.square_length() > maneuver.target_velocity.square_length() {
                InterpolationMode::ExponentialOut
            } else {
              InterpolationMode::ExponentialIn
            };
            last_update = Some(FlightUpdate {
                location: interpolate_value(plan.initial_position.to_vector(), maneuver.target.to_vector(), percent_complete, movement_interpolation_mode).to_point(),
                orientation: interpolate_value(plan.initial_orientation, maneuver.target_rotation, percent_complete, InterpolationMode::Linear),
                velocity: interpolate_value(plan.initial_velocity, maneuver.target_velocity, percent_complete, movement_interpolation_mode),
            });
            break;
        } else {
            // Completed this maneuver
            duration -= maneuver.duration - plan.elapsed_in_current_maneuver;
            last_update = Some(
                FlightUpdate {
                    location: maneuver.target,
                    velocity: maneuver.target_velocity,
                    orientation: maneuver.target_rotation,
                }
            );
            plan.maneuvers.remove(0);

            let FlightUpdate { location, orientation, velocity } = last_update.unwrap();
            plan.initial_orientation = orientation;
            plan.initial_velocity = velocity;
            plan.initial_position = location;
            plan.elapsed_in_current_maneuver = 0.;
        }
    }

    last_update
}

#[derive(Clone, Copy)]
struct FlightUpdate {
    location: Point2D<f32, Solar>,
    orientation: Angle<f32>,
    velocity: Vector2D<f32, Solar>,
}

#[derive(Clone, Copy, Debug)]
enum InterpolationMode {
    Linear,
    ExponentialIn,
    ExponentialOut,
}

fn interpolate_value<T>(original: T, target: T, percent: f32, mode: InterpolationMode) -> T
where T: std::ops::Add<T, Output=T> + std::ops::Sub<T, Output=T> + std::ops::Mul<f32, Output=T> + Copy {
    match mode {
        InterpolationMode::Linear => {
            original + (target - original) * percent
        }
        InterpolationMode::ExponentialIn => {
            original + (target - original) * (percent * percent)
        }
        InterpolationMode::ExponentialOut => {
            let one_minus_percent = 1. - percent;
            let difference = original - target;
            target + difference * one_minus_percent * one_minus_percent
        }
    }
}