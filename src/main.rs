use structopt::StructOpt;
use app_variants::AppVariant;
use simple_quad_app::SimpleQuadApp;
use windowed_app::run;

mod windowed_app;
mod simple_quad_app;
mod app;
mod app_variants;

fn main() {
    let app_variant = AppVariant::from_args();
    pollster::block_on(run::<SimpleQuadApp>("SimpleQuadApp", app_variant)).expect("SimpleQuadApp exited unexpectedly");
}