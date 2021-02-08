use super::{ShipId, ShipSpecification};
use std::f32::consts::PI;

pub fn ship() -> ShipSpecification {
    ShipSpecification {
        id: ShipId::Shuttle,
        image: "/programmerart/pinkship.png",
        mass: 10.,
        thruster_force: 50.,
        rotation: PI / 3.,
    }
}
