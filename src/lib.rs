//! The floret client: a window (or a canvas) onto the hangout.
//!
//! Three small modules, each with one job:
//!   - `input`  turns a keyboard or a thumb into a direction
//!   - `net`    talks to the server and owns nothing else
//!   - `render` is the only code allowed to know what a pixel is
//!
//! The rules of the game are not in here. They live in `floret-protocol`, which
//! the server links too.

use bevy::prelude::*;

mod input;
mod net;
mod render;
mod ui;

// Lets the browser start the game by calling `start()` on the wasm module.
// ---
use std::sync::atomic::{AtomicBool, Ordering};

static STARTED: AtomicBool = AtomicBool::new(false);

/// Build and run the app.
///
/// Guarded against a second call: navigating back to the page re-imports the
/// module without tearing down the old one, and two Bevy apps on one canvas is
/// not a thing.
pub fn run() {
    if STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                canvas: Some("#floret-canvas".into()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MainPlugin)
        .run();
}
// ---

pub struct MainPlugin;

impl Plugin for MainPlugin {
    fn build(&self, app: &mut App) {
        // Order matters and lightyear documents it: ClientPlugins (inside
        // NetPlugin) first, then the protocol, then the Client entity is
        // spawned by NetPlugin's own Startup system.
        app.add_plugins(net::NetPlugin);
        app.add_plugins(floret_protocol::protocol::ProtocolPlugin);
        app.add_plugins(input::InputPlugin);
        app.add_plugins(render::RenderPlugin);
        app.add_plugins(ui::UiPlugin);
    }
}
