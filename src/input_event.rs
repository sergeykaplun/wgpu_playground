use winit::event::{MouseButton, MouseScrollDelta, TouchPhase};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use winit::event::WindowEvent;

#[derive(Clone)]
pub enum EventType {
    Start(u32),
    Move,
    End,
    Wheel(f32),
    None
}

#[derive(Clone)]
pub struct InputEvent {
    pub event_type: EventType,
    pub coords:     [f32; 2],
}

impl InputEvent {
    pub(crate) fn default() -> InputEvent {
        InputEvent { event_type: EventType::None, coords: [0.0; 2] }
    }
    pub(crate) fn new(event_type: i32, x: f32, y: f32) -> InputEvent {
        InputEvent {
            event_type: match event_type {
                0 => EventType::Start(0),
                1 => EventType::End,
                2 => EventType::Move,
                _ => EventType::None
            },
            coords: [x, y]
        }
    }

    pub(crate) fn diff(old: &InputEvent, new: &InputEvent) -> InputEvent {
        InputEvent {
            event_type: EventType::Move,
            coords: [new.coords[0] - old.coords[0], new.coords[1] - old.coords[1]]
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub(crate) fn from_winit_event(event: &WindowEvent) -> InputEvent {
        // match event {
        //     WindowEvent::CursorMoved { position, .. } =>
        //         InputEvent{ event_type:  EventType::Move, coords: [position.x as f32, position.y as f32] },
        //     _ => InputEvent::default()
        // }

        use winit::event::ElementState;

        match event {
            WindowEvent::MouseWheel {delta, phase, ..} => {
                /*match phase {
                    TouchPhase::Started => {}
                    TouchPhase::Moved => {}
                    TouchPhase::Ended => {}
                    TouchPhase::Cancelled => {}
                }*/

                match delta {
                    MouseScrollDelta::LineDelta(x, y) => InputEvent{ event_type: EventType::Wheel(x.max(y.clone())), coords: [0.0, 0.0] },
                    MouseScrollDelta::PixelDelta(_) => InputEvent{ event_type: EventType::None, coords: [0.0, 0.0] },
                }
            },
            WindowEvent::MouseInput { button, state, .. } => {
                match state {
                    ElementState::Pressed => {
                        InputEvent{ event_type: EventType::Start(match button { MouseButton::Left => 0, MouseButton::Right => 1, _ => 2}), coords: [0.0, 0.0] }
                    },
                    ElementState::Released => {
                        InputEvent{ event_type: EventType::End, coords: [0.0, 0.0] }
                    },
                }
            },
                
            WindowEvent::CursorMoved { position, .. } =>
                InputEvent{ event_type:  EventType::Move, coords: [position.x as f32, position.y as f32] },
            _ => InputEvent::default()
        }
    }
}