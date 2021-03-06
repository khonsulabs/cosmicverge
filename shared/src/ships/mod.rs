use std::collections::HashMap;

use num_derive::{FromPrimitive, ToPrimitive};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use crate::solar_systems::Named;

mod shuttle;

pub struct Hangar {
    ships: HashMap<Id, ShipSpecification>,
}

pub fn hangar() -> &'static Hangar {
    static SHARED_HANGAR: OnceCell<Hangar> = OnceCell::new();
    SHARED_HANGAR.get_or_init(Hangar::new)
}

impl Hangar {
    fn new() -> Self {
        let mut hangar = Self {
            ships: HashMap::new(),
        };

        hangar.insert(shuttle::ship());

        hangar
    }

    fn insert(&mut self, ship: ShipSpecification) {
        self.ships.insert(ship.id, ship);
    }

    #[must_use]
    pub fn load(&self, ship: &Id) -> &ShipSpecification {
        self.ships.get(ship).unwrap()
    }
}

pub struct ShipSpecification {
    pub id: Id,
    pub image: &'static str,
    pub mass: f32,
    pub rotation: f32,
    pub thruster_force: f32,
}

impl ShipSpecification {
    #[must_use]
    pub fn acceleration(&self) -> f32 {
        self.thruster_force / self.mass
    }
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Hash,
    PartialEq,
    Eq,
    Copy,
    Clone,
    strum_macros::EnumCount,
    strum_macros::EnumIter,
    FromPrimitive,
    ToPrimitive,
)]
pub enum Id {
    Shuttle,
}

impl Named for Id {
    fn name(&self) -> &'static str {
        match self {
            Self::Shuttle => "Shuttle",
        }
    }
}
