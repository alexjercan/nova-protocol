import { scriptPage, scriptPanel, say } from "../../../comic-script";

export default scriptPage({
    id: "page-12",
    title: "One way in",
    purpose: "Give the stranded crew useful judgment in their own rescue.",
    panels: [
        scriptPanel(
            "12a",
            "Leila studies the port area as Kaveri closes. Do not reveal hidden interior systems through an impossible scan. Her questions go to the people aboard.",
            [
                say(
                    "leila-1",
                    "Leila",
                    "Gantry, did the electrical fault reach the docking equipment?"
                ),
                say(
                    "owen-2",
                    "Owen / comms",
                    "Not that branch. We isolated the main distribution. Leave it isolated."
                ),
            ]
        ),
        scriptPanel(
            "12b",
            "Rina prepares Kaveri's transfer side. Samir clears a receiving space and brings first-aid equipment. No equipment specification or permanent passenger layout is established by this staging.",
            [
                say(
                    "leila-1",
                    "Leila / comms",
                    "Agreed. We'll check the connection before anyone crosses."
                ),
                say(
                    "ivo-2",
                    "Ivo / comms",
                    "The access is clear. Owen needs help moving."
                ),
                say(
                    "rina-3",
                    "Rina",
                    "Stay with him. We'll meet you at the opening."
                ),
            ]
        ),
        scriptPanel(
            "12c",
            "Jonah glances toward Samir rather than asking him to perform a miracle.",
            [
                say("jonah-1", "Jonah", "Ready for him?"),
                say(
                    "samir-2",
                    "Samir",
                    "Ready to help him aboard. I'll need their account of the injury."
                ),
            ]
        ),
    ],
});
