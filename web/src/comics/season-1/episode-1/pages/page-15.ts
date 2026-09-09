import { scriptPage, scriptPanel, say } from "../../../comic-script";

export default scriptPage({
    id: "page-15",
    title: "All three",
    purpose: "Finish the evacuation without inventing a hull destruction.",
    panels: [
        scriptPanel(
            "15a",
            "From Kaveri's side of the passage, Jonah sees Nadia cross last. She brings the crew's working records, not a conveniently labelled conspiracy file.",
            [
                say(
                    "nadia-1",
                    "Nadia",
                    "That's everyone. Three aboard Kaveri."
                ),
                say("jonah-2", "Jonah", "All accounted for."),
            ]
        ),
        scriptPanel(
            "15b",
            "Rina checks that the passage is clear. Leila prepares the separation. Owen stays with Samir; do not crowd everyone onto the flight deck for a group portrait.",
            [
                say("rina-1", "Rina", "Transfer side clear."),
                say("leila-2", "Leila", "Closing and checking."),
            ]
        ),
        scriptPanel(
            "15c",
            "Kaveri separates from Gantry. Hold enough space around the abandoned hull to let its loss of usefulness register. It does not explode. No pirates appear.",
            [
                say(
                    "jonah-1",
                    "Jonah / comms",
                    "Baikal, Kaveri. We have all three. Coming home."
                ),
            ]
        ),
    ],
});
