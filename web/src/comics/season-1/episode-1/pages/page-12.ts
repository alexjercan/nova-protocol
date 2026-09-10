import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-12",
    title: "One way in",
    purpose:
        "Leila checks Gantry's port with its crew while Kaveri prepares to receive them.",
    panels: [
        panel("12a", {
            art: scene("visible-port"),
            action: "Leila studies the port area on an external camera view as Kaveri closes. Her questions go to the people aboard.",
            dialogue: [
                say(
                    "leila-1",
                    "Leila",
                    "Gantry, did the electrical fault reach the docking equipment?"
                ),
                say(
                    "owen-2",
                    "Owen / comms",
                    "Not that branch. We isolated the main distribution. Leave it isolated."
                ),
            ],
            lettering: {
                "leila-1": balloon({
                    at: [18, 18],
                    width: 608,
                    tail: [183, 194],
                    side: "bottom",
                }),
                "owen-2": balloon({
                    at: [648, 18],
                    width: 750,
                    tail: [1315, 170],
                    side: "bottom",
                }),
            },
            labels: [
                label("camera", "GANTRY / EXTERIOR VIEW", {
                    slot: "camera",
                    at: [520, 178],
                    size: 19,
                    color: "mint",
                }),
            ],
        }),
        panel("12b", {
            art: scene("receiving-space"),
            action: "Rina prepares Kaveri's transfer side. Samir clears a receiving space and brings the first-aid case.",
            dialogue: [
                say(
                    "leila-1",
                    "Leila / comms",
                    "Agreed. We'll check the connection before anyone crosses."
                ),
                say(
                    "ivo-2",
                    "Ivo / comms",
                    "The access is clear. Owen needs help moving."
                ),
                say(
                    "rina-3",
                    "Rina",
                    "Stay with him. We'll meet you at the opening."
                ),
            ],
            lettering: {
                "leila-1": balloon({
                    at: [18, 18],
                    width: 405,
                    tail: [58, 199],
                    side: "bottom",
                }),
                "ivo-2": balloon({
                    at: [441, 18],
                    width: 387,
                    tail: [483, 201],
                    side: "bottom",
                }),
                "rina-3": balloon({
                    at: [439, 366],
                    width: 389,
                    tail: [634, 321],
                    side: "top",
                }),
            },
            labels: [
                label("first-aid", "FIRST AID", {
                    slot: "first-aid",
                    at: [259, 403],
                    size: 18,
                    color: "ink",
                }),
            ],
        }),
        panel("12c", {
            art: scene("ready-to-help"),
            action: "Jonah and Samir hold their handholds in the cabin, with the first-aid case stowed beside them. Jonah glances across at Samir.",
            dialogue: [
                say("jonah-1", "Jonah", "Ready for him?"),
                say(
                    "samir-2",
                    "Samir",
                    "Ready to help him aboard. I'll need their account of the injury."
                ),
            ],
            lettering: {
                "jonah-1": balloon({
                    at: [18, 18],
                    width: 230,
                    tail: [146, 151],
                    side: "bottom",
                }),
                "samir-2": balloon({
                    at: [275, 18],
                    width: 257,
                    tail: [428, 244],
                    side: "bottom",
                }),
            },
            labels: [
                label("first-aid", "FIRST AID", {
                    slot: "first-aid",
                    at: [188, 417],
                    size: 18,
                    color: "ink",
                }),
            ],
        }),
    ],
    layout: {
        "12a": { at: [42, 86], size: [1416, 348] },
        "12b": { at: [42, 454], size: [846, 489] },
        "12c": { at: [908, 454], size: [550, 489] },
    },
});
