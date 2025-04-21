use rdev::{listen, simulate, Button, EventType, Key};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    // State to track if clicking is active
    let clicking = Arc::new(Mutex::new(false));
    let clicking_clone = Arc::clone(&clicking);

    // --- Clicking Thread ---
    thread::spawn(move || {
        loop {
            let is_clicking = *clicking_clone.lock().unwrap();
            if is_clicking {
                // Simulate left mouse button press
                if let Err(simulate_error) = simulate(&EventType::ButtonPress(Button::Left)) {
                    eprintln!("Error simulating mouse press: {:?}", simulate_error);
                }
                // Small delay between press and release
                thread::sleep(Duration::from_millis(10));
                // Simulate left mouse button release
                if let Err(simulate_error) = simulate(&EventType::ButtonRelease(Button::Left)) {
                    eprintln!("Error simulating mouse release: {:?}", simulate_error);
                }
                // Delay between clicks - adjust as needed for desired speed
                thread::sleep(Duration::from_millis(20));
            } else {
                // Sleep when not clicking to avoid busy-waiting
                thread::sleep(Duration::from_millis(100));
            }
        }
    });

    // --- Keyboard Listener ---
    println!("Press Left Alt + Left Ctrl + C to toggle clicking. Press Esc to exit.");

    let callback = move |event: rdev::Event| {
        match event.event_type {
            EventType::KeyPress(key) => {
                // Define your activation keys here
                // Example: Left Alt + Left Ctrl + C
                // You might need to check event.alt(), event.ctrl(), etc. depending on rdev version/API
                // For simplicity, let's use a single key for now: Key::KeyC
                // You'll likely want a more robust key combination check
                if key == Key::KeyC {
                    // Placeholder: Replace with your desired combo check
                    let mut clicking_guard = clicking.lock().unwrap();
                    *clicking_guard = !*clicking_guard; // Toggle the state
                    if *clicking_guard {
                        println!("Clicking activated!");
                    } else {
                        println!("Clicking deactivated.");
                    }
                } else if key == Key::Escape {
                    println!("Exiting...");
                    std::process::exit(0);
                }
            }
            _ => {} // Ignore other event types like KeyRelease, MouseMove, etc.
        }
    };

    // Start listening for events. This blocks the main thread.
    if let Err(error) = listen(callback) {
        eprintln!("Error listening for keyboard events: {:?}", error);
    }
}
