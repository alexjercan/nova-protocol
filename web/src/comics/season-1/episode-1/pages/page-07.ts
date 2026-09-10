import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-7",
    title: "On the way home",
    purpose:
        "The load is secured, Gantry leaves first, and Kaveri turns for home.",
    panels: [
        panel("7a", {
            art: scene("gantry-departure"),
            action: "Back inside Kaveri, Rina and Samir finish the restraint and access checks after the assembly has been clamped into the external cradle. The display confirms the load. Through the window, Gantry leaves first, intact.",
            dialogue: [
                say("rina-1", "Rina", "Load's secured. Passage is clear."),
                say("samir-2", "Samir", "And every cover is still on it."),
            ],
            lettering: {
                "rina-1": balloon({
                    at: [852, 18],
                    width: 281,
                    tail: [977, 173],
                    side: "bottom",
                    breakAfter: [2],
                }),
                "samir-2": balloon({
                    at: [1141, 18],
                    width: 257,
                    tail: [1254, 181],
                    side: "bottom",
                    breakAfter: [4],
                }),
            },
            labels: [
                label("label-1", "LOAD SECURED / COVERS ON", {
                    slot: "label-1",
                    at: [1107, 307],
                    size: 14,
                    color: "mint",
                    weight: "normal",
                    spacing: 0.3,
                    anchor: "start",
                }),
                label("label-2", "PASSAGE CLEAR", {
                    slot: "label-2",
                    at: [1107, 332],
                    size: 17,
                    color: "work-card-text",
                    weight: "normal",
                    spacing: 0,
                    anchor: "start",
                }),
            ],
        }),
        panel("7b", {
            art: scene("outbound-aquila"),
            action: "Kaveri outbound from Aquila, with the same covered assembly visible in its external cradle. The town falls away behind it.",
            dialogue: [],
            lettering: {},
        }),
        panel("7c", {
            art: scene("thoughts-of-home"),
            action: "Later in the return leg. Leila checks the load record near Jonah. The work is under control, and her attention can briefly return to home.",
            dialogue: [
                say("leila-1", "Leila", "I might make game night after all."),
                say("jonah-2", "Jonah", "Is that good news for everyone else?"),
                say("leila-3", "Leila", "They can practice while I'm away."),
            ],
            lettering: {
                "leila-1": balloon({
                    at: [18, 18],
                    width: 333,
                    tail: [144, 211],
                    side: "bottom",
                    breakAfter: [5],
                }),
                "jonah-2": balloon({
                    at: [382, 18],
                    width: 314,
                    tail: [548, 219],
                    side: "bottom",
                    breakAfter: [5],
                }),
                "leila-3": balloon({
                    at: [18, 356],
                    width: 378,
                    tail: [155, 301],
                    side: "top",
                    breakAfter: [5],
                }),
            },
            labels: [
                label("label-1", "COLLECTION COMPLETE", {
                    slot: "label-1",
                    at: [448, 397],
                    size: 14,
                    color: "mint",
                    weight: "normal",
                    spacing: 0,
                    anchor: "start",
                }),
                label("label-2", "BAIKAL", {
                    slot: "label-2",
                    at: [448, 428],
                    size: 22,
                    color: "work-card-text",
                    weight: "normal",
                    spacing: 2,
                    anchor: "start",
                }),
            ],
        }),
    ],
    layout: {
        "7a": { at: [42, 86], size: [1416, 350] },
        "7b": { at: [42, 456], size: [680, 487] },
        "7c": { at: [742, 456], size: [716, 487] },
    },
});
