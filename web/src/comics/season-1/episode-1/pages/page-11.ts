import { scriptPage, scriptPanel, say } from "../../../comic-script";

export default scriptPage({
    id: "page-11",
    title: "Across the gap",
    purpose:
        "Make the journey and the change of course tangible without a fight.",
    panels: [
        scriptPanel(
            "11a",
            "A broad exterior: Kaveri on the new course, its replacement assembly still secured. The ship is small against the distance. Do not draw looping jet-fighter trails or a continuous burn simply to fill the panel. No dialogue.",
            []
        ),
        scriptPanel(
            "11b",
            "Later, with Gantry now resolved ahead, Kaveri's crew compares the transmitted condition report with what they can see. The hull is stable, not tumbling. Damage interrupts the form we saw intact at Aquila.",
            [
                say(
                    "tomas-1",
                    "Tomas",
                    "Gantry in sight. Matching their motion."
                ),
                say(
                    "nadia-2",
                    "Nadia / comms",
                    "We can hold attitude. Main drive is out."
                ),
                say("jonah-3", "Jonah", "Understood. We come to you."),
            ]
        ),
    ],
});
