mod pembejeo;
mod mouse;
mod keyboard;
mod event;
mod error;

#[cfg(target_os = "macos")]
mod apple;


pub use pembejeo::*;
pub use mouse::*;
pub use keyboard::*;
pub use event::*;
pub use error::*;

#[cfg(test)]
mod tests {
    use crate::Event;

    #[test]
    fn hello_world() {
        println!("Hello, World!");
        let pembejeo = crate::Pembejeo::new().unwrap();

        loop {
            println!("Waiting for input!");
            let mut event = Event::default();
            while pembejeo.wait(&mut event) {
                println!("Event: {:?}", event);
            }

            println!("Rendering!");
        }
    }

    #[test]
    fn test_sizes() {
        use core_foundation::string::{CFString, CFStringRef};
        let normal_size = std::mem::size_of::<CFString>();
        let ref_size = std::mem::size_of::<CFStringRef>();

        println!("{} {}", normal_size, ref_size);
    }
}
