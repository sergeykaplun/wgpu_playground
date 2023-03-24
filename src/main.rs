use app::AppVariant;
use structopt::StructOpt;
use windowed_app::run;

mod windowed_app;
mod app;
mod camera;
mod assets_helper;
mod model;
mod input_event;

extern crate nalgebra_glm as glm;

#[path = "./examples/gltf/gltf_viewer.rs"]
mod gltf_viewer;
use gltf_viewer::GLTFViewerExample;

fn main() {
    let app_variant = AppVariant::from_args();
    pollster::block_on(run::<GLTFViewerExample>("GLTFViewerExample", app_variant));
}