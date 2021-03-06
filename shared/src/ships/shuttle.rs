use std::f32::consts::PI;

use super::{Id, ShipSpecification};

pub fn ship() -> ShipSpecification {
    ShipSpecification {
        id: Id::Shuttle,
        image: "/assets/programmerart/pinkship.png",
        mass: 10.,
        thruster_force: 50.,
        rotation: PI / 3.,
    }
}
