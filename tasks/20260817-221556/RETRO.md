# Retro

## What worked

- Treating cursor ownership as a continuous screen invariant fixed ordering
  against any one-time or per-frame flight reconciler.

## Bug and fix

- State-entry hooks freed the pointer only once. Move the final ownership check
  to `PostUpdate` while the interactive screen remains active.

## Next time

- Interactive modal screens must own pointer visibility and grab mode for their
  full lifetime, not only during their opening transition.
