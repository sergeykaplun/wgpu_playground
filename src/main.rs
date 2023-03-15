use app::AppVariant;
use flipboard::FlipboardExample;
use boxes::BoxesExample;
use gltf_viewer::GLTFViewerExample;
use structopt::StructOpt;
use fullscreen_triangle::FullscreenTriangleExample;
use shadow_mapping::ShadowMappingExample;
use windowed_app::run;

mod windowed_app;
mod app;
mod camera;
mod assets_helper;
mod model;

extern crate nalgebra_glm as glm;

#[path = "./examples/fullscreen_tiangle/fullscreen_triangle.rs"]
mod fullscreen_triangle;
#[path = "./examples/boxes/boxes.rs"]
mod boxes;
#[path = "./examples/flipboard/flipboard.rs"]
mod flipboard;
#[path = "./examples/shadow_mapping/shadow_mapping.rs"]
mod shadow_mapping;
#[path = "./examples/gltf/gltf_viewer.rs"]
mod gltf_viewer;

fn main() {
    let app_variant = AppVariant::from_args();
    //pollster::block_on(run::<FullscreenTriangleExample>("FullscreenTriangleExample", app_variant)).expect("FullscreenTriangleExample exited unexpectedly");
    //pollster::block_on(run::<BoxesExample>("BoxesExample", app_variant)).expect("BoxesExample exited unexpectedly");
    //pollster::block_on(run::<FlipboardExample>("FlipboardExample", app_variant)).expect("FlipboardExample exited unexpectedly");
    //pollster::block_on(run::<ShadowMappingExample>("ShadowMappingExample", app_variant)).expect("ShadowMappingExample exited unexpectedly");
    pollster::block_on(run::<GLTFViewerExample>("GLTFViewerExample", app_variant)).expect("GLTFViewerExample exited unexpectedly");
}