use std::path::PathBuf;

use cosmicverge_shared::{
    euclid::Length,
    protocol::SolarSystemLocationId,
    solar_systems::{sm0a9f4::SM0A9F4, universe, SolarSystemId, SolarSystemObject},
};
use magrathea::{planet::GeneratedPlanet, ElevationColor, Kilometers, Planet};
use uuid::Uuid;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum ObjectElevations {
    DeepOcean,
    ShallowOcean,
    Beach,
    Grass,
    Forest,
    Mountain,
    Snow,
    SunlikeDeepBase,
    SunlikeBrightMiddle,
    SunlikeHotTop,
}

impl ObjectElevations {
    /// A basic elevation color palette that kinda resembles an earthlike planet
    pub fn earthlike() -> Vec<ElevationColor<Self>> {
        vec![
            ElevationColor::from_u8(
                ObjectElevations::DeepOcean,
                19,
                30,
                180,
                Kilometers::new(-1000.),
            ),
            ElevationColor::from_u8(
                ObjectElevations::ShallowOcean,
                98,
                125,
                223,
                Kilometers::new(0.),
            ),
            ElevationColor::from_u8(
                ObjectElevations::Beach,
                209,
                207,
                169,
                Kilometers::new(100.),
            ),
            ElevationColor::from_u8(
                ObjectElevations::Grass,
                152,
                214,
                102,
                Kilometers::new(200.),
            ),
            ElevationColor::from_u8(ObjectElevations::Forest, 47, 106, 42, Kilometers::new(600.)),
            ElevationColor::from_u8(
                ObjectElevations::Mountain,
                100,
                73,
                53,
                Kilometers::new(1600.),
            ),
            ElevationColor::from_u8(
                ObjectElevations::Snow,
                238,
                246,
                245,
                Kilometers::new(1700.),
            ),
        ]
    }

    pub fn sunlike() -> Vec<ElevationColor<Self>> {
        vec![
            // Deep base glow
            ElevationColor::from_u8(
                ObjectElevations::SunlikeDeepBase,
                189,
                31,
                10,
                Kilometers::new(-200.),
            ),
            // Bright middle
            ElevationColor::from_u8(
                ObjectElevations::SunlikeBrightMiddle,
                250,
                156,
                56,
                Kilometers::new(-180.),
            ),
            // Hot top
            ElevationColor::from_u8(
                ObjectElevations::SunlikeHotTop,
                255,
                218,
                41,
                Kilometers::new(200.),
            ),
        ]
    }
}

pub fn planet_for_location(
    system: &SolarSystemId,
    location: &SolarSystemLocationId,
) -> Planet<ObjectElevations> {
    match system {
        SolarSystemId::SM0A9F4 => {
            match location
                .into_location::<SM0A9F4>()
                .expect("wrong type of location")
            {
                SM0A9F4::Earth => Planet::new_from_iter(
                    Uuid::from_u128(311297988823460753720839672646651867567),
                    Default::default(),
                    Length::new(6371.),
                    ObjectElevations::earthlike(),
                ),
                SM0A9F4::Sun => Planet::new_from_iter(
                    Uuid::from_u128(311297988823460753720839672646651867567),
                    Default::default(),
                    Length::new(256.),
                    ObjectElevations::sunlike(),
                ),
                SM0A9F4::Mercury => unreachable!("hardcoded asset"),
            }
        }
        SolarSystemId::System2 => todo!(),
    }
}

pub fn generate_planet_for_location(
    system: &SolarSystemId,
    location: &SolarSystemObject,
) -> GeneratedPlanet<ObjectElevations> {
    let planet = planet_for_location(system, &location.id.id());
    planet.generate(location.size as u32, &None)
}

fn create_world(static_path: &PathBuf, system: &SolarSystemId, location: &SolarSystemObject) {
    let generated = generate_planet_for_location(system, &location);

    let system_folder = static_path
        .join("magrathea")
        .join(Into::<&'static str>::into(system));
    std::fs::create_dir_all(&system_folder).unwrap();

    generated
        .image
        .save(system_folder.join(&format!("{}.png", location.id.id())))
        .unwrap();
}

pub fn generate_assets(static_folder: PathBuf) {
    for system in universe().systems() {
        for location in system.locations.values() {
            if location.image.is_none() {
                create_world(&static_folder, &system.id, location);
            }
        }
    }
}
