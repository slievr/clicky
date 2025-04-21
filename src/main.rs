#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console window on Windows in release builds

use eframe::{egui, App, Frame};
use rdev::{listen, simulate, Button, EventType, Key}; // Add Key and listen back
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Define the application state struct
struct ClickyApp {
    clicking: Arc<Mutex<bool>>,
}

impl Default for ClickyApp {
    fn default() -> Self {
        let clicking = Arc::new(Mutex::new(false));
        let clicking_clone_clicker = Arc::clone(&clicking);
        let clicking_clone_listener = Arc::clone(&clicking);

        // --- Clicking Thread ---
        thread::spawn(move || {
            let mut last_known_state = *clicking_clone_clicker.lock().unwrap(); // Initialize with current state
            eprintln!(
                "Background thread: Initial clicking state = {}",
                last_known_state
            );

            loop {
                // Add a small delay before locking to reduce contention if needed
                thread::sleep(Duration::from_millis(5));
                let is_clicking = *clicking_clone_clicker.lock().unwrap();

                // Log the state ONLY if it changed
                if is_clicking != last_known_state {
                    eprintln!(
                        "Background thread: Clicking state changed to {}",
                        is_clicking
                    );
                    last_known_state = is_clicking; // Update the last known state
                }

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

        // --- Global Keyboard Listener Thread ---
        thread::spawn(move || {
            let mut ctrl_pressed = false;
            let mut alt_pressed = false;

            let callback = move |event: rdev::Event| {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        match key {
                            Key::ControlLeft => ctrl_pressed = true,
                            Key::Alt => alt_pressed = true,
                            Key::KeyK => {
                                if ctrl_pressed && alt_pressed {
                                    let mut clicking_guard =
                                        clicking_clone_listener.lock().unwrap();
                                    *clicking_guard = !*clicking_guard;
                                    eprintln!(
                                        "Global Keybind (Ctrl+Alt+K) pressed: Toggling clicking state to {}",
                                        *clicking_guard
                                    );
                                }
                            }
                            _ => {} // Ignore other keys
                        }
                    }
                    EventType::KeyRelease(key) => {
                        match key {
                            Key::ControlLeft => ctrl_pressed = false,
                            Key::Alt => alt_pressed = false,
                            _ => {} // Ignore other keys
                        }
                    }
                    _ => {} // Ignore other event types
                }
            };

            println!("Starting global keyboard listener...");
            if let Err(error) = listen(callback) {
                eprintln!("Error setting up global keyboard listener: {:?}", error);
            }
            println!("Global keyboard listener stopped."); // Should ideally not be reached unless there's an error
        });

        Self { clicking }
    }
}

// Implement the eframe::App trait for our struct
impl App for ClickyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Clicky");
            ui.separator();

            let mut clicking_guard = self.clicking.lock().unwrap();
            let is_clicking = *clicking_guard;

            ui.horizontal(|ui| {
                // Start Button
                if ui
                    .add_enabled(!is_clicking, egui::Button::new("Start Clicking"))
                    .clicked()
                {
                    *clicking_guard = true;
                    eprintln!(
                        "Start Button clicked: Toggling clicking state to {}",
                        *clicking_guard
                    );
                }

                // Stop Button
                if ui
                    .add_enabled(is_clicking, egui::Button::new("Stop Clicking"))
                    .clicked()
                {
                    *clicking_guard = false;
                    eprintln!(
                        "Stop Button clicked: Toggling clicking state to {}",
                        *clicking_guard
                    );
                }
            });

            // Update label to reflect the global keybind
            ui.label(
                "Or press Left Ctrl + Left Alt + K globally to toggle clicking.",
            );
            ui.separator();

            // Display the current status with color
            let (status_text, status_color) = if *clicking_guard {
                ("Clicking Active", egui::Color32::GREEN)
            } else {
                ("Clicking Inactive", egui::Color32::GOLD) // Using GOLD as a shade of orange
                                                           // Or use egui::Color32::from_rgb(255, 165, 0) for a specific orange
            };
            ui.label(egui::RichText::new(status_text).color(status_color));
        });

        // Request repaint to ensure UI updates when state changes externally
        ctx.request_repaint();
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([300.0, 150.0]) // Increase window size
            .with_resizable(false), // Optional: Make window non-resizable if desired
        ..Default::default()
    };

    // Run the eframe application
    eframe::run_native(
        "Clicky App", // Window title
        options,
        Box::new(|_cc| Box::<ClickyApp>::default()), // Fix: Directly return the Boxed app state
    )
}
