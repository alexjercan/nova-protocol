# Refactor nova_probe crate for better structure and clarity

- PRIORITY: 40
- TAGS: v0.10.0,refactoring,probe
- ACTIVITY: -
- GATES: -
- RESOLUTION: -

PROBLEM: The `nova_probe` crate feels messy and hacked together

I personally see `nova_timeline`, `nova_invariants` and `nova_frametime` in a
`capabilities` module inside the crate. Then we would have a `trait Capability`
which would define the interface of a capability (mainly collecting evidence ->
TDB exact shape in a prototype during understanding phase). I also noticed
excesive use of `wasm/no-wasm` compiler gates. That makes me think that we can
extract the read/write into a module and abstract it away behind a `Plugin` or
a plain `struct` resource or something (TBD via a prototype). I would also
create an `evaluation` module that does the check runs and verifies the
evidence collected by the capabilities. This would produce a report. The final
step should be converting the report to html. We can probably refactor that
part of the code into a `report` module. Something that use read/write to write
the HTML or whatever we use.

In my mind I have these steps `collect evidence` -> `run evaluation` ->
`generate report`. Each example/bin that includes a Capability collects
evidence. But we also need to add the run evaluation plugin and generate
report one. These are obviously added via NovaProbe Plugin or something like
that.

I think we do not have a "nova probe" plugin so we should add one that manages
all these steps such that it is clear that this binary is being probed.
