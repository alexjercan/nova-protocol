# Devlog #1: modular ships and first combat

This is where Nova Protocol started. The pitch was simple (and slightly delusional): watch a bit of _The Expanse_, decide "I can make a game like this", and then spend the next few weeks discovering exactly how much space sim hides behind that sentence. v0.1.0 is the first playable answer - modular ships you can build, fly on real thrusters, and pull apart with turret fire.

<div class="video-embed">
    <iframe
        src="https://www.youtube-nocookie.com/embed/AJcAMyJ0S3Y"
        title="Nova Protocol Devlog #1"
        loading="lazy"
        allow="
            accelerometer;
            autoplay;
            clipboard-write;
            encrypted-media;
            gyroscope;
            picture-in-picture;
        "
        allowfullscreen
    ></iframe>
</div>
<span class="video-embed__caption"
    >Devlog #1 - the full v0.1.0 walkthrough on YouTube.</span
>

## Space, then a ship that moves

To make a space sim you first need some space. That meant a test scene with a [generated skybox](https://tools.wwwtyro.net/space-3d/index.html) and a scatter of objects standing in for asteroids and debris. The player ship started life as the least glamorous thing possible: a default cube.

A cube that cannot move is not much of a ship, so the first real mechanic was the **thruster section** - a cylinder that applies an impulse in the direction it faces. Hook it to the <kbd>W</kbd> key and suddenly the ship drifts forward (and, on the first try, straight into a rock - _boom_). But forward-only is not flight. Because the physics engine is already doing the work, rotation is just more thrusters bound to more keys, placed to generate torque. Bolt a few on and you get a slightly cursed ship that can actually turn.

## Steering that does not fight you

Flying purely on hand-placed thrusters is fun for about a minute and then it is just hard. So v0.1.0 introduced a dedicated steering section that reads the mouse delta and turns it into a target rotation, driven by a **proportional-derivative controller** that applies torque to slew the hull smoothly instead of snapping. The first pass flew like a shopping trolley; after tuning the constants it started to feel like a spaceship. This is the seed of what later became the flight computer.

## Making it look less like cubes

With movement solid I let myself do some visuals: a proper hull model out of Blender (still fundamentally a cube, now with ridges) mostly to prove the engine could import meshes at all, and a shader for the thruster exhaust so you can actually see when a drive is firing. Small things, but the exhaust plume is the first moment the game reads as a ship rather than a physics demo.

## Turrets, and the math tax

Then: combat. The **turret section** is a model that sits on the hull with a yaw base and a tilt head so it can track in two axes. It aims at whatever the ship's "brain" designates - which in v0.1.0 is barely a brain at all, but it is enough to point the guns.

Getting the angles right was the hard part. Anyone who says programming does not need math has not tried to aim a turret. After a healthy amount of `atan2` and vector algebra it finally pointed where it should - and I learned to clamp its slew rate, because an unclamped turret snaps around like it is possessed. Firing spawns a muzzle flash and a projectile that flies forward and raycasts each tick to see what it hit.

## Health, damage, and satisfying destruction

Projectiles are only fun if hitting things means something, so every ship section carries its own **health component**. Damage comes from turret rounds and from ramming - the impact scales with collision velocity - and when a section's health hits zero, it does not just vanish. A **mesh slicer** breaks it into chunks that physics flings apart, so a kill looks like a real explosion instead of a disappearing box. Because health lives per section, damage is already local: shoot the turret off and the ship keeps flying but stops shooting.

## A HUD you can orient by

UI is my least favourite part of game dev, so the health readout got an honest ugly text box and I moved on. The orientation HUD got more care. Flat 2D arrows never conveyed depth or a real sense of heading, so I landed on an idea that stuck: a sphere around the ship with HUD elements living _on_ it in 3D. The first inhabitant is a velocity cone (same shader as the exhaust) that rides the sphere and points where you are actually moving. It is a work in progress here, but it is the direction the instrument cluster grew from.

## Shipping v0.1.0

Add collision damage, wire a simple ship editor onto the front, drop it all into a demo scene, and that is version 0.1.0: build a modular ship, fly it on real thrusters, and blow things (including yourself) up. It is rough, but every later system - the diegetic autopilot, the section-based combat model, the spherical HUD - is already visible in outline here.

Next devlog: objectives, the first enemy AI, and asteroids that stop being spheres. In the meantime you can [go fly something](../../play/).
