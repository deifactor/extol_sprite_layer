use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::render::RenderApp;
use bevy::sprite::{extract_sprites, queue_sprites, SpriteSystem};
use bevy::{prelude::*, render::Extract, sprite::ExtractedSprites};

/// Adjusts the z-coordinates of your extracted sprites so that they render in
/// the proper order. To use this:
/// - Define some `SpriteLayer` type that implements the [`LayerIndex`] trait.
/// - Add it as a component to all of your sprite entities.
/// - Add this plugin to your app.
///
/// Caveats:
/// - The sprite layer is not propagated from parent to child.
/// - If you have a [`LayerLabel`] set on a component with a non-zero Z
///   coordinate, this will print out a warning, since that z-coordinate will be
///   effectively ignored.
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
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_system(
                update_sprite_z_coordinates::<Layer>
                    .in_set(SpriteSystem::ExtractSprites)
                    .after(extract_sprites)
                    .before(queue_sprites)
                    .in_schedule(ExtractSchedule),
            );
        };
    }
}

/// Trait for thigns that are used to indicate what z-layer a sprite should be
/// on.
///
/// - The [`Ord`] instance is needed since we 'bucket' sprites by their layer in
///   a BTreeMap.
pub trait LayerIndex: Ord + Component + Clone + Debug {
    /// The actual numeric z-value that the layer index corresponds to.  Note
    /// that the *actual* z-value may be up to `layer.as_z_coordinate() <= z <
    /// layer.as_z_coordinate() + 1.0`, since y-sorting is done by adding to
    /// the z-axis. So your z-values should always be at least 1.0 apart.
    fn as_z_coordinate(&self) -> f32;
}

/// Update the z-coordinates of the transform of every sprite with a
/// `LayerLabel` component so that they're rendered in the proper layer with
/// y-sorting.
#[allow(clippy::type_complexity)]
pub fn update_sprite_z_coordinates<Layer: LayerIndex>(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    z_index_query: Extract<Query<(Entity, &Layer, &GlobalTransform)>>,
) {
    let z_index_map = map_z_indices(z_index_query);
    for sprite in extracted_sprites.sprites.iter_mut() {
        if let Some(z) = z_index_map.get(&sprite.entity) {
            if sprite.transform.translation().z != 0.0 {
                warn!("Entity {:?} has a LabelLayer *and* a nonzero z-coordinate {}; this is probably not what you want!", sprite.entity, sprite.transform.translation().z);
            }
            let mut affine = sprite.transform.affine();
            affine.translation.z = *z;
            sprite.transform = GlobalTransform::from(affine);
        }
    }
}

/// Used to sort the entities within a sprite layer.
#[derive(Debug, PartialEq, PartialOrd)]
struct ZIndexSortKey(f32);

impl ZIndexSortKey {
    /// How much to increase the z-axis by. Should always be in `0 <= offset <
    /// 1`.
    fn offset(self) -> f32 {
        self.0 / 4096.0
    }
}

/// Determines the z-value to use for each entity. The z-value is set to
/// `f32::from(layer) + offset`, where `offset` is calculated so that entities
/// with a higher y-coordinate have a higher offset. Entities that are not
/// visible are omitted in the result.
///
/// The returned value is guaranteed to be sorted in increasing z-coordinate
/// value.
#[allow(clippy::type_complexity)]
fn map_z_indices<Layer: LayerIndex>(
    query: Extract<Query<(Entity, &Layer, &GlobalTransform)>>,
) -> HashMap<Entity, f32> {
    let mut by_layer: BTreeMap<&Layer, Vec<(ZIndexSortKey, Entity)>> = BTreeMap::new();
    for (entity, layer, transform) in query.iter() {
        by_layer
            .entry(layer)
            .or_default()
            .push((ZIndexSortKey(-transform.translation().y), entity));
    }

    by_layer
        .into_iter()
        .flat_map(|(layer, mut entities)| {
            let layer_z = layer.as_z_coordinate();
            entities.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
            entities
                .into_iter()
                .map(move |(key, entity)| (entity, layer_z + key.offset()))
        })
        .collect()
}
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
