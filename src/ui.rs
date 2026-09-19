//! On-screen controls. Right now that is one button: go back to the middle.
//!
//! A UI click is an *event* - it happens once, whenever the finger lands. The
//! input system in `input.rs` runs on a fixed tick and needs to know "was the
//! button pressed since I last looked?". This module bridges the two with one
//! bool: the observer sets it, `input.rs` reads and clears it.

use bevy::prelude::*;

/// Set by the button, read and cleared by `input.rs` on the next tick.
///
/// A resource rather than a component because there is only ever one of these,
/// and the input system should not have to go hunting for an entity to ask.
#[derive(Resource, Default)]
pub struct RespawnPressed(pub bool);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RespawnPressed>();
        app.add_systems(Startup, spawn_respawn_button);
    }
}

fn spawn_respawn_button(mut cmds: Commands) {
    let button = cmds
        .spawn((
            // Button brings Node, Interaction and FocusPolicy::Block with it via
            // #[require], so picking works without listing them.
            Button,
            Node {
                // Absolute, or it would be laid out in a row with nothing.
                // Bottom-right is the easy reach for a thumb.
                position_type: PositionType::Absolute,
                bottom: Val::Px(24.0),
                right: Val::Px(24.0),
                // Generous padding: this has to be tappable on glass, not
                // clickable with a mouse pointer.
                padding: UiRect::axes(Val::Px(20.0), Val::Px(14.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .id();

    // The label is its own entity parented to the button. `ChildOf` is the 0.19
    // relationship component - setting it makes Bevy maintain `Children` on the
    // parent for us.
    cmds.spawn((
        Text::new("go to middle"),
        TextColor(Color::WHITE),
        ChildOf(button),
    ));

    // Observers attach to an entity, so this fires only for THIS button.
    cmds.entity(button).observe(
        |_: On<Pointer<Click>>, mut pressed: ResMut<RespawnPressed>| {
            pressed.0 = true;
        },
    );
}
