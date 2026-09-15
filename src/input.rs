//! Reading a keyboard and a touchscreen, and reducing both to one `Vec2`.
//!
//! The whole point of this module is that the network never learns which device
//! you used. WASD and a thumb both become `PlayerInputs { motion }`, so no rule
//! downstream branches on platform - and `shared::stick` keeps the only piece of
//! real arithmetic here testable.

use bevy::prelude::*;
use floret_protocol::{protocol, shared};
use lightyear::{
    input::client::InputSystems,
    prelude::input::native::{ActionState, InputMarker},
};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        // Must land in lightyear's own set, or the input is written after it has
        // already been packed and sent, and arrives a tick late forever.
        app.add_systems(
            FixedPreUpdate,
            write_input.in_set(InputSystems::WriteClientInputs),
        );
    }
}

/// Publish this frame's stick position for the body we control.
///
/// Touch wins when present: a phone has no keyboard to disagree with, and a
/// laptop with a touchscreen is being driven by whichever one the person is
/// actually holding.
fn write_input(
    action: Option<
        Single<&mut ActionState<protocol::PlayerInputs>, With<InputMarker<protocol::PlayerInputs>>>,
    >,
    keys: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
) {
    // No body yet - we are still connecting, or the server has not replicated
    // ours back to us.
    let Some(mut action) = action else {
        return;
    };

    action.0 = protocol::PlayerInputs {
        motion: touch_motion(&touches).unwrap_or_else(|| key_motion(&keys)),
    };
}

/// WASD as a direction. Also the arrow keys, because someone will try them.
fn key_motion(keys: &ButtonInput<KeyCode>) -> Vec2 {
    let held = |a: KeyCode, b: KeyCode| keys.any_pressed([a, b]);
    let axis = |pos, neg| f32::from(pos) - f32::from(neg);

    Vec2::new(
        axis(
            held(KeyCode::KeyD, KeyCode::ArrowRight),
            held(KeyCode::KeyA, KeyCode::ArrowLeft),
        ),
        axis(
            held(KeyCode::KeyW, KeyCode::ArrowUp),
            held(KeyCode::KeyS, KeyCode::ArrowDown),
        ),
    )
}

/// A virtual joystick with no joystick.
///
/// Wherever the thumb first landed is the centre; how far it has since moved is
/// the deflection. There is nothing to draw and nothing to hit, so it works on
/// any screen size and never lands under a thumb that is somewhere else.
///
/// TODO: add the joystick art
fn touch_motion(touches: &Touches) -> Option<Vec2> {
    let touch = touches.iter().next()?;
    Some(shared::stick(touch.position() - touch.start_position()))
}
