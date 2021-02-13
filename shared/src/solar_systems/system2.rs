use euclid::Point2D;
use num_derive::{FromPrimitive, ToPrimitive};

use crate::solar_systems::{Named, SolarSystem, SolarSystemId};

#[derive(Debug, FromPrimitive, ToPrimitive)]
pub enum System2 {
    Sun,
    Earth,
    Mercury,
}

impl Named for System2 {
    fn name(&self) -> &'static str {
        match self {
            Self::Sun => "Blue Star",
            Self::Earth => "Earth 2",
            Self::Mercury => "Ice-y",
        }
    }
}

pub fn system() -> SolarSystem {
    SolarSystem::new(SolarSystemId::System2, Point2D::new(0., 0.))
        .with_background("/helianthusgames/Backgrounds/Red1.png")
        .define_object(System2::Sun, 196., |location| location)
        .define_object(System2::Earth, 64., |location| {
            location.orbiting_at(1400., 600., 30).owned_by(System2::Sun)
        })
        .define_object(System2::Mercury, 16., |location| {
            location
                .orbiting_at(200., 1. / 16., 10)
                .owned_by(System2::Earth)
        })
}
