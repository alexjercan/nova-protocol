import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-13",
    title: "Hold there",
    purpose:
        "Kaveri holds off, checks the alignment, and joins to Gantry's port.",
    panels: [
        panel("13a", {
            art: scene("holding-at-port"),
            action: "An exterior view close enough to show the remaining gap. Gantry's damaged systems stay dark while Kaveri approaches the checked port.",
            dialogue: [
                say(
                    "tomas-1",
                    "Tomas / comms",
                    "Holding here for the port check."
                ),
                say("leila-2", "Leila / comms", "Give me the alignment view."),
            ],
            lettering: {
                "tomas-1": balloon({
                    at: [18, 18],
                    width: 390,
                    tail: [518, 240],
                    side: "bottom",
                }),
                "leila-2": balloon({
                    at: [430, 18],
                    width: 452,
                    tail: [559, 255],
                    side: "bottom",
                }),
            },
            labels: [
                label("kaveri", "KAVERI", {
                    slot: "kaveri",
                    at: [86, 435],
                    size: 20,
                    color: "mint",
                }),
                label("gantry", "GANTRY", {
                    slot: "gantry",
                    at: [691, 705],
                    size: 20,
                    color: "mint",
                }),
            ],
        }),
        panel("13b", {
            art: scene("checked-alignment"),
            action: "Leila reads the alignment view at her console while Tomas works the approach in the same cabin. Each concentrates on a different part of the one operation.",
            dialogue: [
                say(
                    "leila-1",
                    "Leila",
                    "That's the port we checked. Approach is clear."
                ),
                say("tomas-2", "Tomas", "Coming in."),
            ],
            lettering: {
                "leila-1": balloon({
                    at: [18, 18],
                    width: 460,
                    tail: [146, 168],
                    side: "bottom",
                }),
                "tomas-2": balloon({
                    at: [274, 126],
                    width: 204,
                    tail: [397, 236],
                    side: "bottom",
                }),
            },
        }),
        panel("13c", {
            art: scene("checking-seal"),
            action: "The ships joined, their bulk suddenly close. A short rigid sealed connector links the existing side collars, with the ships facing opposite directions. Both coast without thrust, and the transfer spaces are in freefall.",
            dialogue: [
                say(
                    "leila-1",
                    "Leila / comms",
                    "Connection secure. Checking the seal."
                ),
            ],
            lettering: {
                "leila-1": balloon({
                    at: [18, 18],
                    width: 460,
                    tail: [180, 140],
                    side: "bottom",
                }),
            },
        }),
    ],
    layout: {
        "13a": { at: [42, 86], size: [900, 857] },
        "13b": { at: [962, 86], size: [496, 350] },
        "13c": { at: [962, 456], size: [496, 487] },
    },
});
