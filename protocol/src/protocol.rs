//! The imperative shell: component registration, replication rules, and the
//! systems that feed ECS queries through the pure functions in `shared`.
//!
//! Every system in this file is a loop and two lines. That is deliberate. If a
//! system here starts making decisions, the decision belongs in `shared` where
//! it can be tested without spinning up an `App`.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bevy::{ecs::entity::MapEntities, prelude::*};
use lightyear::{
    input::{config::InputConfig, native::plugin::InputPlugin},
    prediction::registry::PredictionBuilderExt,
    prelude::{input::native::ActionState, AppComponentExt, Predicted},
};
use serde::{Deserialize, Serialize};

use crate::shared;

/// Simulation timestep, shared by both peers. 64Hz matches what the prediction
/// and interpolation buffers are tuned for.
pub const TIMESTEP: f64 = 1.0 / 64.0;

/// `TIMESTEP` as the f32 the pure functions want, computed once.
pub const DT: f32 = TIMESTEP as f32;

/// Port 5002: 5001 is already the shooter's on the same box.
const PORT: u16 = 5002;

/// The server's address as clients see it. Fixed, because clients have to know
/// who to dial before they know anything else.
#[cfg(not(feature = "dev-local"))]
pub const SERVER_ADDR: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(174, 175, 161, 63)), PORT);
/// Local dev: client and server are the same machine.
#[cfg(feature = "dev-local")]
pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PORT);

/// What the server binds. Clients use port 0 so two of them on one machine do
/// not collide.
pub const SERVER_BIND_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), PORT);

/// Where a person is, in world units. The only replicated state floret has.
///
/// Velocity is deliberately absent. It is a pure function of the current input
/// (`shared::velocity`), so sending it would be sending something the receiver
/// could already compute.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Position(pub Vec2);

/// Marks an entity as a person in the hangout.
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerMarker;

/// What a client sends upstream each tick.
///
/// One field: where the stick is pushed. The keyboard and the touchscreen both
/// reduce to this before it ever reaches the network, so the server never learns
/// whether you are on a phone, and no rule has to care.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PlayerInputs {
    /// Desired travel direction: x right, y up, length at most 1.
    pub motion: Vec2,
    /// True on the ticks the person is asking to go back to the middle.
    ///
    /// It rides in the input rather than being its own message on purpose: the
    /// input is already replicated, already predicted, and already rolled back.
    /// A teleport that travels this way is authoritative AND instant, with no
    /// new machinery.
    pub respawn: bool,
}

/// Required by `InputPlugin` whether or not we have entity fields to remap.
/// Entity ids are per-world, so anything carrying one across the wire has to
/// translate it. `PlayerInputs` carries none, so this is empty.
impl MapEntities for PlayerInputs {
    fn map_entities<M: EntityMapper>(&mut self, _: &mut M) {}
}

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<PlayerMarker>().replicate();

        app.component::<Position>()
            .replicate() // goes over the wire
            .predict() // clients simulate it ahead of the server
            // TODO: add interpolation
            .with_rollback_condition(shared::position_changed);

        app.add_plugins(InputPlugin::<PlayerInputs> {
            config: InputConfig {
                // Everyone needs everyone else's inputs: every person in the
                // room is predicted from their stick, our own included, so
                // nobody waits a round trip to see anybody move.
                rebroadcast_inputs: true,
                ..default()
            },
        });
    }
}

/// Client side: advance only the entities lightyear is predicting. The confirmed
/// copies are the server's to move, and touching them would fight rollback.
pub fn simulate_predicted(
    mut players: Query<(&mut Position, &ActionState<PlayerInputs>), With<Predicted>>,
) {
    for (mut pos, action) in &mut players {
        pos.set_if_neq(shared::step(*pos, &action.0, DT));
    }
}

/// Server side: advance every person, because here there is nothing but truth.
///
/// The body is identical to `simulate_predicted` on purpose. Only the query
/// filter differs, and the arithmetic they share lives in `shared::step` so the
/// two cannot drift apart. `set_if_neq` keeps a standing-still player from
/// marking `Position` changed and billing the network for it every tick.
pub fn simulate_authoritative(
    mut players: Query<(&mut Position, &ActionState<PlayerInputs>), With<PlayerMarker>>,
) {
    for (mut pos, action) in &mut players {
        pos.set_if_neq(shared::step(*pos, &action.0, DT));
    }
}
