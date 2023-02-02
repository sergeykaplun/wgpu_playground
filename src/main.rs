use app::AppVariant;
use structopt::StructOpt;
use simple_tri_app::SimpleTriApp;
use windowed_app::run;

mod windowed_app;
mod simple_tri_app;
mod app;

fn main() {
    let app_variant = AppVariant::from_args();
    pollster::block_on(run::<SimpleTriApp>("FullscreenTriApp", app_variant)).expect("FullscreenTriApp exited unexpectedly");
}