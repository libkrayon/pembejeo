use std::default;

#[derive(Debug, Default, Clone, PartialEq)]
pub enum Event {
    #[default]
    Empty,

    MouseMotion(MouseMotionEvent)
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseMotionEvent {
    pub device_id: String,
    pub x: i16,
    pub y: i16,
}