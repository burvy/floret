// Bevy query types are inherently verbose; a type alias for a single-use query
// reads worse than the query. Bevy's own crates allow this too.
#![allow(clippy::type_complexity)]

use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*, state::app::StatesPlugin};
use floret_protocol::protocol;

mod server;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        // MinimalPlugins, not DefaultPlugins: there is no window to open and no
        // pixel to draw. The runner ticks faster than the timestep so a tick is
        // never late just because the loop woke up at the wrong moment.
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            protocol::TIMESTEP / 4.0,
        ))),
        StatesPlugin,
        LogPlugin::default(),
    ));
    app.add_plugins(server::ServerPlugin);
    app.add_plugins(protocol::ProtocolPlugin);
    app.run();
}
