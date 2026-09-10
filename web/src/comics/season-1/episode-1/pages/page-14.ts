import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-14",
    title: "Bring him through",
    purpose:
        "Rescue people through coordinated work, without an on-foot game promise.",
    panels: [
        panel("14a", {
            art: scene("meeting-at-opening"),
            action: "After the seal and pressure checks, Rina meets Ivo at the opened passage, using a handhold to stop herself. We enter Gantry's space only with Rina. Owen is alert but cannot move safely unaided. He is supported and secured on a padded stretcher with straps for the freefall transfer; his exact injury remains unspecified.",
            dialogue: [
                say("rina-1", "Rina", "I'm Rina. Tell me what he can manage."),
                say(
                    "ivo-2",
                    "Ivo",
                    "He's been helping with the checks. Moving is the problem."
                ),
                say("owen-3", "Owen", "I can tell you when to stop."),
            ],
            lettering: {
                "rina-1": balloon({
                    at: [18, 18],
                    width: 430,
                    tail: [91, 238],
                    side: "bottom",
                }),
                "ivo-2": balloon({
                    at: [466, 18],
                    width: 466,
                    tail: [650, 218],
                    side: "bottom",
                }),
                "owen-3": balloon({
                    at: [950, 18],
                    width: 448,
                    tail: [1230, 210],
                    side: "bottom",
                }),
            },
            labels: [
                label("gantry", "GANTRY / TRANSFER SIDE", {
                    slot: "gantry",
                    at: [485, 136],
                    size: 16,
                    color: "work-card-text",
                }),
            ],
        }),
        panel("14b", {
            art: scene("moving-together"),
            action: "Rina and Ivo guide Owen and his support through the passage, controlling his motion with handholds. They do not carry his weight under their arms. Give the secured body, their hands, and the clear route enough space to show how they start and stop together. Weightlessness does not remove his injury or inertia.",
            dialogue: [
                say("rina-1", "Rina", "Good. We move together. Ready?"),
                say("owen-2", "Owen", "Ready."),
            ],
            lettering: {
                "rina-1": balloon({
                    at: [18, 18],
                    width: 343,
                    tail: [176, 256],
                    side: "bottom",
                }),
                "owen-2": balloon({
                    at: [380, 18],
                    width: 142,
                    tail: [435, 210],
                    side: "bottom",
                }),
            },
        }),
        panel("14c", {
            art: scene("securing-support"),
            action: "Samir receives Owen on Kaveri's side. Rina and Ivo keep his motion controlled until Samir has secured his support at the receiving position. Nobody lets go and expects him to stay put. The scene does not imply he needs no further care.",
            dialogue: [
                say(
                    "samir-1",
                    "Samir",
                    "Owen, I'm Samir. Let's get you supported before we do anything else."
                ),
                say("owen-2", "Owen", "I'd like that."),
            ],
            lettering: {
                "samir-1": balloon({
                    at: [18, 18],
                    width: 464,
                    tail: [110, 257],
                    side: "bottom",
                }),
                "owen-2": balloon({
                    at: [288, 122],
                    width: 194,
                    tail: [292, 244],
                    side: "bottom",
                }),
            },
        }),
    ],
    layout: {
        "14a": { at: [42, 86], size: [1416, 320] },
        "14b": { at: [42, 426], size: [896, 517] },
        "14c": { at: [958, 426], size: [500, 517] },
    },
});
