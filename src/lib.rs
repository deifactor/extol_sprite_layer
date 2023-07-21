#![doc = include_str!("../README.md")]
use std::cmp::Reverse;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use bevy::render::RenderApp;
use bevy::sprite::{extract_sprites, queue_sprites, ExtractedSprite, SpriteSystem};
use bevy::{prelude::*, render::Extract, sprite::ExtractedSprites};
use ordered_float::OrderedFloat;
#[cfg(feature = "parallel_y_sort")]
use rayon::slice::ParallelSliceMut;

/// This plugin will modify the z-coordinates of the extracted sprites stored
/// in Bevy's [`ExtractedSprites`] so that they're rendered in the proper
/// order. See the crate documentation for how to use it.
///
/// In general you should only instantiate this plugin with a single type you
/// use throughout your program.
///
/// By default your sprites will also be y-sorted. If you don't need this,
/// replace the [`SpriteLayerOptions`] like so:
///
/// ```
/// # use bevy::prelude::*;
/// # use extol_sprite_layer::SpriteLayerOptions;
/// # let mut app = App::new();
/// app.insert_resource(SpriteLayerOptions { y_sort: false });
/// ```
pub struct SpriteLayerPlugin<Layer> {
    phantom: PhantomData<Layer>,
}

impl<Layer> Default for SpriteLayerPlugin<Layer> {
    fn default() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<Layer: LayerIndex> Plugin for SpriteLayerPlugin<Layer> {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpriteLayerOptions>();
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            ExtractSchedule,
            update_sprite_z_coordinates::<Layer>
                .in_set(SpriteSystem::ExtractSprites)
                .in_set(SpriteLayerSet)
                .after(extract_sprites)
                .before(queue_sprites),
        );
    }
}

/// Configure how the sprite layer
#[derive(Debug, Resource, Reflect)]
pub struct SpriteLayerOptions {
    pub y_sort: bool,
}

impl Default for SpriteLayerOptions {
    fn default() -> Self {
        Self { y_sort: true }
    }
}

/// Set for all systems related to [`SpriteLayerPlugin`]. This is run in the
/// render app's [`ExtractSchedule`], *not* the main app.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, SystemSet)]
pub struct SpriteLayerSet;

/// Trait for the type you use to indicate your sprites' layers. Add this as a
/// component to any entity you want to treat as a sprite. Note that this does
/// *not* propagate.
pub trait LayerIndex: Eq + Hash + Component + Clone + Debug {
    /// The actual numeric z-value that the layer index corresponds to.  Note
    /// that the z-value for an entity can be any value in the range
    /// `layer.as_z_coordinate() <= z < layer.as_z_coordinate() + 1.0`, and the
    /// exact values are an implementation detail!
    ///
    /// With the default Bevy camera settings, your return values from this
    /// function should be between 0 and 999.0, since the camera is at z =
    /// 1000.0. Prefer smaller z-values since that gives more precision.
    fn as_z_coordinate(&self) -> f32;
}

/// Update the z-coordinates of the transform of every sprite with a
/// `LayerIndex` component so that they're rendered in the proper layer with
/// y-sorting.
#[allow(clippy::type_complexity)]
fn update_sprite_z_coordinates<Layer: LayerIndex>(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    options: Extract<Res<SpriteLayerOptions>>,
    transform_query: Extract<Query<(Entity, &GlobalTransform), With<Layer>>>,
    layer_query: Extract<Query<&Layer>>,
) {
    if options.y_sort {
        let z_index_map = map_z_indices(transform_query, layer_query);
        for sprite in extracted_sprites.sprites.iter_mut() {
            if let Some(z) = z_index_map.get(&sprite.entity) {
                set_sprite_coordinate(sprite, *z);
            }
        }
    } else {
        for sprite in extracted_sprites.sprites.iter_mut() {
            if let Ok(layer) = layer_query.get(sprite.entity) {
                set_sprite_coordinate(sprite, layer.as_z_coordinate());
            }
        }
    }
}

/// Sets the z-coordinate of the sprite's transform.
fn set_sprite_coordinate(sprite: &mut ExtractedSprite, z: f32) {
    if sprite.transform.translation().z != 0.0 {
        // not currently disableable, but I'm open if you file an issue :)
        warn!(
            "Entity {:?} has a LabelLayer *and* a nonzero z-coordinate {}; this is probably not what you want!",
            sprite.entity,
            sprite.transform.translation().z
        );
    }
    // hacky hacky; I can't find a way to directly mutate the GlobalTransform.
    let mut affine = sprite.transform.affine();
    affine.translation.z = z;
    sprite.transform = GlobalTransform::from(affine);
}

/// Used to sort the entities within a sprite layer.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ZIndexSortKey(Reverse<OrderedFloat<f32>>);

impl ZIndexSortKey {
    // This is reversed because bevy uses +y pointing upwards, which is the
    // opposite of what you generally want.
    fn new(transform: &GlobalTransform) -> Self {
        Self(Reverse(OrderedFloat(transform.translation().y)))
    }
}

/// Determines the z-value to use for each entity. The z-value is set to
/// `layer.as_z_coordinate() + offset`, where `offset` is calculated so that
/// entities with a higher y-coordinate have a higher offset.
#[allow(clippy::type_complexity)]
fn map_z_indices<Layer: LayerIndex>(
    transform_query: Extract<Query<(Entity, &GlobalTransform), With<Layer>>>,
    layer_query: Extract<Query<&Layer>>,
) -> HashMap<Entity, f32> {
    // We y-sort everything because this avoids the overhead of grouping
    // entities by their layer. Using sort_by_cached_key to make the vec's
    // elements smaller doesn't seem to help here.
    let mut all_entities: Vec<(ZIndexSortKey, Entity)> = transform_query
        .iter()
        .map(|(entity, transform)| (ZIndexSortKey::new(transform), entity))
        .collect();

    // most of the expense is here.
    #[cfg(feature = "parallel_y_sort")]
    all_entities.par_sort_unstable();
    #[cfg(not(feature = "parallel_y_sort"))]
    all_entities.sort_unstable();

    let scale_factor = 1.0 / all_entities.len() as f32;
    all_entities
        .into_iter()
        .enumerate()
        .map(|(i, (_, entity))| {
            (
                entity,
                // NOTE: it's possible that the scale factor will be small
                // enough relative to the z coordinate that these are equal for
                // consecutive values. This occurs when z-coordinate *
                // len(all_entities) > 2^23 (floats have 24 bits of
                // precision). Even with a z-coordinate of 1000, this requires
                // over 8000 entities to hit, which I think is fine.
                layer_query.get(entity).unwrap().as_z_coordinate() + i as f32 * scale_factor,
            )
        })
        .collect()
}
