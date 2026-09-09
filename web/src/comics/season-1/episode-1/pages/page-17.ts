import { scriptPage, scriptPanel, say } from "../../../comic-script";

export default scriptPage({
    id: "page-17",
    title: "Back to work",
    purpose:
        "Deliver both the people and the intact part; the community can continue.",
    panels: [
        scriptPanel(
            "17a",
            "Kaveri back at Baikal. Elena meets Jonah's crew and Gantry's people at arrival. Owen still receives assistance. Keep the welcome practical and relieved rather than staging a civic ceremony.",
            [
                say(
                    "elena-1",
                    "Elena",
                    "Welcome to Baikal. Let's get you settled."
                ),
            ]
        ),
        scriptPanel(
            "17b",
            "Leila and Rina bring the replacement into processing. Return to the visual anchors of page 2: isolated line, surrounding plant still safely operating.",
            [
                say("rina-1", "Rina", "Every cover still on it."),
                say("leila-2", "Leila", "Now we can take them off."),
            ]
        ),
        scriptPanel(
            "17c",
            "Later, with the assembly installed and checks complete, the isolated line returns to service. This is a time cut, not an instant repair. Ebro is still waiting for the delayed load; restarting a pump does not create the missing production immediately.",
            [
                say(
                    "leila-1",
                    "Leila / comms",
                    "Line's back. We'll have the load, just not on the old schedule."
                ),
            ]
        ),
    ],
});
