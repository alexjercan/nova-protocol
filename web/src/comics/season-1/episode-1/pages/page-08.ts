import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-8",
    title: "A familiar voice",
    purpose:
        "A distress call from Gantry reaches Kaveri, and Nadia reports what is left aboard.",
    panels: [
        panel("8a", {
            art: scene("familiar-signal"),
            action: "Samir at communications. A distress call interrupts the quiet. Jonah turns before Samir has finished identifying the sender.",
            dialogue: [
                say(
                    "nadia-1",
                    "Nadia / comms",
                    "Gantry, requesting assistance. Main propulsion disabled."
                ),
                say("samir-2", "Samir", "That's the captain from Aquila."),
            ],
            lettering: {
                "nadia-1": balloon({
                    at: [18, 18],
                    width: 374,
                    tail: [160, 319],
                    side: "bottom",
                }),
                "samir-2": balloon({
                    at: [411, 18],
                    width: 269,
                    tail: [558, 192],
                    side: "bottom",
                }),
            },
            labels: [
                label("channel", "GANTRY / INCOMING", {
                    slot: "channel",
                    at: [43, 306],
                    size: 19,
                    color: "mint",
                }),
                label("request", "REQUESTING ASSISTANCE", {
                    slot: "request",
                    at: [43, 342],
                    size: 14,
                    color: "work-card-text",
                }),
            ],
        }),
        panel("8b", {
            art: scene("answering-gantry"),
            action: "Jonah answers from the cabin. The call stays on the console speaker; Nadia is heard, not seen.",
            dialogue: [
                say(
                    "jonah-1",
                    "Jonah",
                    "Gantry, Kaveri. We hear you. How many aboard?"
                ),
                say(
                    "nadia-2",
                    "Nadia / comms",
                    "Three. All alive. Owen's hurt. We've isolated the damaged spaces."
                ),
            ],
            lettering: {
                "jonah-1": balloon({
                    at: [18, 18],
                    width: 330,
                    tail: [170, 200],
                    side: "bottom",
                }),
                "nadia-2": balloon({
                    at: [366, 18],
                    width: 314,
                    tail: [609, 331],
                    side: "bottom",
                }),
            },
            labels: [
                label("channel", "GANTRY", {
                    slot: "channel",
                    at: [423, 289],
                    size: 21,
                    color: "mint",
                }),
                label("audio", "OPEN CHANNEL", {
                    slot: "audio",
                    at: [423, 322],
                    size: 15,
                    color: "work-card-text",
                }),
            ],
        }),
        panel("8c", {
            art: scene("recording-report"),
            action: "Samir records the report while Tomas begins checking the relative tracks. Everything they know arrives over the channel from Nadia.",
            dialogue: [
                say("samir-1", "Samir", "What's still working?"),
                say(
                    "nadia-2",
                    "Nadia / comms",
                    "Emergency power. Comms, attitude control, limited life support. They took the cargo and left us here."
                ),
                say(
                    "jonah-3",
                    "Jonah",
                    "Send your reserve estimate and the port status. We're checking an intercept."
                ),
            ],
            lettering: {
                "samir-1": balloon({
                    at: [18, 18],
                    width: 300,
                    tail: [197, 202],
                    side: "bottom",
                }),
                "nadia-2": balloon({
                    at: [340, 18],
                    width: 550,
                    tail: [518, 287],
                    side: "bottom",
                }),
                "jonah-3": balloon({
                    at: [910, 18],
                    width: 488,
                    tail: [1220, 212],
                    side: "bottom",
                }),
            },
            labels: [
                label("report", "GANTRY / CREW REPORT", {
                    slot: "report",
                    at: [414, 291],
                    size: 18,
                    color: "mint",
                }),
                label("reserves", "RESERVES / REQUESTED", {
                    slot: "reserves",
                    at: [414, 337],
                    size: 17,
                    color: "work-card-text",
                }),
                label("port", "PORT STATUS / REQUESTED", {
                    slot: "port",
                    at: [414, 378],
                    size: 17,
                    color: "work-card-text",
                }),
            ],
        }),
    ],
    layout: {
        "8a": { at: [42, 86], size: [698, 390] },
        "8b": { at: [760, 86], size: [698, 390] },
        "8c": { at: [42, 496], size: [1416, 447] },
    },
});
