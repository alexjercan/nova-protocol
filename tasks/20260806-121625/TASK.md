# Refactor nova_* crate for better structure and clarity

- PRIORITY: 40
- TAGS: v0.10.0, refactoring, project
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -

PROBLEM: The `nova_probe` crate feels messy and hacked together; I have the
same feeling of other crates, some feel a bit too coupled; There are useless
comment all over the code;

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

This is an example of my review of `nova_probe` crate, now it's your job to do
a full multi agent deep dive review of the other crates and the structure of
the project to build a better understanding. The goal of this task is "cleanup"
and "improvement" of the code, which is a hard problem in my opinion because we
need to define what that means; try to use my review of what improving means;
during the understanding phase ASK ME A LOT OF QUESTIONS about any decision you
might consider. Let's try to identify what `improving` the code really means,
because I don't want this to be just shuffling code around but still getting to
a actually good result -> better performance, easier to test, less code,
simpler code, - honestly I wouldn't be able to say that these represent
improvements. It's more about readability and being able to go through the code
structure fast and being able to tell what each module/system does from the
folder structure. Something else is code should be self documenting, keep docs
only for public APIs (make clippy happy). But in code comments should be kept
minimal and only for actually important things "comment why not what". First
step of understanding should be collecting all the context then figuring out
what to do with it.
