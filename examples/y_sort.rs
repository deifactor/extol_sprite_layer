//! An example that demonstrates the effect of y-sorting. The two sets of
//! squares have the same coordinates, but the one on the right uses sprite
//! layers and so is y-sorted. Tap space to toggle y-sorting.
use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use extol_sprite_layer::*;

#[derive(Debug, Clone, Component, Hash, PartialEq, Eq)]
enum SpriteLayer {
    // we only need one 'layer' for the demo
    Middle,
}

impl LayerIndex for SpriteLayer {
    fn as_z_coordinate(&self) -> f32 {
        use SpriteLayer::*;
        match *self {
            Middle => 1.,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SpriteLayerPlugin::<SpriteLayer>::default())
        .add_systems(Startup, spawn_sprites)
        .add_systems(
            Update,
            toggle_y_sort.run_if(input_just_pressed(KeyCode::Space)),
        )
        .insert_resource(ClearColor(Color::BLACK))
        .run();
}

fn spawn_sprites(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    // generate a nice color gradient and shuffle it
    let mut color_pos: Vec<(Color, Vec3)> = (0..10)
        .map(|i| {
            let i = i as f32;
            (
                Color::hsl(36.0 * i, 0.5, 0.5),
                Vec3::new(10.0 * i, 10.0 * i, 0.0),
            )
        })
        .collect();

    fastrand::shuffle(&mut color_pos);
    for (color, pos) in color_pos.into_iter() {
        let sprite = Sprite {
            color,
            custom_size: Some(Vec2::new(60.0, 60.0)),
            ..default()
        };

        commands.spawn((SpriteBundle {
            sprite: sprite.clone(),
            transform: Transform::from_translation(pos - 80.0 * Vec3::X),
            ..default()
        },));
        commands.spawn((
            SpriteBundle {
                sprite: sprite.clone(),
                transform: Transform::from_translation(pos + 80.0 * Vec3::X),
                ..default()
            },
            SpriteLayer::Middle,
        ));
    }

    info!("Tap space to toggle y-sorting.");
}

fn toggle_y_sort(mut options: ResMut<SpriteLayerOptions>) {
    options.y_sort = !options.y_sort;
    info!("y sort is now {}", options.y_sort);
}
