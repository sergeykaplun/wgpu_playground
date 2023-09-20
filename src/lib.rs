#[cfg(target_os = "android")]
#[path = "./android/ffi.rs"]
mod android;

extern crate nalgebra_glm as glm;

mod app;
mod camera;
mod assets_helper;
mod model;
mod input_event;

#[path = "./pieces/geometry_primitives.rs"]
mod geometry_primitives;
#[path = "./pieces/skybox.rs"]
mod skybox;

#[path = "./examples/skybox/skybox_example.rs"]
mod skybox_example;
