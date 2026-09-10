import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-11",
    title: "Across the gap",
    purpose:
        "Make the journey and the change of course tangible without a fight.",
    panels: [
        panel("11a", {
            art: scene("across-the-gap"),
            action: "A broad exterior: Kaveri on the new course, its replacement assembly still secured. The ship is small against the distance. Do not draw looping jet-fighter trails or a continuous burn simply to fill the panel. No dialogue.",
            dialogue: [],
            lettering: {},
            labels: [],
        }),
        panel("11b", {
            art: scene("gantry-in-sight"),
            action: "Later, with Gantry now resolved ahead, Kaveri's crew compares the transmitted condition report with what they can see. The hull is stable, not tumbling. Damage interrupts the form we saw intact at Aquila.",
            dialogue: [
                say(
                    "tomas-1",
                    "Tomas",
                    "Gantry in sight. Matching their motion."
                ),
                say(
                    "nadia-2",
                    "Nadia / comms",
                    "We can hold attitude. Main drive is out."
                ),
                say("jonah-3", "Jonah", "Understood. We come to you."),
            ],
            lettering: {
                "tomas-1": balloon({
                    at: [18, 18],
                    width: 375,
                    tail: [161, 202],
                    side: "bottom",
                }),
                "nadia-2": balloon({
                    at: [415, 18],
                    width: 541,
                    tail: [391, 371],
                    side: "bottom",
                }),
                "jonah-3": balloon({
                    at: [990, 18],
                    width: 408,
                    tail: [1262, 213],
                    side: "bottom",
                }),
            },
            labels: [
                label("channel", "GANTRY / COMMS", {
                    slot: "channel",
                    at: [424, 383],
                    size: 17,
                    color: "work-card-text",
                }),
            ],
        }),
    ],
    layout: {
        "11a": { at: [42, 86], size: [1416, 440] },
        "11b": { at: [42, 546], size: [1416, 397] },
    },
});
