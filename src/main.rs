use simple_quad_app::SimpleQuadApp;
use windowed_app::run;

mod windowed_app;
mod simple_quad_app;
mod app;

fn main() {
    pollster::block_on(run::<SimpleQuadApp>("SimpleQuadApp")).expect("SimpleQuadApp exited unexpectedly");
}