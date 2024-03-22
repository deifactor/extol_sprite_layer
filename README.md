`extol_sprite_layer` lets you specify the drawing order for sprites in your [Bevy](https://bevyengine.org/) game using a separate component rather than via the z-coordinate of your transforms.

## How to use

```rust
use bevy::prelude::*;
use extol_sprite_layer::{LayerIndex, SpriteLayerPlugin, SpriteLayerOptions};

// Define a type to represent your layers. All the traits here other than Copy
// are mandatory.
#[derive(Debug, Copy, Clone, Component, PartialEq, Eq, Hash)]
enum SpriteLayer {
    Background,
    Object,
    Enemy,
    Player,
    Ui,
}

impl LayerIndex for SpriteLayer {
    // Convert your type to an actual z-coordinate.
    fn as_z_coordinate(&self) -> f32 {
        use SpriteLayer::*;
        match *self {
            // Note that the z-coordinates must be at least 1 apart...
            Background => 0.,
            Object => 1.,
            Enemy => 2.,
            // ... but can be more than that.
            Player => 990.,
            Ui => 995.
        }
    }
}

let mut app = App::new();
// Then, add the plugin to your app.
app
  .add_plugins(DefaultPlugins)
  .add_plugins(SpriteLayerPlugin::<SpriteLayer>::default());

// Now just use SpriteLayer as a component and don't set the z-component on any
// of your transforms.

// To disable y-sorting, do
app.insert_resource(SpriteLayerOptions { y_sort: false });
```

### Caveats

Broadly speaking, it does the following:

1. In the `Last` schedule, it Sets the z-coordinate on the `GlobalTransform` (and *not* the `Transform`) for every entity with a layer (and their descendants)
2. In the `First` schedule, it re-zeros their `GlobalTransform`'s z-coordinate.
3. In both cases, it *skips change detection*.

This works because subapps are all run after your main app's `Main` schedule.

This is ugly, and may break things in subtle ways, but it has the properties that:

- sprite layers are inherited
- your application code can compute distances without having to truncate the z-coordinate of displacements (which you would have to do if the z-coordinates were not zeroed)

## Motivation

When making a 2D game in [bevy](https://bevyengine.org/), the z-coordinate is essentially used as a layer index: things with a higher z-coordinate are rendered on top of things with a lower z-coordinate. This works, but it has a few problems:

The z-coordinate isn't relevant for things like distance or normalization. The following code is subtly wrong:

```rust
use bevy::prelude::*;

/// Get a unit vector pointing from the position of `source` to `target`, or
/// zero if they're close.
fn get_normal_direction(query: Query<&GlobalTransform>, source: Entity, target: Entity) -> Vec2 {
    let from_pos = query.get(source).unwrap().translation();
    let to_pos = query.get(target).unwrap().translation();
    (from_pos - to_pos).normalize_or_zero().truncate()
}
```

The bug is that we normalize *before* truncating, so the z-coordinate still 'counts' for purposes of length. So if the source is at `(0, 0, 0)` and the target is at `(0, 1, 100)`, then we'll return `(0, 0.001)`!

Entities on the same layer are drawn in an effectively arbitrary order that can change between frames. If your game can have entities on the same layer overlap with each other, this means entities will 'flip-flop' back and forth. This is distracting. The usual solution is *y-sorting*, where entities on the same layer are sorted by their y-coordinate, so enemies lower on the screen are drawn on top. Here's an example from my WIP game [tengoku](https://codeberg.org/ext0l/tengoku), which is what led me to develop this crate. In both images, all the enemies (the blue 'soldiers') are on the same layer. In the first one, enemies are drawn roughly in spawn order, which makes the pile appear disorganized and unnatural. The second is y-sorted, resulting in a much cleaner-looking pile. (The red robot in the center, representing the player, is on a higher layer in both cases.)

![non-y-sorted enemies piled up in a disorderly way](./docs/before.png)
![y-sorted enemies in a much cleaner pile](./docs/after.png)


## Performance

If y-sorting is enabled (the default), this plugin is `O(N log N)`, where `N` is the number of entities with sprite layers. In benchmarks on my personal machine (a System76 Lemur Pro 10), with 10000 sprites, the plugin added about 600us of overhead with y-sorting; Disabling the `rayon` feature (or running on a single-threaded runtime) roughly doubles this.

If y-sorting is *not* enabled then the overhead is `O(N)` and not significant enough to worry about.

## Known issues

- For performance reasons, y-sorting sorts *all* entities at once. This means that if the product of a layer's z-coordinate with the number of sprites is larger than 2^23 or so, you can run into floating point precision issues.

## Help

Feel free to ping me in one of the channels on [the Bevy Discord server](https://discord.com/invite/bevy); I'm @Sera.
