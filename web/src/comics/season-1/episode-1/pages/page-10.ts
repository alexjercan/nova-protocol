import { scriptPage, scriptPanel, say } from "../../../comic-script";

export default scriptPage({
    id: "page-10",
    title: "The decision",
    purpose: "Refusal changes who pays, not whether preparation proceeds.",
    panels: [
        scriptPanel(
            "10a",
            "Elena on Kaveri's comms display. Behind Jonah, Tomas continues the route work.",
            [
                say(
                    "jonah-1",
                    "Jonah",
                    "Three people. We can reach them. It will put the replacement in late."
                ),
                say(
                    "elena-2",
                    "Elena / comms",
                    "Understood. I'll ask EarthWorks to cover the diversion. Keep preparing."
                ),
            ]
        ),
        scriptPanel(
            "10b",
            "Aboard Kaveri, Samir monitors Elena's call with Daniel Reed at EarthWorks Operations. Clear speaker labels identify the two ends of the call. Keep the panel on that exchange, rather than crowding the other preparations behind him.",
            [
                say(
                    "elena-1",
                    "Elena / comms",
                    "Kaveri can reach your crew in time. Will EarthWorks cover the diversion?"
                ),
                say(
                    "daniel-2",
                    "Daniel / comms",
                    "No. We have a recovery scheduled."
                ),
                say(
                    "elena-3",
                    "Elena / comms",
                    "Your people won't last that long."
                ),
                say(
                    "daniel-4",
                    "Daniel / comms",
                    "I'm not approving the cost."
                ),
            ]
        ),
        scriptPanel(
            "10c",
            "Elena speaks to Jonah again. He looks to Tomas, not toward an imaginary approval indicator. Leave room around the final instruction.",
            [
                say(
                    "elena-1",
                    "Elena / comms",
                    "Clearwell will cover it. Bring them home."
                ),
                say("jonah-2", "Jonah", "Tomas. Take the intercept."),
            ]
        ),
    ],
});
