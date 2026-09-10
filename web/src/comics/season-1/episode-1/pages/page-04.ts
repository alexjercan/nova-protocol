import {
    page,
    panel,
    scene,
    say,
    balloon,
    location,
} from "../../../comic-script";

export default page({
    id: "page-4",
    title: "Outbound",
    purpose:
        "The crew take their stations and Kaveri leaves Baikal for Aquila.",
    panels: [
        panel("4a", {
            art: scene("departure-stations"),
            action: "A Kaveri location card introduces the move aboard. Tomas reports readiness and Jonah gives the departure order.",
            dialogue: [
                say(
                    "tomas-1",
                    "Tomas",
                    "All aboard. Cargo space is clear, and the Aquila course is ready."
                ),
                say("jonah-2", "Jonah", "All right. Take us out."),
            ],
            lettering: {
                "tomas-1": balloon({
                    at: [455, 20],
                    width: 472,
                    tail: [636, 215],
                    side: "bottom",
                    breakAfter: [5, 10],
                }),
                "jonah-2": balloon({
                    at: [999, 20],
                    width: 389,
                    tail: [1197, 210],
                    side: "bottom",
                    breakAfter: [],
                }),
            },
            cards: [
                location("location", {
                    name: "KAVERI",
                    place: "Departing Baikal",
                    date: "Later that day",
                    note: "Clearwell Waterworks' workship",
                    at: [25, 20],
                    width: 390,
                }),
            ],
        }),
        panel("4b", {
            art: scene("outbound-baikal"),
            action: "Kaveri departs with its cradle empty and handling arm stowed. Baikal and Ebro recede.",
            dialogue: [],
            lettering: {},
        }),
    ],
    layout: {
        "4a": { at: [42, 86], size: [1416, 407] },
        "4b": { at: [42, 513], size: [1416, 430] },
    },
});
