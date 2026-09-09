import { scriptPage, scriptPanel, say } from "../../../comic-script";

export default scriptPage({
    id: "page-14",
    title: "Bring him through",
    purpose:
        "Rescue people through coordinated work, without an on-foot game promise.",
    panels: [
        scriptPanel(
            "14a",
            "After the seal and pressure checks, Rina meets Ivo at the opened passage, using a handhold to stop herself. We enter Gantry's space only with Rina. Owen is alert but cannot move safely unaided. He is supported and secured for the freefall transfer; his exact injury and support equipment remain for staging review.",
            [
                say("rina-1", "Rina", "I'm Rina. Tell me what he can manage."),
                say(
                    "ivo-2",
                    "Ivo",
                    "He's been helping with the checks. Moving is the problem."
                ),
                say("owen-3", "Owen", "I can tell you when to stop."),
            ]
        ),
        scriptPanel(
            "14b",
            "Rina and Ivo guide Owen and his support through the passage, controlling his motion with handholds. They do not carry his weight under their arms. Give the secured body, their hands, and the clear route enough space to show how they start and stop together. Weightlessness does not remove his injury or inertia.",
            [
                say("rina-1", "Rina", "Good. We move together. Ready?"),
                say("owen-2", "Owen", "Ready."),
            ]
        ),
        scriptPanel(
            "14c",
            "Samir receives Owen on Kaveri's side. Rina and Ivo keep his motion controlled until Samir has secured his support at the receiving position. Nobody lets go and expects him to stay put. The scene does not imply he needs no further care.",
            [
                say(
                    "samir-1",
                    "Samir",
                    "Owen, I'm Samir. Let's get you supported before we do anything else."
                ),
                say("owen-2", "Owen", "I'd like that."),
            ]
        ),
    ],
});
