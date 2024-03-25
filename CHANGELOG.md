# Release notes

## Version 0.5.0

**Major functionality changes**:

- The plugin now modifies entities' transforms instead of intervening in the render step. This means
  that this should now work on any entity, not just sprites.
- In order to preserve the useful invariant that transforms are not affected, this plugin now runs
  in the `Last` schedule to directly set the `GlobalTransform` and in the `First` schedule to zero
  it out. If you have code that runs in either of these schedules, you can explicitly sequence
  with respect to the `SpriteLayerSet` system set.
- Layers now propagate to the children of each entity; if a child has an explicitly-set layer, that
  overrides the inherited layer.

**Minor changes**:

- We now always use single-threaded sort, as multi-threaded sort is a pessimization(!) for all
  the reasonable entity counts I tested. The `parallel_y_sort` feature has been removed.
- Benchmarks are now finer-grained and have less overhead.

## ~~Version 0.4.0~~

This version has been yanked, since it doesn't actually work due to changes in bevy around how
entities in the render world are created. (Specifically, they have no association with their
original entities).

- ~~Compatibility with bevy 0.13. No functional changes.~~

## Version 0.3.0

- Compatibility with bevy 0.12. No functional changes.

## Version 0.2.0

- Compatibility with bevy 0.11. No functional changes.

## Version 0.1.1

- Actually put the system in a `pub` set so you can order around it.

## Version 0.1

- Initial release!
