//! Listening, admitting people, and being the authority on where everyone is.

use std::time::Duration;

use bevy::prelude::*;
use floret_protocol::{protocol, shared};
use lightyear::{
    connection::{client::Connected, client_of::ClientOf, server::Start},
    core::timeline::LocalTimeline,
    input::input_buffer::InputBuffer,
    netcode::{server_plugin::NetcodeConfig, NetcodeServer},
    prelude::{input::native::ActionState, server::ServerPlugins, *},
    webtransport::server::WebTransportServerIo,
};

// The browser will not open a WebTransport connection to an untrusted
// certificate, and this server answers on a bare IP rather than a name, so the
// cert has to be issued for the IP. Let's Encrypt will do that under its
// short-lived profile. Renew with Posh-ACME, in PowerShell:
//
//   Install-Module -Name Posh-ACME -Scope CurrentUser
//   Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
//   Import-Module Posh-ACME
//   Set-PAServer LE_PROD
//   New-PACertificate '192.0.2.1' `      # your public IP
//     -Plugin WebRoot `
//     -PluginArgs @{ WRPath = 'C:\acme-challenge' } `
//     -Profile shortlived `
//     -AcceptTOS `
//     -Contact 'you@example.com'
//
// floret shares the shooter's certificate: same machine, same IP, different
// port, and a cert says nothing about ports.
#[cfg(not(feature = "dev-local"))]
const KEY_PATH: &str =
    r"C:\Users\Burvy\AppData\Local\Posh-ACME\LE_PROD\3716529536\174.175.161.63\cert.key";
#[cfg(not(feature = "dev-local"))]
const CERT_PATH: &str =
    r"C:\Users\Burvy\AppData\Local\Posh-ACME\LE_PROD\3716529536\174.175.161.63\fullchain.cer";

/// Ticks of silence before we stop replaying someone's last input. 128 ticks at
/// 64Hz is two seconds.
const STALE_INPUT_TICKS: i32 = 128;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_secs_f64(protocol::TIMESTEP),
        });
        app.add_systems(Startup, startup);
        app.add_observer(on_connect);
        // Zeroing a stale input has to happen before we act on it, otherwise we
        // walk someone for one more tick on input we already decided to ignore.
        app.add_systems(
            FixedUpdate,
            (clear_stale_input, protocol::simulate_authoritative).chain(),
        );
        app.add_systems(Update, heartbeat);
    }
}

/// Open the socket and start listening.
fn startup(mut cmds: Commands) -> Result {
    #[cfg(not(feature = "dev-local"))]
    let identity = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(Identity::load_pemfiles(CERT_PATH, KEY_PATH))?;

    // Offline dev: a throwaway cert made at startup. No files, no network. The
    // client's own dev-local skips validation, so the contents do not matter.
    #[cfg(feature = "dev-local")]
    let identity = Identity::self_signed(["localhost", "127.0.0.1"])?;

    let server = cmds
        .spawn((
            NetcodeServer::new(NetcodeConfig {
                client_timeout_secs: 300,
                ..default()
            }),
            LocalAddr(protocol::SERVER_BIND_ADDR),
            WebTransportServerIo {
                certificate: identity,
            },
        ))
        .id();

    cmds.trigger(Start { entity: server });
    Ok(())
}

/// Someone arrived: give their connection an outbox and put a body in the room.
fn on_connect(
    trigger: On<Add, Connected>,
    clients: Query<(), With<ClientOf>>,
    mut cmds: Commands,
) {
    // Something connected that is not a client of ours.
    if !clients.contains(trigger.entity) {
        return;
    }

    // `ControlledBy` looks for `With<ReplicationSender>`, so this comes first.
    cmds.entity(trigger.entity).insert(ReplicationSender);

    cmds.spawn((
        protocol::PlayerMarker,
        protocol::Position(shared::SPAWN),
        Replicate::to_clients(NetworkTarget::All),
        // Everyone predicts everyone. Combined with rebroadcast inputs, a peer's
        // movement appears the moment their input arrives rather than a round
        // trip later.
        PredictionTarget::to_clients(NetworkTarget::All),
        // The server owns this body and says which connection drives it. That
        // fact replicates, arriving at the owning client as a `Controlled`
        // marker, which is how the client knows whose input to write.
        ControlledBy {
            owner: trigger.entity,
            // SessionBased: the body leaves when the person does.
            lifetime: Default::default(),
        },
    ));
}

/// Lightyear replays a quiet client's last input forever, which with our long
/// timeout means someone who closed their laptop mid-stride keeps walking. Once
/// they have been silent for `STALE_INPUT_TICKS`, put the stick back to centre.
fn clear_stale_input(
    timeline: Res<LocalTimeline>,
    mut players: Query<
        (
            &mut ActionState<protocol::PlayerInputs>,
            &InputBuffer<ActionState<protocol::PlayerInputs>, protocol::PlayerInputs>,
        ),
        With<protocol::PlayerMarker>,
    >,
) {
    let tick = timeline.tick();
    for (mut action, buffer) in &mut players {
        let quiet = buffer
            .last_remote_tick
            .is_none_or(|last| tick - last > STALE_INPUT_TICKS);

        if quiet && action.0 != protocol::PlayerInputs::default() {
            action.0 = protocol::PlayerInputs::default();
        }
    }
}

/// Say who is here, once a minute, so the log shows life.
fn heartbeat(
    clients: Query<(), (With<ClientOf>, With<Connected>)>,
    players: Query<(), With<protocol::PlayerMarker>>,
    time: Res<Time>,
    mut next: Local<f32>,
) {
    if time.elapsed_secs() < *next {
        return;
    }
    *next = time.elapsed_secs() + 60.0;
    info!(
        "clients: {}, people in the room: {}",
        clients.iter().count(),
        players.iter().count(),
    );
}
