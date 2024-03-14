#![doc = include_str!("../README.md")]
use std::cmp::Reverse;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::transform::TransformSystem;
use ordered_float::OrderedFloat;
#[cfg(feature = "parallel_y_sort")]
use rayon::slice::ParallelSliceMut;

/// This plugin adjusts your entities' transforms so that their z-coordinates are sorted in the
/// proper order, where the order is specified by the `Layer` component. Note that since this sets
/// the z-coordinate, the children of a component with a sprite layer will effectively be on the
/// same sprite layer (though you can override this by giving them a sprite layer of their own). See
/// the crate documentation for how to use it.
///
/// In general you should only instantiate this plugin with a single type you use throughout your
/// program.
///
/// By default your sprites will also be y-sorted. If you don't need this, replace the
/// [`SpriteLayerOptions`] like so:
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
        app.init_resource::<SpriteLayerOptions>().add_systems(
            PostUpdate,
            // We need to run these systems *after* the transform's systems because they need the
            // proper y-coordinate to be set for y-sorting.
            (
                compute_target_z_coordinates::<Layer>.pipe(set_transform_from_layer::<Layer>),
                bevy::transform::systems::sync_simple_transforms,
                bevy::transform::systems::propagate_transforms,
            )
                .chain()
                .in_set(SpriteLayerSet)
                .after(TransformSystem::TransformPropagate),
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

/// Compute the z-coordinate that each entity should have. This is equal to its layer's equivalent
/// z-coordinate, plus an offset in the range [0, 1) corresponding to its y-sorted position
/// (if y-sorting is enabled).
pub fn compute_target_z_coordinates<Layer: LayerIndex>(
    query: Query<(Entity, &GlobalTransform, &Layer)>,
    options: Res<SpriteLayerOptions>,
) -> HashMap<Entity, f32> {
    let mut z_map: HashMap<Entity, f32> = query
        .iter()
        .map(|(entity, _, layer)| (entity, layer.as_z_coordinate()))
        .collect();
    if options.y_sort {
        // We y-sort everything because this avoids the overhead of grouping
        // entities by their layer. Using sort_by_cached_key to make the vec's
        // elements smaller doesn't seem to help here.
        let mut all_entities: Vec<(ZIndexSortKey, Entity)> = query
            .iter()
            .map(|(entity, transform, _)| (ZIndexSortKey::new(transform), entity))
            .collect();

        // most of the expense is here.
        #[cfg(feature = "parallel_y_sort")]
        all_entities.par_sort_unstable();
        #[cfg(not(feature = "parallel_y_sort"))]
        all_entities.sort_unstable();

        let scale_factor = 1.0 / all_entities.len() as f32;
        for (i, (_, entity)) in all_entities.into_iter().enumerate() {
            dbg!(i, scale_factor);
            *z_map.get_mut(&entity).unwrap() += (i as f32) * scale_factor;
        }
    }
    z_map
}

pub type NeedsLayerUpdate<Layer> = (With<Layer>, Or<(Changed<Transform>, Changed<Layer>)>);

/// Update the z-transform of each entity with a [`Layer`] component so that its final [`GlobalTransform`]
/// will have the z-axis set properly.
pub fn set_transform_from_layer<Layer: LayerIndex>(
    In(z_map): In<HashMap<Entity, f32>>,
    mut query: Query<(Entity, &mut Transform), NeedsLayerUpdate<Layer>>,
    parent_query: Query<&Parent>,
) {
    for (entity, mut transform) in query.iter_mut() {
        // We want to set the z-coordinate of the GlobalTransform to be `layer.as_z_coordinate()`.
        // If we assume that this will be true of all of our ancestors (after transform propagation),
        // then we just need to set the z-coordinate of this entity's transform to entity.z - ancestor.z.
        let ancestor_z = parent_query
            .iter_ancestors(entity)
            .find_map(|e| z_map.get(&e));
        let Some(entity_z) = z_map.get(&entity) else {
            error!("entity {entity:?} at {} somehow doesn't have a z-map entry; this is a bug in extol_sprite_layer", transform.translation);
            continue;
        };
        let offset = entity_z - ancestor_z.unwrap_or(&0.0);
        transform.translation.z = offset;
    }
}

/// Used to sort the entities within a sprite layer.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ZIndexSortKey(Reverse<OrderedFloat<f32>>);

impl ZIndexSortKey {
    // This is reversed because bevy uses +y pointing upwards, which is the
    // opposite of what you generally want.
    fn new(transform: &GlobalTransform) -> Self {
        Self(Reverse(OrderedFloat(transform.translation().y)))
    }
}
