use euclid::Point2D;
use num_derive::{FromPrimitive, ToPrimitive};

use crate::solar_systems::{Named, SolarSystem, SolarSystemId};

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

pub fn system() -> SolarSystem {
    SolarSystem::new(SolarSystemId::SM0A9F4)
        .with_background("/helianthusgames/Backgrounds/BlueStars.png")
        .with_location(
            SM0A9F4::Sun,
            "/helianthusgames/Suns/2.png",
            128.,
            |location| location,
        )
        .with_location(
            SM0A9F4::Earth,
            "/helianthusgames/Terran_or_Earth-like/1.png",
            32.,
            |location| {
                location
                    .located_at(Point2D::new(600., 0.))
                    .owned_by(SM0A9F4::Earth)
            },
        )
        .with_location(
            SM0A9F4::Mercury,
            "/helianthusgames/Rocky/1.png",
            24.,
            |location| {
                location
                    .located_at(Point2D::new(200., 200.))
                    .owned_by(SM0A9F4::Earth)
            },
        )
}
