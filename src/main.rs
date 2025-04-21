#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console window on Windows in release builds

use eframe::egui::IconData;
use eframe::{egui, App, Frame};
use image::ImageFormat;
use rdev::{listen, simulate, Button, EventType, Key};
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// Define the application state struct
struct ClickyApp {
    clicking: Arc<Mutex<bool>>,
    cps: Arc<Mutex<f64>>,
    clicker_thread: Arc<Mutex<Option<(JoinHandle<()>, Arc<AtomicBool>)>>>, // Store thread handle and stop flag
}

impl Default for ClickyApp {
    fn default() -> Self {
        let clicking = Arc::new(Mutex::new(false));
        let cps = Arc::new(Mutex::new(1.0));
        let clicker_thread = Arc::new(Mutex::new(None)); // No clicking thread initially
        let clicking_clone_listener = Arc::clone(&clicking);
        let cps_clone = Arc::clone(&cps);
        let clicker_thread_clone = Arc::clone(&clicker_thread);

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
                                    let was_clicking = *clicking_guard;
                                    *clicking_guard = !was_clicking;

                                    if !was_clicking && *clicking_guard {
                                        // Start clicking thread when activated via hotkey
                                        let current_cps = *cps_clone.lock().unwrap();
                                        start_clicking_thread(&clicker_thread_clone, current_cps);
                                    } else if was_clicking && !*clicking_guard {
                                        // Stop clicking thread when deactivated via hotkey
                                        stop_clicking_thread(&clicker_thread_clone);
                                    }

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
            println!("Global keyboard listener stopped.");
        });

        Self {
            clicking,
            cps,
            clicker_thread,
        }
    }
}

// Helper function to start the clicking thread
fn start_clicking_thread(
    clicker_thread: &Arc<Mutex<Option<(JoinHandle<()>, Arc<AtomicBool>)>>>,
    initial_cps: f64,
) {
    let mut thread_guard = clicker_thread.lock().unwrap();

    // Only start a new thread if one isn't already running
    if thread_guard.is_none() {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = Arc::clone(&stop_flag);
        let cps_value = initial_cps.max(1.0);

        // Calculate delay once outside the loop
        let delay_us = (1_000_000.0_f64 / cps_value).round() as u64;

        // Spawn the clicking thread
        let handle = thread::spawn(move || {
            eprintln!("Clicking thread started with CPS: {}", cps_value);

            while !stop_flag_clone.load(Ordering::SeqCst) {
                // Single click with precise timing
                if let Err(simulate_error) = simulate(&EventType::ButtonPress(Button::Left)) {
                    eprintln!("Error simulating mouse press: {:?}", simulate_error);
                }

                if let Err(simulate_error) = simulate(&EventType::ButtonRelease(Button::Left)) {
                    eprintln!("Error simulating mouse release: {:?}", simulate_error);
                }

                // Sleep for the pre-calculated duration
                if delay_us > 10 {
                    thread::sleep(Duration::from_micros(delay_us - 10));
                }

                // Check stop flag more frequently for faster response
                if stop_flag_clone.load(Ordering::SeqCst) {
                    break;
                }
            }

            eprintln!("Clicking thread stopped");
        });

        // Store the thread handle and stop flag
        *thread_guard = Some((handle, stop_flag));
    }
}

// Helper function to stop the clicking thread
fn stop_clicking_thread(clicker_thread: &Arc<Mutex<Option<(JoinHandle<()>, Arc<AtomicBool>)>>>) {
    let mut thread_guard = clicker_thread.lock().unwrap();

    if let Some((handle, stop_flag)) = thread_guard.take() {
        // Signal the thread to stop
        stop_flag.store(true, Ordering::SeqCst);

        // Optionally wait for the thread to finish
        // Uncomment if you want to ensure the thread has stopped before continuing
        // if handle.join().is_err() {
        //     eprintln!("Error joining clicking thread");
        // }
    }
}

// Implement the eframe::App trait for our struct
impl App for ClickyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Clicky");
            ui.separator();

            let mut clicking_guard = self.clicking.lock().unwrap();
            let mut cps_guard = self.cps.lock().unwrap();
            let is_clicking = *clicking_guard;
            let clicker_thread = Arc::clone(&self.clicker_thread);

            ui.horizontal(|ui| {
                // Start Button
                if ui
                    .add_enabled(!is_clicking, egui::Button::new("Start Clicking"))
                    .clicked()
                {
                    *clicking_guard = true;
                    let current_cps = *cps_guard;

                    // Start clicking thread when button is clicked
                    start_clicking_thread(&clicker_thread, current_cps);

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

                    // Stop clicking thread when button is clicked
                    stop_clicking_thread(&clicker_thread);

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
            ui.add_enabled(
                !is_clicking, // Disable slider when clicking is active
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
                ("Clicking Inactive", egui::Color32::GOLD)
            };
            ui.label(egui::RichText::new(status_text).color(status_color));
        });

        // Request repaint to ensure UI updates when state changes externally
        ctx.request_repaint();
    }
}

// Clean up any running threads when the app is closed
impl Drop for ClickyApp {
    fn drop(&mut self) {
        // Stop the clicking thread if it's running
        stop_clicking_thread(&self.clicker_thread);
    }
}

fn load_icon(bytes: &[u8]) -> Result<IconData, String> {
    let image = image::load(Cursor::new(bytes), ImageFormat::Png)
        .map_err(|e| e.to_string())?
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
