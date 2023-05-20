#[macro_use]
extern crate rocket;

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use colorgrad::{CustomGradient, Gradient};

use rocket::fs::FileServer;
use rocket::serde::json::Json;
use sacn_unofficial::packet::ACN_SDT_MULTICAST_PORT;
use sacn_unofficial::source::SacnSource;
use serde::{Deserialize, Serialize};

const UNIVERSE: u16 = 1; // Universe the data is to be sent on.
const SYNC_UNI: Option<u16> = None; // Don't want the packet to be delayed on the receiver awaiting synchronisation.
const PRIORITY: u8 = 100; // The priority for the sending data, must be 1-200 inclusive,  None means use default.
#[derive(PartialEq, Serialize, Deserialize, Clone)]

enum Mode {
    Static,
    Scrolling,
}
struct AppState {
    gradient: Gradient,
    mode: Mode,
}
#[derive(Serialize, Deserialize)]
struct JsonTransceiver {
    col1: String,
    col2: String,
    mode: Mode,
}

// Define a struct to hold the shared state with thread-safe access
struct SharedState {
    app_state: Arc<Mutex<AppState>>,
}

// Define an endpoint to read the value of the counter
#[get("/")]
fn read(state: &rocket::State<SharedState>) -> String {
    let state = state.inner().app_state.lock().unwrap();
    state.gradient.at(0.5).to_hex_string()
}

// Define an endpoint to overwrite the value of the counter
#[get("/<hex>")]
fn write(state: &rocket::State<SharedState>, hex: String) -> String {
    let mut state = match state.inner().app_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.gradient = CustomGradient::new()
        .html_colors(&[&hex, &hex])
        .build()
        .unwrap();
    hex
}
#[get("/gradient/<col1>/<col2>")]
fn write_gradient(state: &rocket::State<SharedState>, col1: String, col2: String) -> String {
    let mut state = match state.inner().app_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.gradient = CustomGradient::new()
        .html_colors(&[&col1, &col2, &col1])
        .build()
        .unwrap();
    let middle = state.gradient.at(0.5).to_hex_string();
    drop(state);
    middle
}

#[get("/gradient")]
fn read_gradient(state: &rocket::State<SharedState>) -> Json<JsonTransceiver> {
    let state = match state.inner().app_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    rocket::serde::json::Json(JsonTransceiver {
        col1: state.gradient.at(0.0).to_hex_string(),
        col2: state.gradient.at(0.5).to_hex_string(),
        mode: state.mode.clone(),
    })
}

fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

#[rocket::main]
async fn main() {
    let destination_address: SocketAddr = SocketAddr::new(
        IpAddr::V4("192.168.1.73".parse().unwrap()),
        ACN_SDT_MULTICAST_PORT,
    );

    let dst_ip: Option<SocketAddr> = Some(destination_address); // Sending the data using IP multicast so don't have a destination IP.

    let local_addr: SocketAddr = SocketAddr::new(
        IpAddr::V4("0.0.0.0".parse().unwrap()),
        ACN_SDT_MULTICAST_PORT + 1,
    );

    let mut src: SacnSource = SacnSource::with_ip("Source", local_addr).unwrap();

    src.register_universe(UNIVERSE).unwrap(); // Register with the source that will be sending on the given universe.
                                              // Create the shared state and wrap it in an Arc and a Mutex
    let app_state = Arc::new(Mutex::new(AppState {
        mode: Mode::Scrolling,
        gradient: colorgrad::rainbow(),
    }));
    let shared_state = SharedState { app_state };

    // Spawn a thread that will continuously print the value of the counter
    let shared_state_clone = shared_state.app_state.clone();
    thread::spawn(move || {
        let mut shifter = 0;

        loop {
            let state = shared_state_clone.lock().unwrap();
            let mut data: Vec<u8> = Vec::new();

            if state.mode == Mode::Static {
                shifter = 0;
            } else {
                shifter = (shifter + 1) % 171
            }

            for i in 0..170 {
                let rgb = hex_to_rgb(
                    &state
                        .gradient
                        .at(i as f64 / 170 as f64)
                        .to_hex_string()
                        .trim_start_matches('#'),
                )
                .unwrap();
                data.push(rgb.2); // B
                data.push(rgb.0); // R
                data.push(rgb.1); // G
            }

            data.rotate_right(shifter as usize * 3);

            src.send(&[UNIVERSE], &data, Some(PRIORITY), dst_ip, SYNC_UNI)
                .unwrap();
            drop(state); // Release the lock before waiting
            thread::sleep(Duration::from_millis(10));
        }
    });

    // Start the Rocket server and pass in the shared state
    rocket::build()
        .manage(shared_state)
        .mount("/api", routes![read, write, read_gradient, write_gradient])
        .mount("/", FileServer::from("public"))
        .launch()
        .await
        .unwrap();
}
