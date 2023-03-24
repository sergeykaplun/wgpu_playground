use std::ffi::c_void;

use jni::{JNIEnv, objects::JClass, sys::{jobject, jlong, jint, jfloat}};
use jni_fn::jni_fn;
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, HasRawDisplayHandle, HasRawWindowHandle,
    RawDisplayHandle, RawWindowHandle,
};

use wgpu::{RequestAdapterOptions, DeviceDescriptor, Features, Limits, SurfaceConfiguration, TextureUsages, Device, Surface};
use crate::{app::{App, ShaderType}, input_event::InputEvent};
use crate::gltf_viewer::GLTFViewerExample;
use crate::assets_helper::android_resources::AndroidResourceManager;
use crate::input_event::EventType;

#[no_mangle]
#[jni_fn("com.crest.ukraine.JNITie")]
pub fn createNativeScene(env: *mut JNIEnv, _class: JClass, surface: jobject, asset_manager: jobject) -> jlong{
    let native_window = NativeWindow::new(env, surface);
    
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let surface = unsafe { instance.create_surface(&native_window).unwrap() };
    
    let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions{
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    })).unwrap();

    let (device, queue) = pollster::block_on(
        adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    features: Features::empty(),
                    limits: Limits::default()
                },
                None
            )
    ).unwrap();

    let caps = surface.get_capabilities(&adapter);
    let surface_config = SurfaceConfiguration { 
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: caps.formats[0],
        width: native_window.size[0],
        height: native_window.size[1],
        present_mode: caps.present_modes[0],
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![caps.formats[0]]
    };
    surface.configure(&device, &surface_config); 
    
    let asset_manager = unsafe { ndk_sys::AAssetManager_fromJava(env as *mut _, asset_manager) };
    let native_scene = GLTFViewerExample::new(&surface_config, &device, queue, ShaderType::WGSL, &AndroidResourceManager{ asset_manager });

    Box::into_raw(Box::new(GraphicApp{
        device,
        surface,
        native_scene,
        prev_input_event: InputEvent::default(),
        //asset_manager
    })) as jlong
}


#[jni_fn("com.crest.ukraine.JNITie")]
pub fn drawFrame(_env: *mut JNIEnv, _class: JClass, native_app: jlong){
    let graphic_app = unsafe { &mut *(native_app as *mut GraphicApp) };
    <GLTFViewerExample as App<AndroidResourceManager>>::render(&mut graphic_app.native_scene, &graphic_app.surface, &graphic_app.device);
}

#[jni_fn("com.crest.ukraine.JNITie")]
pub fn handleEvent(_env: *mut JNIEnv, _class: JClass, native_app: jlong, action: jint, x: jfloat, y: jfloat){
    let event = InputEvent::new(action, x, y);
    let graphic_app = unsafe { &mut *(native_app as *mut GraphicApp) };
    let input_event = match graphic_app.prev_input_event.event_type {
        EventType::Start | EventType::Move => {
            Some(InputEvent::diff(&graphic_app.prev_input_event, &event))
        },
        _ => None
    };
    graphic_app.prev_input_event = event;
    if let Some(input_event) = input_event {
        <GLTFViewerExample as App<AndroidResourceManager>>::process_input(&mut graphic_app.native_scene, &input_event);
    }
}

struct NativeWindow {
    native_window: *mut ndk_sys::ANativeWindow,
    size: [u32; 2]
}

impl NativeWindow {
    fn new(env: *mut JNIEnv, surface: jobject) -> NativeWindow {
        let native_window = unsafe {ndk_sys::ANativeWindow_fromSurface(env as *mut _, surface as *mut _)};
        let width = unsafe { ndk_sys::ANativeWindow_getWidth(native_window) } as u32;
        let height = unsafe { ndk_sys::ANativeWindow_getHeight(native_window) } as u32;
        NativeWindow{
            native_window,
            size: [width, height]
        }
    }
}

unsafe impl HasRawWindowHandle for NativeWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut handle = AndroidNdkWindowHandle::empty();
        handle.a_native_window = self.native_window as *mut _ as *mut c_void;
        RawWindowHandle::AndroidNdk(handle)
    }
}

unsafe impl HasRawDisplayHandle for NativeWindow {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Android(AndroidDisplayHandle::empty())
    }
}

pub struct GraphicApp {
    device: Device,
    surface: Surface,
    prev_input_event: InputEvent,
    native_scene: GLTFViewerExample,
    //asset_manager: *mut ndk_sys::AAssetManager
}