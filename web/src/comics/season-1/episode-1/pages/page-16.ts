import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-16",
    title: "The return",
    purpose:
        "During the return, Ivo learns of the refusal and Nadia asks for the record.",
    panels: [
        panel("16a", {
            art: scene("asking-for-record"),
            action: "Later aboard Kaveri. Owen rests on his secured support with the first-aid case nearby. Ivo has learned of the funding refusal, and Nadia asks Samir for the record itself.",
            dialogue: [
                say("ivo-1", "Ivo", "They knew we were alive?"),
                say(
                    "samir-2",
                    "Samir",
                    "Elena sent them the estimate before the refusal."
                ),
                say(
                    "nadia-3",
                    "Nadia",
                    "Send me that exchange. I want an answer from Operations."
                ),
            ],
            lettering: {
                "ivo-1": balloon({
                    at: [18, 18],
                    width: 430,
                    tail: [140, 258],
                    side: "bottom",
                }),
                "samir-2": balloon({
                    at: [466, 18],
                    width: 466,
                    tail: [585, 252],
                    side: "bottom",
                }),
                "nadia-3": balloon({
                    at: [950, 18],
                    width: 448,
                    tail: [1107, 251],
                    side: "bottom",
                }),
            },
            labels: [
                label("first-aid", "FIRST AID", {
                    slot: "first-aid",
                    at: [483, 444],
                    size: 18,
                    color: "ink",
                }),
                label("record", "RECEIVED LOG", {
                    slot: "record",
                    at: [297, 326],
                    size: 17,
                    color: "work-card-text",
                }),
            ],
        }),
        panel("16b", {
            art: scene("quiet-thanks"),
            action: "Jonah pauses beside Nadia during the return, with the stars of the coast in the window behind them.",
            dialogue: [
                say("nadia-1", "Nadia", "Thank you for coming."),
                say("jonah-2", "Jonah", "I'm glad we reached you."),
            ],
            lettering: {
                "nadia-1": balloon({
                    at: [195, 18],
                    width: 440,
                    tail: [340, 226],
                    side: "bottom",
                }),
                "jonah-2": balloon({
                    at: [782, 18],
                    width: 520,
                    tail: [1025, 224],
                    side: "bottom",
                }),
            },
        }),
    ],
    layout: {
        "16a": { at: [42, 86], size: [1416, 510] },
        "16b": { at: [42, 616], size: [1416, 327] },
    },
});
