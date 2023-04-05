#[cfg(target_os = "android")]
#[path = "./android/ffi.rs"]
mod android;

extern crate nalgebra_glm as glm;

mod app;
mod camera;
mod assets_helper;
mod model;
mod input_event;

#[path = "./examples/pbr/pbr.rs"]
mod pbr;