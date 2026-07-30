# Spike: how to improve the NOVA OS CRT monitor look and feel

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog,spike,ui,hud
- KIND: SPIKE
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

Design review of the NOVA OS CRT ship-computer: what reads well, what is meh,
and what would make it feel like a real phosphor CRT rather than a green
rectangle with effects painted on. Grounded in the current shader
(`assets/shaders/nova_os_crt.wgsl`), the drawer
(`crates/nova_gameplay/src/hud/drawer.rs`) and the captures in
`tasks/20260726-180807/shots/`.

The research and recommendation live in `tasks/20260726-193040/SPIKE.md`. This
task is the spike record; the worthwhile improvements are seeded as separate
tasks (see the SPIKE.md "Next steps").
