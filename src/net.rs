//! Dialing the server and letting it tell us who is in the room.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use bevy::prelude::*;
use floret_protocol::protocol;
use lightyear::{
    netcode::{client_plugin::NetcodeConfig, Key, NetcodeClient},
    prelude::{client::ClientPlugins, input::native::InputMarker, *},
    webtransport::client::WebTransportClientIo,
};

/// Port 0: the OS picks a free one. Fixing it would stop two clients on one
/// machine from connecting at once, which is exactly how we test.
const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ClientPlugins {
            tick_duration: Duration::from_secs_f64(protocol::TIMESTEP),
        });
        app.add_systems(Startup, connect);
        app.add_systems(FixedUpdate, protocol::simulate_predicted);
        app.add_observer(claim_our_body);
    }
}

/// Open the connection.
fn connect(mut cmds: Commands) -> Result {
    let auth = Authentication::Manual {
        server_addr: protocol::SERVER_ADDR,
        client_id: getrandom::u64()?, // TODO: use accounts
        private_key: Key::default(),
        protocol_id: 0,
    };

    let client = cmds
        .spawn((
            // NetcodeClient requires Link and Client, and inserts PeerAddr from
            // the address inside `auth`, so none of those are listed here.
            NetcodeClient::new(auth, NetcodeConfig::default())?,
            LocalAddr(CLIENT_ADDR),
            WebTransportClientIo {
                // Empty: the server presents a real Let's Encrypt certificate,
                // so the browser validates it the ordinary way and we have no
                // digest to pin. dev-local skips validation instead.
                certificate_digest: String::new(),
            },
            // Opt in to receiving replicated entities and to simulating them
            // ahead of the server.
            ReplicationReceiver,
            PredictionManager::default(),
        ))
        .id();

    cmds.trigger(Connect { entity: client });
    Ok(())
}

/// The server marks one replicated body as ours. Tag it so the input module
/// knows which `ActionState` to write.
fn claim_our_body(ours: On<Add, Controlled>, mut cmds: Commands) {
    cmds.entity(ours.entity)
        .insert(InputMarker::<protocol::PlayerInputs>::default());
}
