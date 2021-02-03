use super::{ShipId, ShipSpecification};

pub fn ship() -> ShipSpecification {
    ShipSpecification {
        id: ShipId::Shuttle,
        image: "/programmerart/pinkship.png",
    }
}
