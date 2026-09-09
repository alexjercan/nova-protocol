import { scriptPage, scriptPanel, say } from "../../../comic-script";

export default scriptPage({
    id: "page-8",
    title: "A familiar voice",
    purpose: "Replace the routine return with a specific, credible need.",
    panels: [
        scriptPanel(
            "8a",
            "Samir at communications. A distress call interrupts the quiet. Jonah turns before Samir has finished identifying the sender.",
            [
                say(
                    "nadia-1",
                    "Nadia / comms",
                    "Gantry, requesting assistance. Main propulsion disabled."
                ),
                say("samir-2", "Samir", "That's the captain from Aquila."),
            ]
        ),
        scriptPanel(
            "8b",
            "Jonah answers. Stay aboard Kaveri; no cutaway to the attack or Gantry's interior.",
            [
                say(
                    "jonah-1",
                    "Jonah",
                    "Gantry, Kaveri. We hear you. How many aboard?"
                ),
                say(
                    "nadia-2",
                    "Nadia / comms",
                    "Three. All alive. Owen's hurt. We've isolated the damaged spaces."
                ),
            ]
        ),
        scriptPanel(
            "8c",
            "Samir records the report while Tomas begins checking the relative tracks. Information comes from Nadia, not an omniscient map labelling every danger.",
            [
                say("samir-1", "Samir", "What's still working?"),
                say(
                    "nadia-2",
                    "Nadia / comms",
                    "Emergency power. Comms, attitude control, limited life support. They took the cargo and left us here."
                ),
                say(
                    "jonah-3",
                    "Jonah",
                    "Send your reserve estimate and the port status. We're checking an intercept."
                ),
            ]
        ),
    ],
});
