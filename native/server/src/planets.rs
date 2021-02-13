use std::path::PathBuf;

use cosmicverge_shared::{
    euclid::Length,
    protocol::SolarSystemLocationId,
    solar_systems::{
        sm0a9f4::SM0A9F4, system2::System2, universe, SolarSystemId, SolarSystemObject,
    },
};
use magrathea::{planet::GeneratedPlanet, ElevationColor, Kilometers, Planet};
use uuid::Uuid;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum ObjectElevations {
    DeepOcean,
    Canyon,
    ShallowOcean,
    Crater,
    Beach,
    Ground,
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

    pub fn redrock() -> Vec<ElevationColor<Self>> {
        vec![
            ElevationColor::from_u8(
                ObjectElevations::Canyon,
                0x69,
                0x1D,
                0x1D,
                Kilometers::new(-1000.),
            ),
            ElevationColor::from_u8(
                ObjectElevations::Crater,
                0x96,
                0x48,
                0x48,
                Kilometers::new(-50.),
            ),
            ElevationColor::from_u8(
                ObjectElevations::Ground,
                0xB8,
                0x6F,
                0x6F,
                Kilometers::new(200.),
            ),
            ElevationColor::from_u8(
                ObjectElevations::Mountain,
                0x74,
                0x20,
                0x20,
                Kilometers::new(1000.),
            ),
        ]
    }

    pub fn whiterock() -> Vec<ElevationColor<Self>> {
        vec![
            ElevationColor::from_u8(
                ObjectElevations::Canyon,
                0x9B,
                0xA8,
                0xA8,
                Kilometers::new(-1000.),
            ),
            ElevationColor::from_u8(
                ObjectElevations::Crater,
                0xE9,
                0xF6,
                0xF6,
                Kilometers::new(-50.),
            ),
            ElevationColor::from_u8(
                ObjectElevations::Ground,
                0xCE,
                0xDF,
                0xDF,
                Kilometers::new(200.),
            ),
            ElevationColor::from_u8(
                ObjectElevations::Mountain,
                0xAA,
                0xB9,
                0xB9,
                Kilometers::new(1000.),
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

    pub fn blue_sunlike() -> Vec<ElevationColor<Self>> {
        vec![
            // Deep base glow
            ElevationColor::from_u8(
                ObjectElevations::SunlikeDeepBase,
                10,
                31,
                189,
                Kilometers::new(-200.),
            ),
            // Bright middle
            ElevationColor::from_u8(
                ObjectElevations::SunlikeBrightMiddle,
                56,
                156,
                250,
                Kilometers::new(-180.),
            ),
            // Hot top
            ElevationColor::from_u8(
                ObjectElevations::SunlikeHotTop,
                41,
                218,
                255,
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
                SM0A9F4::Sun => Planet::new_from_iter(
                    Uuid::from_u128(311297988823460753720839672646651867567),
                    Default::default(),
                    Length::new(4000.),
                    ObjectElevations::sunlike(),
                ),
                SM0A9F4::Earth => Planet::new_from_iter(
                    Uuid::from_u128(311297988823460753720839672646651867567),
                    Default::default(),
                    Length::new(6371.),
                    ObjectElevations::earthlike(),
                ),
                SM0A9F4::Mercury => Planet::new_from_iter(
                    Uuid::from_u128(311297988823460753720839672646651867564),
                    Default::default(),
                    Length::new(400.),
                    ObjectElevations::redrock(),
                ),
            }
        }
        SolarSystemId::System2 => match location.into_location::<System2>().unwrap() {
            System2::Sun => Planet::new_from_iter(
                Uuid::from_u128(311297988823460753720839672646651867565),
                Default::default(),
                Length::new(4000.),
                ObjectElevations::blue_sunlike(),
            ),
            System2::Mercury => Planet::new_from_iter(
                Uuid::from_u128(311297988823460753720839672646651867561),
                Default::default(),
                Length::new(200.),
                ObjectElevations::whiterock(),
            ),
            System2::Earth => Planet::new_from_iter(
                Uuid::from_u128(311297988823460753720839672646651867562),
                Default::default(),
                Length::new(6371.),
                ObjectElevations::earthlike(),
            ),
        },
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
