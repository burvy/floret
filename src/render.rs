//! The only module that knows floret has pixels.
//!
//! `Position` is the game's idea of where someone is; `Transform` is Bevy's.
//! Keeping the conversion in one system means the simulation never has to own a
//! `Transform`, which is why the server can link the same rules without a
//! renderer at all.

use bevy::prelude::*;
use floret_protocol::{protocol, shared};
use lightyear::prelude::{Controlled, Predicted};

/// Us.
const OWN_COLOR: Color = Color::srgb(0.98, 0.76, 0.28);
/// Everyone else.
const PEER_COLOR: Color = Color::srgb(0.45, 0.72, 0.95);
/// The floor of the hangout.
const GROUND_COLOR: Color = Color::srgb(0.13, 0.14, 0.18);

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(GROUND_COLOR));
        app.add_systems(Startup, spawn_camera);
        // Draw, then place bodies, then place the camera on ours. Chained so a
        // body never renders for a frame at the origin, and so the camera never
        // lags a frame behind the body it is following.
        app.add_systems(
            Update,
            (give_body_a_look, follow_position, follow_player).chain(),
        );
    }
}

fn spawn_camera(mut cmds: Commands) {
    cmds.spawn(Camera2d);
}

/// Give every newly predicted person a sprite.
///
/// Written as a BSN scene rather than a bundle: `queue_apply_scene` patches only
/// the fields named, so `Sprite`'s other defaults survive and a later scene can
/// layer art on top without respecifying these.
fn give_body_a_look(
    mut cmds: Commands,
    arrivals: Query<(Entity, Has<Controlled>), (Added<Predicted>, With<protocol::PlayerMarker>)>,
) {
    for (person, is_us) in &arrivals {
        let color = if is_us { OWN_COLOR } else { PEER_COLOR };
        // Braces around both values: bsn! reads a bare `Some(..)` as an enum
        // *patch* and goes looking for an `Option::default_some()`, so an
        // Option-valued field has to be handed over as a plain expression.
        cmds.entity(person).queue_apply_scene(bsn! {
            Sprite {
                color: {color},
                custom_size: {Some(Vec2::splat(shared::PLAYER_SIZE))},
            }
        });
    }
}

/// Copy `Position` into the `Transform` that Bevy actually draws.
fn follow_position(mut bodies: Query<(&protocol::Position, &mut Transform), With<Predicted>>) {
    for (pos, mut transform) in &mut bodies {
        transform.translation = pos.0.extend(0.0);
    }
}

/// Keep the camera on our own body.
///
/// The world is far larger than any screen, so a fixed camera would leave you
/// walking into blank space within seconds. Snapping rather than easing: a lag
/// between your input and the view is the one thing prediction exists to avoid.
///
/// TODO: add deadzone/lookahead and all the pretty stuff so its not just locked
/// to the player
fn follow_player(
    body: Option<Single<&protocol::Position, (With<Controlled>, With<Predicted>)>>,
    camera: Option<Single<&mut Transform, With<Camera2d>>>,
) {
    // Either can be missing for a frame: the camera spawns on the first tick,
    // and our body only exists once the server has replicated it back to us.
    let (Some(body), Some(mut camera)) = (body, camera) else {
        return;
    };
    // Keep z: in 2D it decides what falls inside the camera's visible range.
    camera.translation = body.0.extend(camera.translation.z);
}
