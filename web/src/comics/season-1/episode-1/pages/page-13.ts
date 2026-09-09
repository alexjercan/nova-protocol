import { scriptPage, scriptPanel, say } from "../../../comic-script";

export default scriptPage({
    id: "page-13",
    title: "Hold there",
    purpose:
        "Let controlled movement, not a surprise failure, carry the tension.",
    panels: [
        scriptPanel(
            "13a",
            "Exterior close enough to understand the remaining gap. Gantry's damaged systems stay inactive. Kaveri approaches the verified port; keep both hull identities clear and do not add a second docking arrangement for this view.",
            [
                say(
                    "tomas-1",
                    "Tomas / comms",
                    "Holding here for the port check."
                ),
                say("leila-2", "Leila / comms", "Give me the alignment view."),
            ]
        ),
        scriptPanel(
            "13b",
            "Leila and Tomas concentrate on their different parts of the same operation. Neither explains basic flight controls to the competent captain.",
            [
                say(
                    "leila-1",
                    "Leila",
                    "That's the port we checked. Approach is clear."
                ),
                say("tomas-2", "Tomas", "Coming in."),
            ]
        ),
        scriptPanel(
            "13c",
            "The ships connected, their bulk suddenly close. Both coast without thrust; the transfer spaces are in freefall. The moment is quiet rather than triumphant. Pressure and seal checks occur before the following page's opening; the particular transfer hardware remains a design decision.",
            [
                say(
                    "leila-1",
                    "Leila / comms",
                    "Connection secure. Checking the seal."
                ),
            ]
        ),
    ],
});
