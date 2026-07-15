# Modding

> Heads up: a data-driven modding format is **planned, not shipped yet**. This page will fill in once it lands. Here is where things actually stand today.

## Where authoring stands today

The [scenario](../scenarios/) engine is already fully expressive - objects, the events / filters / actions vocabulary, and typed variables - but it is currently driven from **code**. Every shipped scenario is a small function that builds its configuration in Rust; there is no external scenario file format (no RON, YAML or JSON) to load a mission from disk yet.

In other words, the vocabulary a mod would use exists and is stable; what is missing is the data layer that lets you write a mission _without_ touching the codebase.

## What is planned

The intended next step is a **data-driven scenario format** (most likely RON asset files) that wraps the same event / filter / action / variable vocabulary, so a scenario becomes a file you can author and share rather than compiled code. Once that format is stable, this page will document its schema, the object and event reference, and how to load your own scenarios.

Until then, the [Scenarios](../scenarios/) page is the best map of what the engine can already express.
