import { scriptPage, scriptPanel, say } from "../../../comic-script";

export default scriptPage({
    id: "page-9",
    title: "What it will cost",
    purpose: "Let each crewmember contribute to an informed decision.",
    panels: [
        scriptPanel(
            "9a",
            "Tomas brings the revised route to Jonah. No invented numerical countdown is printed. The drawing distinguishes Kaveri's possible arrival from Gantry's expected endurance. Only the arrival ordering matters here, not a plotted orbit.",
            [
                say(
                    "tomas-1",
                    "Tomas",
                    "We can reach them before their reserves run out. The scheduled recovery can't."
                ),
                say("jonah-2", "Jonah", "And Baikal?"),
                say(
                    "tomas-3",
                    "Tomas",
                    "Later. Enough that we miss the repair plan. Ebro's load won't be ready."
                ),
            ]
        ),
        scriptPanel(
            "9b",
            "Rina and Leila assess the transfer route beside the secured assembly. The replacement stays aboard. Make the distinction between preserving cargo and clearing an assisted passage visible.",
            [
                say(
                    "rina-1",
                    "Rina",
                    "We can bring three people through here. The assembly stays tied down."
                ),
                say(
                    "leila-2",
                    "Leila",
                    "If their port is sound, we can dock. We don't need to restart their main bus."
                ),
                say("rina-3", "Rina", "Then let's get them off it."),
            ]
        ),
        scriptPanel(
            "9c",
            "Tomas is not isolated as the scene's unkind person. Jonah takes the warning seriously before making a commitment on Clearwell's behalf.",
            [
                say(
                    "tomas-1",
                    "Tomas",
                    "I want them out too. I want Elena to know what we're spending."
                ),
                say(
                    "jonah-2",
                    "Jonah",
                    "She will. Keep working the intercept."
                ),
            ]
        ),
    ],
});
