//! An example that demonstrates the effect of y-sorting.
use bevy::{
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
};
use extol_sprite_layer::*;

#[derive(Debug, Clone, Component, PartialEq, Eq, PartialOrd, Ord)]
enum SpriteLayer {
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
        .add_plugin(SpriteLayerPlugin::<SpriteLayer>::default())
        .add_startup_system(spawn_sprites)
        .add_systems((
            add_sprite_layers.run_if(input_just_pressed(KeyCode::Space)),
            remove_sprite_layers.run_if(input_just_released(KeyCode::Space)),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .run();
}

fn spawn_sprites(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    for _ in 0..100 {
        spawn_square(&mut commands);
    }
    info!("Hold space to enable y-sorting");
}

fn spawn_square(commands: &mut Commands) {
    let color = Color::hsl(360.0 * fastrand::f32(), 0.5, 0.5);
    let pos = Transform::from_xyz(
        fastrand::f32() * 200.0 - 100.0,
        fastrand::f32() * 200.0 - 100.0,
        0.0,
    );
    commands.spawn((SpriteBundle {
        sprite: Sprite {
            color,
            custom_size: Some(Vec2::new(50.0, 50.0)),
            ..default()
        },
        transform: pos,
        ..default()
    },));
}

fn add_sprite_layers(mut commands: Commands, entities: Query<Entity, With<Sprite>>) {
    for entity in &entities {
        commands.entity(entity).insert(SpriteLayer::Middle);
    }
    info!("Enabling sprite layers");
}

fn remove_sprite_layers(mut commands: Commands, entities: Query<Entity, With<Sprite>>) {
    for entity in &entities {
        commands.entity(entity).remove::<SpriteLayer>();
    }
    info!("Disabling sprite layers");
}
