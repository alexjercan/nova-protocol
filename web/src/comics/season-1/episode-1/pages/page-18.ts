import { scriptPage, scriptPanel, say } from "../../../comic-script";

export default scriptPage({
    id: "page-18",
    title: "The bill",
    purpose:
        "Put institutional unfairness beside a successful rescue, then end at home.",
    panels: [
        scriptPanel(
            "18a",
            "Jonah and Elena at the familiar window. Ebro remains part of their working view. Elena has already spoken with Mara; there is no cutaway aboard Foundation.",
            [
                say(
                    "elena-1",
                    "Elena",
                    "Mara called from Foundation. She's glad Gantry's people survived."
                ),
                say("jonah-2", "Jonah", "And the delivery?"),
                say("elena-3", "Elena", "Still late. The penalty stands."),
            ]
        ),
        scriptPanel(
            "18b",
            "Jonah listens while Elena lays out the consequence without passing blame down to the crew. This is lost margin and rescheduling, not Clearwell's destruction.",
            [
                say("jonah-1", "Jonah", "Can we carry it?"),
                say(
                    "elena-2",
                    "Elena",
                    "It'll hurt. We can carry it. I told you to bring them home."
                ),
            ]
        ),
        scriptPanel(
            "18c",
            "Jonah puts the borrowed mug back within Elena's reach. Through the window: Baikal's ordinary work continuing. End on the object and the people, not an ominous pirate silhouette or an invitation to cheer an unpaid bill.",
            [
                say("jonah-1", "Jonah", "At least this one's back on budget."),
                say("elena-2", "Elena", "One problem solved."),
            ]
        ),
    ],
});
