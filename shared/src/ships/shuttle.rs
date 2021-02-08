use super::{ShipId, ShipSpecification};

pub fn ship() -> ShipSpecification {
    ShipSpecification {
        id: ShipId::Shuttle,
        image: "/programmerart/pinkship.png",
        mass: 10.,
        thruster_force: 100.,
        rotor_force: 5.,
        mass_radius: 1.,
    }
}
