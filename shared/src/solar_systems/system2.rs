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
            Self::Sun => "Red-y",
            Self::Earth => "Desert-y",
            Self::Mercury => "Ice-y",
        }
    }
}

pub fn system() -> SolarSystem {
    SolarSystem::new(SolarSystemId::System2, Point2D::new(0., 0.))
        .with_background("/helianthusgames/Backgrounds/Red1.png")
        .define_object(
            System2::Sun,
            "/helianthusgames/Suns/12.png",
            196.,
            |location| location,
        )
        .define_object(
            System2::Earth,
            "/helianthusgames/Desert_or_Martian/1.png",
            64.,
            |location| {
                location
                    .located_at(Point2D::new(1400., 0.))
                    .owned_by(System2::Earth)
            },
        )
        .define_object(
            System2::Mercury,
            "/helianthusgames/Ice_or_Snow/1.png",
            16.,
            |location| {
                location
                    .located_at(Point2D::new(200., 200.))
                    .owned_by(System2::Earth)
            },
        )
}
