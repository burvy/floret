//! The functional core: every rule in floret, as a pure function.
//!
//! Nothing here reads the clock, touches the network, or mutates the world. A
//! function in this file, given the same arguments, returns the same answer on
//! the client and on the server - which is exactly the property client-side
//! prediction needs. When prediction and truth disagree, the player sees
//! rubberbanding, and the cheapest way to never disagree is to have only one
//! copy of the arithmetic.
//!
//! The ECS systems that call these live in `protocol.rs`. They are three lines
//! each and contain no rules at all.

use bevy::prelude::*;

use crate::protocol::{PlayerInputs, Position};

/// How fast a person walks, in world units per second.
pub const SPEED: f32 = 220.0;

/// Half-extents of the world. Square, because nothing about an open world is
/// tied to a screen's aspect ratio.
///
/// This is a safety rail, not a room: 40,000 units across is about three minutes
/// of walking corner to corner, so you meet it only by deciding to. It exists at
/// all because f32 loses precision as coordinates grow, and an unbounded world
/// lets someone walk until positions get too coarse for rollback to compare. At
/// this distance an f32 step is about 0.002 units, comfortably finer than the
/// 0.25 that `position_changed` cares about.
pub const BOUNDS: Vec2 = Vec2::splat(20_000.0);

/// Side length of a person's sprite.
pub const PLAYER_SIZE: f32 = 28.0;

/// Drag distance, in physical pixels, that counts as full stick deflection.
pub const JOYSTICK_RADIUS: f32 = 64.0;

/// Where a person appears when they first join: the middle.
pub const SPAWN: Vec2 = Vec2::ZERO;

/// One tick of movement for one person.
///
/// This is the whole simulation. `dt` is passed in rather than read from a
/// clock so that the function stays pure and so that rollback can re-run it for
/// past ticks without lying about how much time passed.
pub fn step(pos: Position, input: &PlayerInputs, dt: f32) -> Position {
    advance(pos, velocity(input), dt)
}

/// Desired velocity from a stick reading.
///
/// `clamp_length_max` rather than `normalize`: a half-pushed stick should walk
/// at half speed, but a diagonal keyboard press (1, 1) must not walk 1.41x
/// faster than a straight one.
pub fn velocity(input: &PlayerInputs) -> Vec2 {
    input.motion.clamp_length_max(1.0) * SPEED
}

/// Integrate a position and keep it inside the hangout.
pub fn advance(pos: Position, vel: Vec2, dt: f32) -> Position {
    Position((pos.0 + vel * dt).clamp(-BOUNDS, BOUNDS))
}

/// Turn a touch drag into a stick reading in the same space the keyboard
/// produces: x right, y up, length at most 1.
///
/// Screen coordinates grow downward while the world grows upward, so y flips
/// here. Getting this wrong is invisible on a keyboard and instantly obvious on
/// a phone, which is why it is one named function with a test rather than a
/// stray minus sign inside an input system.
pub fn stick(drag: Vec2) -> Vec2 {
    (Vec2::new(drag.x, -drag.y) / JOYSTICK_RADIUS).clamp_length_max(1.0)
}

/// Rollback test for `Position`.
///
/// Floats that were computed in a different order are never bit-identical, so
/// an exact comparison would roll back constantly. A quarter of a unit is far
/// below what an eye can see at this sprite size.
pub fn position_changed(this: &Position, that: &Position) -> bool {
    this.0.distance(that.0) >= 0.25
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(motion: Vec2) -> PlayerInputs {
        PlayerInputs { motion }
    }

    #[test]
    fn diagonal_is_not_faster_than_straight() {
        let straight = velocity(&input(Vec2::new(1.0, 0.0))).length();
        let diagonal = velocity(&input(Vec2::new(1.0, 1.0))).length();
        assert!((straight - diagonal).abs() < 0.01, "{straight} vs {diagonal}");
        assert!((straight - SPEED).abs() < 0.01);
    }

    #[test]
    fn half_stick_walks_at_half_speed() {
        let half = velocity(&input(Vec2::new(0.5, 0.0))).length();
        assert!((half - SPEED / 2.0).abs() < 0.01, "{half}");
    }

    #[test]
    fn nobody_leaves_the_world() {
        // Ten minutes of walking right - well past the edge - and the rail holds.
        let far = (0..38_400).fold(Position(SPAWN), |pos, _| {
            step(pos, &input(Vec2::X), 1.0 / 64.0)
        });
        assert_eq!(far.0.x, BOUNDS.x);
    }

    #[test]
    fn the_edge_is_far_enough_away_to_be_a_walk() {
        // If this ever drops under a minute, BOUNDS has stopped meaning "open".
        let seconds_to_the_edge = BOUNDS.x / SPEED;
        assert!(seconds_to_the_edge > 60.0, "{seconds_to_the_edge}s");
    }

    #[test]
    fn standing_still_does_not_drift() {
        let start = Position(Vec2::new(12.0, -30.0));
        assert_eq!(step(start, &input(Vec2::ZERO), 1.0 / 64.0), start);
    }

    #[test]
    fn dragging_down_the_screen_walks_down_the_world() {
        // Screen y grows downward, world y grows upward.
        assert!(stick(Vec2::new(0.0, JOYSTICK_RADIUS)).y < 0.0);
        assert!(stick(Vec2::new(0.0, -JOYSTICK_RADIUS)).y > 0.0);
    }

    #[test]
    fn a_long_drag_is_still_only_full_deflection() {
        assert!(stick(Vec2::splat(4000.0)).length() <= 1.0001);
    }

    #[test]
    fn rollback_ignores_float_noise_but_not_real_movement() {
        let here = Position(Vec2::ZERO);
        assert!(!position_changed(&here, &Position(Vec2::new(0.001, 0.0))));
        assert!(position_changed(&here, &Position(Vec2::new(5.0, 0.0))));
    }
}
