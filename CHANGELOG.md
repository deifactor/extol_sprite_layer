# Release notes

## Version 0.5.0

- **Major functionality change**: the plugin now modifies entities' transforms instead of
  intervening in the render step. This means that this should now work on any entity, not just
  sprites. This in particular means that version 0.4.0 is broken and should not be used.

## ~~Version 0.4.0~~

- ~~Compatibility with bevy 0.13. No functional changes.~~

## Version 0.3.0

- Compatibility with bevy 0.12. No functional changes.

## Version 0.2.0

- Compatibility with bevy 0.11. No functional changes.

## Version 0.1.1

- Actually put the system in a `pub` set so you can order around it.

## Version 0.1

- Initial release!
