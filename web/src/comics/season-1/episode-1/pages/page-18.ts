import { page, panel, scene, say, balloon } from "../../../comic-script";

export default page({
    id: "page-18",
    title: "The bill",
    purpose:
        "Elena and Jonah count the cost of the diversion at the office window, and the mug comes home.",
    panels: [
        panel("18a", {
            art: scene("penalty-at-window"),
            action: "Jonah and Elena at the same office window, with Ebro still waiting outside.",
            dialogue: [
                say(
                    "elena-1",
                    "Elena",
                    "Mara called from Foundation. She's glad Gantry's people survived."
                ),
                say("jonah-2", "Jonah", "And the delivery?"),
                say("elena-3", "Elena", "Still late. The penalty stands."),
            ],
            lettering: {
                "elena-1": balloon({
                    at: [18, 18],
                    width: 570,
                    tail: [270, 245],
                    side: "bottom",
                }),
                "jonah-2": balloon({
                    at: [1014, 18],
                    width: 382,
                    tail: [1237, 210],
                    side: "bottom",
                }),
                "elena-3": balloon({
                    at: [455, 329],
                    width: 470,
                    tail: [330, 270],
                    side: "top",
                }),
            },
        }),
        panel("18b", {
            art: scene("carrying-the-cost"),
            action: "Jonah listens while Elena lays out what the delay costs the company.",
            dialogue: [
                say("jonah-1", "Jonah", "Can we carry it?"),
                say(
                    "elena-2",
                    "Elena",
                    "It'll hurt. We can carry it. I told you to bring them home."
                ),
            ],
            lettering: {
                "jonah-1": balloon({
                    at: [18, 18],
                    width: 236,
                    tail: [100, 244],
                    side: "bottom",
                }),
                "elena-2": balloon({
                    at: [274, 18],
                    width: 388,
                    tail: [520, 290],
                    side: "bottom",
                }),
            },
        }),
        panel("18c", {
            art: scene("mug-back-home"),
            action: "Jonah puts the borrowed mug back on the worktop within Elena's reach. Through the window, Ebro waits and Baikal's ordinary work continues.",
            dialogue: [
                say("jonah-1", "Jonah", "At least this one's back on budget."),
                say("elena-2", "Elena", "One problem solved."),
            ],
            lettering: {
                "jonah-1": balloon({
                    at: [18, 18],
                    width: 382,
                    tail: [145, 282],
                    side: "bottom",
                }),
                "elena-2": balloon({
                    at: [424, 18],
                    width: 274,
                    tail: [551, 230],
                    side: "bottom",
                }),
            },
        }),
    ],
    layout: {
        "18a": { at: [42, 86], size: [1416, 415] },
        "18b": { at: [42, 521], size: [680, 422] },
        "18c": { at: [742, 521], size: [716, 422] },
    },
});
