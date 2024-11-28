use pembejeo::{Event, Pembejeo};

fn main() {
    let pembejeo = Pembejeo::new().unwrap();
    println!("Hello, World!");

    loop {
        let mut event = Event::default();
        while pembejeo.wait(&mut event) {
            //println!("Event: {:?}", event);
        }
    }
}