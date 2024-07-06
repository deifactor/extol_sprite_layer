use bevy::{app::PluginsState, prelude::*};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
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

fn setup_app(count: u64) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(SpriteLayerPlugin::<SpriteLayer>::default());
    for _ in 0..count {
        let sprite = Sprite {
            custom_size: Some(Vec2::new(60.0, 60.0)),
            ..default()
        };
        app.world_mut().spawn((
            SpriteBundle {
                sprite,
                transform: Transform::from_xyz(0., fastrand::f32(), 0.),
                ..default()
            },
            SpriteLayer::Middle,
        ));
    }
    while app.plugins_state() != PluginsState::Ready {
        bevy::tasks::tick_global_task_pools_on_main_thread();
    }
    app.finish();
    app.cleanup();
    app
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("update");
    for count in [1000, 2000, 4000, 8000, 16000] {
        group.throughput(criterion::Throughput::Elements(count));
        group.bench_with_input(BenchmarkId::new("y-sorted", count), &count, |b, &count| {
            let mut app = setup_app(count);
            b.iter(|| app.update());
        });
        group.bench_with_input(BenchmarkId::new("unsorted", count), &count, |b, &count| {
            let mut app = setup_app(count);
            b.iter(|| app.update());
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
