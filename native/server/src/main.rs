#[macro_use]
extern crate tracing;

use std::path::Path;

use basws_server::shared::Uuid;
use magrathea::{coloring::Earthlike, Planet};
use magrathea::{ElevationColor, Kilometers};
use magrathea::{image, planet};
use magrathea::euclid::Length;
use magrathea::euclid::Point2D;
use magrathea::image::{DynamicImage, RgbaImage};
use magrathea::Light;
use magrathea::palette::Srgb;
use warp::{Filter, Rejection, Reply};
use warp::filters::BoxedFilter;

#[cfg(debug_assertions)]
const STATIC_FOLDER_PATH: &str = "../../web/static";
#[cfg(not(debug_assertions))]
const STATIC_FOLDER_PATH: &str = "static";

#[cfg(debug_assertions)]
const PRIVATE_ASSETS_PATH: &str = "../../private/assets";
#[cfg(not(debug_assertions))]
const STATIC_FOLDER_PATH: &str = "static";

#[tokio::main]
async fn main() {
    info!("server starting up");

    let custom_logger = warp::log::custom(|info| {
        if info.status().is_server_error() {
            error!(
                path = info.path(),
                method = info.method().as_str(),
                status = info.status().as_str(),
                "Request Served"
            );
        } else {
            info!(
                path = info.path(),
                method = info.method().as_str(),
                status = info.status().as_u16(),
                "Request Served"
            );
        }
    });

    let base_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_owned());
    let base_dir = Path::new(&base_dir);
    let static_path = base_dir.join(STATIC_FOLDER_PATH);
    let index_path = static_path.join("index.html");


    let spa = warp::get()
        .and(warp::fs::dir(static_path).or(warp::fs::file(index_path)));
    let private_assets_path = base_dir.join(PRIVATE_ASSETS_PATH);
    let private_assets = warp::fs::dir(private_assets_path);

    let api = warp::path("api").and(magrathea_filter());

    warp::serve(api.or(private_assets).or(spa)
        .with(custom_logger)).run(([0, 0, 0, 0], 7879)).await
}

fn magrathea_filter() -> BoxedFilter<(impl Reply, )> {
    warp::path("magrathea")
        .and(warp::path("world").and(
            warp::path::param()).
            and(warp::path::param()).
            and(warp::path::param()).
            and(warp::path::param()).
            and(warp::path::param())
            .map(|seed, x, y, radius, resolution| create_world(MagratheaType::Planet, seed, x, y, radius, resolution))
            .or(
                warp::path("sun").and(
                    warp::path::param()).
                    and(warp::path::param()).
                    and(warp::path::param())
                    .map(|seed, radius, resolution| create_world(MagratheaType::Sun, seed, 0., 0., radius, resolution)
                    ))
        ).boxed()
}

enum MagratheaType {
    Planet,
    Sun,
}

fn create_world(kind: MagratheaType, seed: Uuid, x: f32, y: f32, radius: f32, resolution: u32) -> impl Reply {
    let planet = Planet::<Earthlike> {
        seed,
        origin: Point2D::<f32, Kilometers>::new(x, y),
        radius: Length::new(radius),
        colors: ElevationColor::earthlike(),
    };
    let generated = planet.generate(resolution, &Some(Light {
        color: Srgb::new(1., 1., 1.),
        sols: 1.,
    }));
    let image = match kind {
        MagratheaType::Planet => generate_world(seed, x, y, radius, resolution),
        MagratheaType::Sun => generate_sun(seed, x, y, radius, resolution),
    };
    let image = DynamicImage::ImageRgba8(image);
    let mut bytes: Vec<u8> = Vec::new();
    image.write_to(&mut bytes, image::ImageOutputFormat::Png).unwrap();

    warp::reply::with_header(bytes, "Content-Type", "image/png")
}

fn generate_world(seed: Uuid, x: f32, y: f32, radius: f32, resolution: u32) -> RgbaImage {
    let planet = Planet {
        seed,
        origin: Point2D::<f32, Kilometers>::new(x, y),
        radius: Length::new(radius),
        colors: ElevationColor::earthlike(),
    };
    planet.generate(resolution, &Some(Light {
        color: Srgb::new(1., 1., 1.),
        sols: 1.,
    })).image
}

fn generate_sun(seed: Uuid, x: f32, y: f32, radius: f32, resolution: u32) -> RgbaImage {
    let planet = Planet {
        seed,
        origin: Point2D::<f32, Kilometers>::new(x, y),
        radius: Length::new(radius),
        colors: ElevationColor::sunlike(),
    };
    planet.generate(resolution, &None).image
}