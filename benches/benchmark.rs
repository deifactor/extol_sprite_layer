use bevy::{log::LogPlugin, prelude::*, winit::WinitPlugin};
use criterion::{criterion_group, criterion_main, Criterion};
use extol_sprite_layer::{LayerIndex, SpriteLayerPlugin};

#[derive(Debug, Clone, Component, Hash, PartialEq, Eq)]
enum SpriteLayer {
    // we only need one 'layer' for the benchmark
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

fn setup_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<WinitPlugin>()
            .disable::<LogPlugin>(),
    )
    .add_plugins(SpriteLayerPlugin::<SpriteLayer>::default());
    for _ in 0..10000 {
        let sprite = Sprite {
            custom_size: Some(Vec2::new(60.0, 60.0)),
            ..default()
        };
        app.world.spawn((
            SpriteBundle {
                sprite,
                transform: Transform::from_xyz(0., fastrand::f32(), 0.),
                ..default()
            },
            SpriteLayer::Middle,
        ));
    }
    while !app.ready() {
        bevy::tasks::tick_global_task_pools_on_main_thread();
    }
    app.finish();
    app.cleanup();
    app
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut app = setup_app();
    c.bench_function("create app", |b| {
        b.iter(|| app.update());
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
