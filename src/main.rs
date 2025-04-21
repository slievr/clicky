#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console window on Windows in release builds

use eframe::egui::IconData;
use eframe::{egui, App, Frame};
use image::ImageFormat;
use rdev::{listen, simulate, Button, EventType, Key}; // Add Key and listen back
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Define the application state struct
struct ClickyApp {
    clicking: Arc<Mutex<bool>>,
    cps: Arc<Mutex<f64>>, // Add CPS state
}

impl Default for ClickyApp {
    fn default() -> Self {
        let clicking = Arc::new(Mutex::new(false));
        let cps = Arc::new(Mutex::new(1.0)); // Initialize CPS to 1.0
        let clicking_clone_clicker = Arc::clone(&clicking);
        let clicking_clone_listener = Arc::clone(&clicking);
        let cps_clone_clicker = Arc::clone(&cps);

        // --- Clicking Thread ---
        thread::spawn(move || {
            let mut last_known_state = *clicking_clone_clicker.lock().unwrap(); // Initialize with current state
            eprintln!(
                "Background thread: Initial clicking state = {}",
                last_known_state
            );

            loop {
                // Read shared state *inside* the loop
                let is_clicking = *clicking_clone_clicker.lock().unwrap();
                let current_cps: f64 = *cps_clone_clicker.lock().unwrap(); // Read CPS value on each iteration

                // Log the state ONLY if it changed
                if is_clicking != last_known_state {
                    eprintln!(
                        "Background thread: Clicking state changed to {}",
                        is_clicking
                    );
                    last_known_state = is_clicking; // Update the last known state
                }

                if is_clicking {
                    // Calculate delay based on the current CPS value
                    let cps_value = current_cps.max(1.0_f64);

                    // Simplified direct approach for accurate CPS
                    // For higher CPS values, we need to be more precise with timing

                    // Calculate time between clicks in microseconds for more precision
                    let delay_us = (1_000_000.0_f64 / cps_value).round() as u64;

                    // Single click with precise timing
                    if let Err(simulate_error) = simulate(&EventType::ButtonPress(Button::Left)) {
                        eprintln!("Error simulating mouse press: {:?}", simulate_error);
                    }

                    // Very minimal delay between press and release (0.01ms)
                    thread::sleep(Duration::from_nanos(10));

                    if let Err(simulate_error) = simulate(&EventType::ButtonRelease(Button::Left)) {
                        eprintln!("Error simulating mouse release: {:?}", simulate_error);
                    }

                    // Calculate remaining time to sleep to maintain accurate CPS
                    // Subtract the time we already spent on the press/release (10µs)
                    if delay_us > 10 {
                        thread::sleep(Duration::from_micros(delay_us - 10));
                    }
                } else {
                    // Sleep longer when not clicking
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

        Self { clicking, cps }
    }
}

// Implement the eframe::App trait for our struct
impl App for ClickyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Clicky");
            ui.separator();

            let mut clicking_guard = self.clicking.lock().unwrap();
            let mut cps_guard = self.cps.lock().unwrap(); // Add CPS guard
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
            ui.label("Or press Left Ctrl + Left Alt + K globally to toggle clicking.");
            ui.separator();

            // --- CPS Slider ---
            ui.add(
                // Set slider range to max 150 CPS
                egui::Slider::new(&mut *cps_guard, 1.0..=150.0)
                    .text("Clicks per Second")
                    .logarithmic(false)
                    .show_value(true),
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

fn load_icon(bytes: &[u8]) -> Result<IconData, String> {
    let image = image::load(Cursor::new(bytes), ImageFormat::Png)
        .map_err(|e| e.to_string())? // Convert image error to String
        .to_rgba8();
    let (width, height) = image.dimensions();
    Ok(IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn main() -> Result<(), eframe::Error> {
    // Load the icon bytes at compile time
    let icon_bytes = include_bytes!("../icon.png"); // Assumes icon.png is in the project root
    let icon = load_icon(icon_bytes).expect("Failed to load application icon.");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([300.0, 150.0])
            .with_resizable(false)
            .with_icon(icon), // Set the loaded icon
        ..Default::default()
    };

    // Run the eframe application
    eframe::run_native(
        "Clicky App",
        options,
        Box::new(|_cc| Box::<ClickyApp>::default()),
    )
}
