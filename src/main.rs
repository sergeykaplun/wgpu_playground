use app::AppVariant;
use structopt::StructOpt;
use windowed_app::run;

mod windowed_app;
mod app;
mod camera;
mod assets_helper;
mod model;
mod input_event;

#[path = "./pieces/geometry_primitives.rs"]
mod geometry_primitives;
#[path = "./pieces/skybox.rs"]
mod skybox;

extern crate nalgebra_glm as glm;

/*#[path = "./examples/pbr/pbr.rs"]
mod pbr;
use pbr::PBRExample;*/

/*#[path = "./examples/volumetric/volumetric.rs"]
mod volumetric;
use volumetric::VolumetricExample;*/

#[path = "./examples/2d_liquid/liquid2d.rs"]
mod liquid2d;
use liquid2d::Liquid2DExample;

fn main() {
    let app_variant = AppVariant::from_args();
    //pollster::block_on(run::<PBRExample>("PBRExample", app_variant));
    pollster::block_on(run::<Liquid2DExample>("Liquid 2D Example", app_variant));
}