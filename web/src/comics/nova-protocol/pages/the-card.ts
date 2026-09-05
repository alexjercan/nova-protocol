import {
    chapterHeader,
    comicPage,
    grid,
    narration,
    panel,
    speech,
    svgAsset,
    transcript,
    widePanel,
} from "../../comic-page";
import { ART, CAPTAIN, HALLORAN, OKORO } from "../cast";

export default comicPage(
    chapterHeader({
        eyebrow: "Act one // The job",
        number: "01",
        title: "The card",
        subtitle: "Four trim marks, two transits, three crates",
    }),
    grid(
        { columns: 2, rows: [1.15, "auto"] },
        panel(
            { variant: "crt", label: "The maintenance release" },
            svgAsset(ART.handlingCard, {
                alt: "A console plot titled MAINTENANCE RELEASE, CUTTER ONE, CARD 7-R. Plate Seven is a dashed region with three crate marks, the third flagged NO MANIFEST. Four trim marks make a box. A gold route bends around a circle marked SURVEY BODY through TRANSIT 1 and TRANSIT 2. Meridian is under way in fifty-six minutes.",
            }),
            narration(
                "A re-qualification card after yard work. Fly the box, give two legs to the computer, bring in three crates off Plate Seven. Nothing was wrong.",
                { at: "bottom-left", tone: "amber" }
            )
        ),
        panel(
            { variant: "crt", label: "Plate Seven" },
            svgAsset(ART.cratesOnPlateSeven, {
                alt: "Cutter One deep in the junk with two crates in its rack, its work arm reaching for a third. Phosphor tag diamonds mark the recovered crates on their hulls. The third tag is gold: NO MANIFEST.",
            }),
            speech(
                OKORO,
                "Two secure, both manifests clean. The third turned up loose after the break. No mass on it.",
                { at: "top-left", channel: "cabin" }
            ),
            speech(
                CAPTAIN,
                "Of course it did. Lock Transit One and give the leg to the computer.",
                { at: "bottom-right", channel: "cabin" }
            )
        ),
        widePanel(
            { variant: "crt", label: "The handling card" },
            transcript(
                [
                    [
                        HALLORAN,
                        "Yard replaced the port manifold. Computer says everything is green.",
                    ],
                    [OKORO, "And you don't believe it."],
                    [
                        HALLORAN,
                        "Neither did Prospector Six, before they lost her in the belt.",
                    ],
                    [OKORO, "Different company. News said pilot error."],
                    [
                        HALLORAN,
                        "The flight recorder didn't. Port manifold locked open. Nobody aboard survived.",
                        "amber",
                    ],
                    [CAPTAIN, "Run your box."],
                ],
                { title: "Cutter One // cabin // before the box" }
            )
        )
    )
);
