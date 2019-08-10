use common::{clock::Clock, logging::VelorenLogger};
use heaptrack::track_mem;
use log::info;
use server::{Event, Input, Server, ServerSettings};
use std::time::Duration;

track_mem!();

const TPS: u64 = 30;

fn main() {
    // Init logging
    VelorenLogger::new()
        .with_term(&log::LevelFilter::Info)
        .apply();

    info!("Starting server-cli...");

    // Set up an fps clock
    let mut clock = Clock::start();

    // Load settings
    let settings = ServerSettings::load();

    // Create server
    let mut server = Server::new(settings).expect("Failed to create server instance!");

    loop {
        let events = server
            .tick(Input::default(), clock.get_last_delta())
            .expect("Failed to tick server");

        for event in events {
            match event {
                Event::ClientConnected { entity: _ } => info!("Client connected!"),
                Event::ClientDisconnected { entity: _ } => info!("Client disconnected!"),
                Event::Chat { entity: _, msg } => info!("[Client] {}", msg),
            }
        }

        // Clean up the server after a tick.
        server.cleanup();

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));
    }
}
