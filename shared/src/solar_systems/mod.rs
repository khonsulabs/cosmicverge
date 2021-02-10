use std::collections::HashMap;

use euclid::Point2D;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use crate::protocol::SolarSystemLocationId;

mod sm0a9f4;
mod system2;

#[derive(Debug)]
pub struct Universe {
    solar_systems: HashMap<SolarSystemId, SolarSystem>,
    solar_systems_by_name: HashMap<String, SolarSystemId>,
}

pub fn universe() -> &'static Universe {
    static UNIVERSE: OnceCell<Universe> = OnceCell::new();
    UNIVERSE.get_or_init(Universe::cosmic_verge)
}

impl Universe {
    pub fn cosmic_verge() -> Self {
        let mut universe = Self {
            solar_systems: Default::default(),
            solar_systems_by_name: Default::default(),
        };

        universe.insert(sm0a9f4::system());
        universe.insert(system2::system());

        universe
    }

    fn insert(&mut self, system: SolarSystem) {
        if let Some(old) = self.solar_systems.get(&system.id) {
            panic!(
                "Reused ID {:?} between {:#?} and {:#?}",
                system.id, old, system
            );
        }

        self.solar_systems_by_name
            .insert(system.id.name().to_ascii_lowercase(), system.id);
        self.solar_systems.insert(system.id, system);
    }

    pub fn get(&self, id: &SolarSystemId) -> &SolarSystem {
        &self.solar_systems[id]
    }

    pub fn find_by_name(&self, name: &str) -> Option<&SolarSystem> {
        self.solar_systems_by_name
            .get(&name.trim().to_ascii_lowercase())
            .map(|id| self.solar_systems.get(id))
            .flatten()
    }

    pub fn systems(&self) -> impl Iterator<Item = &SolarSystem> {
        self.solar_systems.values()
    }
}

#[derive(Debug)]
pub struct SolarSystem {
    pub id: SolarSystemId,
    pub background: Option<&'static str>,
    pub galaxy_location: Point2D<f32, Galactic>,
    pub locations: HashMap<SolarSystemLocationId, SolarSystemObject>,
}

impl SolarSystem {
    fn new(id: SolarSystemId, galaxy_location: Point2D<f32, Galactic>) -> Self {
        Self {
            id,
            galaxy_location,
            background: Default::default(),
            locations: Default::default(),
        }
    }

    fn define_object<F: FnOnce(SolarSystemObject) -> SolarSystemObject, ID: NamedLocation>(
        mut self,
        id: ID,
        image: &'static str,
        size: f32,
        initializer: F,
    ) -> Self {
        let location = initializer(SolarSystemObject::new(id, image, size));
        self.locations.insert(location.id.id(), location);
        self
    }

    fn with_background(mut self, background: &'static str) -> Self {
        self.background = Some(background);
        self
    }
}

#[derive(Debug)]
pub struct SolarSystemObject {
    pub id: Box<dyn NamedLocation>,
    pub image: &'static str,
    pub size: f32,
    pub location: Point2D<f32, Solar>,
    pub owned_by: Option<Box<dyn NamedLocation>>,
}

impl SolarSystemObject {
    fn new<ID: NamedLocation>(id: ID, image: &'static str, size: f32) -> Self {
        Self {
            id: Box::new(id),
            image,
            size,
            location: Default::default(),
            owned_by: None,
        }
    }

    fn located_at(mut self, location: Point2D<f32, Solar>) -> Self {
        self.location = location;
        self
    }

    fn owned_by<ID: NamedLocation>(mut self, owner: ID) -> Self {
        self.owned_by = Some(Box::new(owner));
        self
    }
}

pub struct Pixels;
pub struct Solar;
pub struct Galactic;

pub trait Identifiable {
    fn id(&self) -> SolarSystemLocationId;
}

pub trait Named {
    fn name(&self) -> &'static str;
}

pub trait NamedLocation: Identifiable + Named + Send + Sync + std::fmt::Debug + 'static {}

impl<T> Identifiable for T
where
    T: ToPrimitive,
{
    fn id(&self) -> SolarSystemLocationId {
        SolarSystemLocationId(self.to_i64().unwrap())
    }
}

impl<T> NamedLocation for T where T: Identifiable + Named + Send + Sync + std::fmt::Debug + 'static {}

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
pub enum SolarSystemId {
    SM0A9F4,
    System2,
}

impl Named for SolarSystemId {
    fn name(&self) -> &'static str {
        match self {
            SolarSystemId::SM0A9F4 => "SM-0-A9F4",
            SolarSystemId::System2 => "System 2",
        }
    }
}
