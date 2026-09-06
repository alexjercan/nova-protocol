# Narrative channels for mods

A `Channel` is WHERE a story line was heard, and how the comms panel draws one
that was. It is not a faction and not a speaker: the same person reaches the
cockpit down two different channels and the difference is the point. Meridian
Control on the work channel is talking to you; the same desk read over the guard
channel is something you are overhearing.

Every [`NarrativeCue`](../actions/#narrativecue) names a channel by id. An id no
bundle authors is an error at lint and a refusal at load - the game will not
start a scenario whose lines have nowhere to be heard, because a line in a
guessed voice is a beat that does not land.

This page is the field-by-field channel reference. For general RON spelling rules
such as double parentheses, `Some(...)`, and asset schemes, see
[RON spelling rules](../reference/#how-ron-content-is-written).

## The Channel item

```ron
[
    Channel((
        id: "my_mod_distress",
        tone: Threat,
        tag: Some("DISTRESS"),
        signal_strength: 0.8,
    )),
]
```

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | Stable key, the overlay key, and what a cue names. Re-declare a base id to restyle that channel in place; a new id is a new channel. Prefix new ids with your mod id. |
| `tone` | tone | required | The chip family the card is drawn in - see [below](#tones). |
| `tag` | `Option` string | `None` | The channel name shown beside the speaker. `None` = the channel needs no saying. |
| `signal_strength` | number | required | How strongly the card is drawn, `0` exclusive to `1`. |

A channel carries no behaviour. Every field is a presentation fact the panel
reads, which is why a channel is content rather than code: nothing in the game
has to change for a campaign to add a distress band, a corporate net or a
smuggler's private channel.

## Tones

`tone` picks the channel's whole colour family at once - the card's frame, its
header, and (lifted toward white) the line the player reads. One field rather
than four colours, so a mod's channel lands inside the CRT palette instead of
beside it.

| tone | the HUD reads it as |
|---|---|
| `Phosphor` | The HUD's own default voice - green. A line in the room. |
| `Amber` | The "attend to this" amber the objective chips use. |
| `Threat` | Combat red, the colour of locks and hostile targets. |
| `Comms` | The incoming-transmission blue. |

A tone is not exclusive: two channels may share one. They will be hard to tell
apart, which is a choice you can make deliberately - a pirate band imitating
fleet traffic - and a mistake the rest of the time.

## Tag and signal strength

These are the two dials that say how much of the line is ADDRESSED to the player.

A `tag` labels the channel beside the speaker. Tag the channels a player has to
tell apart from the panel's own default voice; a tag on every line distinguishes
it from nothing, so most channels want `None`.

`signal_strength` fades the frame, header and body together, so a weak channel
stays recognisably itself instead of turning into a second set of colours. A line
the ship was sent is at full strength. A fragment the cockpit merely caught is
not.

## Base channels

Three ship, and any mod may name them:

| id | tone | tag | strength | what it is |
|---|---|---|---|---|
| `comms` | `Comms` | none | 1.0 | The work channel: traffic addressed to this ship. What the comms panel IS, so it needs no tag. |
| `crew` | `Phosphor` | none | 1.0 | Inside the hull, off the radio entirely - the crew talking to each other. |
| `guard` | `Amber` | `GUARD` | 0.7 | Everybody's channel, nobody's conversation. Tagged and weak, so a fragment reads as something the cockpit caught. |

Read `assets/base/channels/base.content.ron` for the exact rows.

## Using a channel

Name the id on the cue:

```ron
NarrativeCue((
    channel: "my_mod_distress",
    speaker: "Unknown",
    text: "...anyone on this band, we are venting...",
))
```

Your mod may name a channel it authors, one the base game ships, or one from a
mod it [depends on](../mod-files/#paths-and-dependencies). It may not name a
channel that merely happens to be installed - the lint refuses it, so a bundle
that works on your machine works on everyone's.
