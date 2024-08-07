#![doc = include_str!("../README.md")]
use std::cmp::Reverse;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use bevy::ecs::entity::EntityHashMap; // noticeably faster than std's
use bevy::prelude::*;
use ordered_float::OrderedFloat;
use tap::Tap;

/// This plugin adjusts your entities' transforms so that their z-coordinates are sorted in the
/// proper order, where the order is specified by the `Layer` component. Layers propagate to
/// children (including through entities with no )
///
/// Layers propagate to children, including 'through' entities with no [`GlobalTransform`].
///
/// If you need to know the z-coordinate, you can read it out of the [`GlobalTransform`] after the
/// [`SpriteLayer::SetZCoordinates`] set has run.
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
        app.init_resource::<SpriteLayerOptions>()
            .add_systems(
                First,
                clear_z_coordinates.in_set(SpriteLayerSet::ClearZCoordinates),
            )
            .add_systems(
                Last,
                // We need to run these systems *after* the transform's systems because they need the
                // proper y-coordinate to be set for y-sorting.
                (propagate_layers::<Layer>.pipe(set_z_coordinates::<Layer>),)
                    .chain()
                    .in_set(SpriteLayerSet::SetZCoordinates),
            )
            .register_type::<RenderZCoordinate>();
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
pub enum SpriteLayerSet {
    ClearZCoordinates,
    SetZCoordinates,
}

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

/// Clears the z-coordinate of everything with a `RenderZCoordinate` component.
pub fn clear_z_coordinates(mut query: Query<&mut Transform, With<RenderZCoordinate>>) {
    for mut transform in query.iter_mut() {
        transform.bypass_change_detection().translation.z = 0.0;
    }
}

/// Propagates the `Layer` of each entity to the `InheritedLayer` of itself and all of its
/// descendants.
pub fn propagate_layers<Layer: LayerIndex>(
    recursive_query: Query<(Option<&Children>, Option<&Layer>)>,
    root_query: Query<(Entity, &Layer), Without<Parent>>,
    mut size: Local<usize>,
) -> EntityHashMap<Layer> {
    let mut layer_map = EntityHashMap::default();
    layer_map.reserve(*size);
    for (entity, layer) in &root_query {
        propagate_layers_impl(entity, layer, &recursive_query, &mut layer_map);
    }
    *size = size.max(layer_map.len());
    layer_map
}

/// Recursive impl for [`inherited_layers`].
fn propagate_layers_impl<Layer: LayerIndex>(
    entity: Entity,
    propagated_layer: &Layer,
    query: &Query<(Option<&Children>, Option<&Layer>)>,
    layer_map: &mut EntityHashMap<Layer>,
) {
    let (children, layer) = query.get(entity).expect("query shouldn't ever fail");
    let layer = layer.unwrap_or(propagated_layer);
    layer_map.insert(entity, layer.clone());

    let Some(children) = children else {
        return;
    };

    for child in children {
        propagate_layers_impl(*child, layer, query, layer_map);
    }
}

/// Compute the z-coordinate that each entity should have. This is equal to its layer's equivalent
/// z-coordinate, plus an offset in the range [0, 1) corresponding to its y-sorted position
/// (if y-sorting is enabled).
pub fn set_z_coordinates<Layer: LayerIndex>(
    In(layers): In<EntityHashMap<Layer>>,
    mut transform_query: Query<&mut GlobalTransform>,
    options: Res<SpriteLayerOptions>,
) {
    if options.y_sort {
        // We y-sort everything because this avoids the overhead of grouping
        // entities by their layer.
        let key_fn = |entity: &Entity| {
            transform_query
                .get(*entity)
                .map(ZIndexSortKey::new)
                .unwrap_or_else(|_| ZIndexSortKey::new(&Default::default()))
        };
        // note: parallelizing with rayon is slower(!) here. I'm not sure why. maybe it has to do
        // with some kind of inter-thread overhead or L1/L2 cache not being shared?
        let y_sorted = layers
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .tap_mut(|v| v.sort_by_cached_key(key_fn));

        let scale_factor = 1.0 / y_sorted.len() as f32;
        for (i, entity) in y_sorted.into_iter().enumerate() {
            let z = layers[&entity].as_z_coordinate() + (i as f32) * scale_factor;
            set_transform_z(&mut transform_query, entity, z);
        }
    } else {
        for (entity, layer) in layers {
            set_transform_z(&mut transform_query, entity, layer.as_z_coordinate());
        }
    }
}

/// Sets the given entity's global transform z. Does nothing if it doesn't have one.
fn set_transform_z(query: &mut Query<&mut GlobalTransform>, entity: Entity, z: f32) {
    // hacky hacky; I can't find a way to directly mutate the GlobalTransform.
    let Some(mut transform) = query.get_mut(entity).ok() else {
        return;
    };
    let transform = transform.bypass_change_detection();
    let mut affine = transform.affine();
    affine.translation.z = z;
    *transform = GlobalTransform::from(affine);
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

/// Stores the z-coordinate that will be used at render time. Don't modify this yourself.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Component, Reflect)]
pub struct RenderZCoordinate(pub f32);

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Component)]
    enum Layer {
        Top,
        Middle,
        Bottom,
    }

    impl LayerIndex for Layer {
        fn as_z_coordinate(&self) -> f32 {
            use Layer::*;
            match self {
                Bottom => 0.0,
                Middle => 1.0,
                Top => 2.0,
            }
        }
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(TransformPlugin)
            .add_plugins(SpriteLayerPlugin::<Layer>::default());

        app
    }

    /// Just verify that adding the plugin doesn't somehow blow everything up.
    #[test]
    fn plugin_add_smoke_check() {
        let _ = test_app();
    }

    fn transform_at(x: f32, y: f32) -> TransformBundle {
        TransformBundle::from_transform(Transform::from_xyz(x, y, 0.0))
    }

    fn get_z(world: &World, entity: Entity) -> f32 {
        world
            .get::<GlobalTransform>(entity)
            .unwrap()
            .translation()
            .z
    }

    #[test]
    fn simple() {
        let mut app = test_app();
        let top = app
            .world_mut()
            .spawn((transform_at(1.0, 1.0), Layer::Top))
            .id();
        let middle = app
            .world_mut()
            .spawn((transform_at(1.0, 1.0), Layer::Middle))
            .id();
        let bottom = app
            .world_mut()
            .spawn((transform_at(1.0, 1.0), Layer::Bottom))
            .id();
        app.update();

        assert!(get_z(app.world(), bottom) < get_z(app.world(), middle));
        assert!(get_z(app.world(), middle) < get_z(app.world(), top));
    }

    fn layer_bundle(layer: Layer) -> impl Bundle {
        (transform_at(0.0, 0.0), layer)
    }

    #[test]
    fn inherited() {
        let mut app = test_app();
        let top = app.world_mut().spawn(layer_bundle(Layer::Top)).id();
        let child_with_layer = app
            .world_mut()
            .spawn(layer_bundle(Layer::Middle))
            .set_parent(top)
            .id();
        let child_without_layer = app
            .world_mut()
            .spawn(transform_at(0.0, 0.0))
            .set_parent(top)
            .id();
        app.update();

        // we use .floor() here since y-sorting can add a fractional amount to the coordinates
        assert_eq!(
            get_z(app.world(), child_with_layer).floor(),
            Layer::Middle.as_z_coordinate()
        );
        assert_eq!(
            get_z(app.world(), child_without_layer).floor(),
            get_z(app.world(), top).floor()
        );
    }

    #[test]
    fn y_sorting() {
        let mut app = test_app();
        for _ in 0..10 {
            app.world_mut()
                .spawn((transform_at(0.0, fastrand::f32()), Layer::Top));
        }
        app.update();
        let positions =
            app.world_mut()
                .run_system_once(|query: Query<&GlobalTransform>| -> Vec<Vec3> {
                    query
                        .into_iter()
                        .map(|transform| transform.translation())
                        .collect()
                });
        let sorted_by_z = positions
            .clone()
            .tap_mut(|positions| positions.sort_by_key(|vec| OrderedFloat(vec.z)));
        let sorted_by_y = positions
            .tap_mut(|positions| positions.sort_by_key(|vec| Reverse(OrderedFloat(vec.y))));
        assert_eq!(sorted_by_z, sorted_by_y);
    }

    #[test]
    fn child_with_no_transform() {
        let mut app = test_app();
        let entity = app.world_mut().spawn(layer_bundle(Layer::Top)).id();
        let child = app.world_mut().spawn_empty().set_parent(entity).id();
        let grandchild = app
            .world_mut()
            .spawn(transform_at(0.0, 0.0))
            .set_parent(child)
            .id();
        app.update();
        assert_eq!(
            get_z(app.world(), grandchild).floor(),
            Layer::Top.as_z_coordinate()
        );
    }
}
