# Modal input: one owner for the keyboard, not a guard list per system

- STATUS: OPEN
- PRIORITY: 74
- TAGS: v0.12.0,input,editor,ui

Follows the review-fix task. Raised by round 1 of `/nova-review`, where a
one-line guard fix was judged the wrong shape for the problem.

## The problem

No arbiter decides which consumer owns a keystroke. Each keyboard system carries
its own hand-written guard list, so anything new suppresses nothing by default.
`typing_into_a_field` guards one specific modal consumer; the keybind rebind is
a second one and was added without a predicate, so pressing Delete to bind
Delete also runs `delete_key` and destroys the marked section
(`keybind.rs:381`, `placement.rs:276`, `lib.rs:175`). The review-fix task adds a
stop-gap predicate for exactly that pair. This task removes the need for it.

## The shape

Modal input, along the lines of vim's normal / insert / visual. A mode owns the
keyboard while it is active and every consumer asks the mode rather than
carrying a denial list. Bind mode swallows everything but its own exit, so the
camera stops taking WASD without knowing bind mode exists; placement and
shortcuts go quiet the same way.

Consumers that already are modes in all but name: text entry
(`typing_into_a_field`), the gallery (`gallery_open`), the keybind rebind, and
plausibly Nova OS.

## Done when

- The stop-gap guard from the review-fix task is deleted, and the collision it
  covered is unreachable by construction rather than by predicate.
- A new `examples/systems` range with its `catalog_drift` roster.
- Live run: `cargo check` cannot see an input mode fail.
