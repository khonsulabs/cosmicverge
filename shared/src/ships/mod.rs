use std::collections::HashMap;

use once_cell::sync::OnceCell;

use crate::solar_systems::Named;
use num_derive::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};

mod shuttle;

pub struct Hangar {
    ships: HashMap<ShipId, ShipSpecification>,
}

pub fn hangar() -> &'static Hangar {
    static SHARED_HANGAR: OnceCell<Hangar> = OnceCell::new();
    SHARED_HANGAR.get_or_init(Hangar::new)
}

impl Hangar {
    fn new() -> Self {
        let mut hangar = Self {
            ships: Default::default(),
        };

        hangar.insert(shuttle::ship());

        hangar
    }

    fn insert(&mut self, ship: ShipSpecification) {
        self.ships.insert(ship.id, ship);
    }

    pub fn load(&self, ship: &ShipId) -> &ShipSpecification {
        self.ships.get(&ship).unwrap()
    }
}

pub struct ShipSpecification {
    pub id: ShipId,
    pub image: &'static str,
    pub mass: f64,
    pub thruster_force: f32,
    pub rotor_force: f32,
    pub mass_radius: f32,
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
pub enum ShipId {
    Shuttle,
}

impl Named for ShipId {
    fn name(&self) -> &'static str {
        match self {
            ShipId::Shuttle => "Shuttle",
        }
    }
}
