//! Basic demonstration of the plugin's functionality.
use bevy::prelude::*;
use extol_sprite_layer::*;

#[derive(Debug, Clone, Component, Hash, PartialEq, Eq)]
enum SpriteLayer {
    Top,
    Middle(u8),
}

impl LayerIndex for SpriteLayer {
    fn as_z_coordinate(&self) -> f32 {
        use SpriteLayer::*;
        match *self {
            Top => 100.0,
            Middle(z) => (z as f32) / 256.0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SpriteLayerPlugin::<SpriteLayer>::default())
        .add_systems(Startup, spawn_sprites)
        .insert_resource(ClearColor(Color::BLACK))
        // disable y-sorting for simplicity
        .insert_resource(SpriteLayerOptions { y_sort: false })
        .run();
}

fn spawn_sprites(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    // generate a nice color gradient and shuffle it
    let mut color_pos: Vec<(SpriteLayer, Color, Vec3)> = (0..10)
        .map(|i| {
            let i = i as f32;
            (
                SpriteLayer::Middle(i as u8),
                Color::hsl(36.0 * i, 0.5, 0.5),
                Vec3::new(10.0 * i, 10.0 * i, 0.0),
            )
        })
        .collect();

    fastrand::shuffle(&mut color_pos);
    for (layer, color, pos) in color_pos.into_iter() {
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
            layer,
        ));
    }

    // spawn some white squares that should be on top of everything else
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(30.0, 30.0)),
            ..default()
        },
        transform: Transform::from_translation(-50.0 * Vec3::X),
        ..default()
    });
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(30.0, 30.0)),
                ..default()
            },
            transform: Transform::from_translation(110.0 * Vec3::X),
            ..default()
        },
        SpriteLayer::Top,
    ));
}
