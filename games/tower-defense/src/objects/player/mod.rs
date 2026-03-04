pub mod character;
pub mod hand;

use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_plugins((character::plugin, hand::plugin));
}
