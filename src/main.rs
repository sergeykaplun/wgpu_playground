use app::AppVariant;
use boxes::BoxesExample;
use structopt::StructOpt;
use fullscreen_triangle::FullscreenTriangleExample;
use windowed_app::run;

mod windowed_app;
mod app;
mod camera;

#[path = "./examples/fullscreen_tiangle/fullscreen_triangle.rs"]
mod fullscreen_triangle;
#[path = "./examples/boxes/boxes.rs"]
mod boxes;

fn main() {
    let app_variant = AppVariant::from_args();
    //pollster::block_on(run::<FullscreenTriangleExample>("FullscreenTriangleExample", app_variant)).expect("FullscreenTriangleExample exited unexpectedly");
    pollster::block_on(run::<BoxesExample>("BoxesExample", app_variant)).expect("BoxesExample exited unexpectedly");
}