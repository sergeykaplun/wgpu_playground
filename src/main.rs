use app::AppVariant;
use structopt::StructOpt;
use fullscreen_triangle::FullscreenTriangleExample;
use windowed_app::run;

mod windowed_app;
mod app;

#[path = "./examples/fullscreen_tiangle/fullscreen_triangle.rs"]
mod fullscreen_triangle;

fn main() {
    let app_variant = AppVariant::from_args();
    pollster::block_on(run::<FullscreenTriangleExample>("FullscreenTriangleExample", app_variant)).expect("FullscreenTriangleExample exited unexpectedly");
}