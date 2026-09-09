import { scriptPage, scriptPanel, say } from "../../../comic-script";

export default scriptPage({
    id: "page-16",
    title: "The return",
    purpose:
        "Let the survivors react differently without instant political conversion.",
    panels: [
        scriptPanel(
            "16a",
            "Later aboard Kaveri. Owen rests with support nearby. Ivo has learned of the funding refusal. Nadia asks Samir for the actual record rather than joining in an accusation she cannot yet support.",
            [
                say("ivo-1", "Ivo", "They knew we were alive?"),
                say(
                    "samir-2",
                    "Samir",
                    "Elena sent them the estimate before the refusal."
                ),
                say(
                    "nadia-3",
                    "Nadia",
                    "Send me that exchange. I want an answer from Operations."
                ),
            ]
        ),
        scriptPanel(
            "16b",
            "Jonah pauses beside them during the return. A quiet acknowledgement, not a speech about what kind of person he is.",
            [
                say("nadia-1", "Nadia", "Thank you for coming."),
                say("jonah-2", "Jonah", "I'm glad we reached you."),
            ]
        ),
    ],
});
