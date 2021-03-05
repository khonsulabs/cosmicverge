use euclid::Point2D;
use num_derive::{FromPrimitive, ToPrimitive};

use crate::solar_systems::{Named, SolarSystem, SystemId};

#[derive(Debug, FromPrimitive, ToPrimitive)]
pub enum SM0A9F4 {
    Sun,
    Earth,
    Mercury,
}

impl Named for SM0A9F4 {
    fn name(&self) -> &'static str {
        match self {
            SM0A9F4::Sun => "Sun",
            SM0A9F4::Earth => "Earth",
            SM0A9F4::Mercury => "Mercury",
        }
    }
}

#[must_use]
pub fn system() -> SolarSystem {
    SolarSystem::new(SystemId::SM0A9F4, Point2D::new(0., 0.))
        .define_object(SM0A9F4::Sun, 128., |location| location)
        .define_object(SM0A9F4::Earth, 32., |location| {
            location.orbiting_at(600., 365., 0).owned_by(SM0A9F4::Sun)
        })
        .define_object(SM0A9F4::Mercury, 24., |location| {
            location
                .orbiting_at(200., 58.66, 200)
                .owned_by(SM0A9F4::Sun)
        })
}
