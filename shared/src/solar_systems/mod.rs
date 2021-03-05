use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    f32::consts::PI,
    iter::FromIterator,
    sync::RwLock,
};

use euclid::{Angle, Point2D, Vector2D};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use crate::protocol::navigation;

pub mod sm0a9f4;
pub mod system2;

const LONGEST_PLANET_ORBIT_DAYS: i64 = 3650;

pub type SolarSystemOrbits = HashMap<navigation::SolarSystemId, Point2D<f32, Solar>>;

#[derive(Debug)]
pub struct Universe {
    solar_systems: HashMap<SolarSystemId, SolarSystem>,
    solar_systems_by_name: HashMap<String, SolarSystemId>,
    orbits: RwLock<HashMap<SolarSystemId, SolarSystemOrbits>>,
}

pub fn universe() -> &'static Universe {
    static UNIVERSE: OnceCell<Universe> = OnceCell::new();
    UNIVERSE.get_or_init(Universe::cosmic_verge)
}

impl Universe {
    #[must_use]
    pub fn cosmic_verge() -> Self {
        let mut universe = Self {
            solar_systems: HashMap::new(),
            solar_systems_by_name: HashMap::new(),
            orbits: RwLock::new(HashMap::new()),
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
            .and_then(|id| self.solar_systems.get(id))
    }

    pub fn systems(&self) -> impl Iterator<Item = &SolarSystem> {
        self.solar_systems.values()
    }

    pub fn update_orbits(&self, timestamp: f64) {
        let mut orbits = self.orbits.write().unwrap();
        for system in self.solar_systems.values() {
            orbits.insert(system.id, system.calculate_orbits(timestamp));
        }
    }

    pub fn orbits_for(&self, system: SolarSystemId) -> SolarSystemOrbits {
        let orbits = self.orbits.read().unwrap();
        orbits[&system].clone()
    }
}

#[derive(Debug)]
pub struct SolarSystem {
    pub id: SolarSystemId,
    pub background: Option<&'static str>,
    pub galaxy_location: Point2D<f32, Galactic>,
    pub locations: HashMap<navigation::SolarSystemId, Object>,
    pub locations_by_owners:
        HashMap<Option<navigation::SolarSystemId>, Vec<navigation::SolarSystemId>>,
}

impl SolarSystem {
    fn new(id: SolarSystemId, galaxy_location: Point2D<f32, Galactic>) -> Self {
        Self {
            id,
            galaxy_location,
            background: None,
            locations: HashMap::new(),
            locations_by_owners: HashMap::new(),
        }
    }

    fn define_object<F: FnOnce(Object) -> Object, ID: NamedLocation>(
        mut self,
        id: ID,
        size: f32,
        initializer: F,
    ) -> Self {
        let location = initializer(Object::new(self.id, id, size));
        let id = location.id.id();
        let owner_locations = self
            .locations_by_owners
            .entry(location.owned_by.as_ref().map(|o| o.id()))
            .or_default();
        owner_locations.push(id);
        self.locations.insert(id, location);
        self
    }

    #[allow(dead_code)] // This will be used eventually when we have more art
    const fn with_background(mut self, background: &'static str) -> Self {
        self.background = Some(background);
        self
    }

    fn calculate_orbits(&self, timestamp: f64) -> SolarSystemOrbits {
        let mut orbits = SolarSystemOrbits::new();
        let mut objects_to_process =
            VecDeque::from_iter(self.locations_by_owners.get(&None).unwrap());
        while let Some(object_id) = objects_to_process.pop_front() {
            let object = &self.locations[object_id];
            let location = match &object.owned_by {
                Some(owner) => {
                    let orbit_around = *orbits.get(&owner.id()).expect("Error in ownership chain");
                    // All planets for now will follow a basic ellipse with the radius having a constant multiplier
                    // The orbit will swing y twice as much as the x axis
                    let truncated_epoch = (timestamp as i64 + object.orbit_seed)
                        % (LONGEST_PLANET_ORBIT_DAYS * 24 * 60 * 60);
                    let period_in_seconds = f64::from(object.orbit_days) * 24. * 60. * 60.;
                    let orbit_amount =
                        (truncated_epoch as f64 % period_in_seconds) / period_in_seconds;
                    let orbit_angle = orbit_amount as f32 * PI * 2.;

                    let (x, y) = Angle::radians(orbit_angle).sin_cos();
                    let relative_location = Vector2D::new(
                        (x * object.orbit_distance).mul_add(2., object.orbit_distance / 2.),
                        y * object.orbit_distance,
                    );

                    orbit_around + relative_location
                }
                None => Point2D::default(),
            };

            orbits.insert(*object_id, location);

            if let Some(children) = self.locations_by_owners.get(&Some(*object_id)) {
                objects_to_process.extend(children);
            }
        }
        orbits
    }
}

#[derive(Debug)]
pub struct Object {
    pub id: Box<dyn NamedLocation>,
    pub system: SolarSystemId,
    pub image: Option<&'static str>,
    pub size: f32,
    pub orbit_distance: f32,
    pub orbit_days: f32,
    orbit_seed: i64,
    pub owned_by: Option<Box<dyn NamedLocation>>,
}

impl Object {
    fn new<ID: NamedLocation>(system: SolarSystemId, id: ID, size: f32) -> Self {
        Self {
            id: Box::new(id),
            system,
            size,
            image: None,
            owned_by: None,
            orbit_seed: 0,
            orbit_distance: 0.,
            orbit_days: 0.,
        }
    }

    fn orbiting_at(mut self, orbit_distance: f32, orbit_days: f32, orbit_seed: i64) -> Self {
        self.orbit_distance = orbit_distance;
        assert!(orbit_days < LONGEST_PLANET_ORBIT_DAYS as f32);
        self.orbit_days = orbit_days;
        self.orbit_seed = orbit_seed;
        self
    }

    fn owned_by<ID: NamedLocation>(mut self, owner: ID) -> Self {
        self.owned_by = Some(Box::new(owner));
        self
    }

    // fn with_image(mut self, url: &'static str) -> Self {
    //     self.image = Some(url);
    //     self
    // }

    #[must_use]
    pub fn image_url(&self) -> Cow<'static, str> {
        if let Some(image) = self.image {
            Cow::from(image)
        } else {
            Cow::from(format!(
                "/magrathea/{}/{}.png",
                Into::<&'static str>::into(self.system),
                self.id.id()
            ))
        }
    }
}

pub struct Pixels;
pub struct Solar;
pub struct Galactic;

pub trait Identifiable {
    fn id(&self) -> navigation::SolarSystemId;
}

pub trait Named {
    fn name(&self) -> &'static str;
}

pub trait NamedLocation: Identifiable + Named + Send + Sync + std::fmt::Debug + 'static {}

impl<T> Identifiable for T
where
    T: ToPrimitive,
{
    fn id(&self) -> navigation::SolarSystemId {
        navigation::SolarSystemId(self.to_i64().unwrap())
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
    strum_macros::IntoStaticStr,
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
            Self::SM0A9F4 => "SM-0-A9F4",
            Self::System2 => "System 2",
        }
    }
}
