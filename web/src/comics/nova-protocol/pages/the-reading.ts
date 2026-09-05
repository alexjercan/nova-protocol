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
import { ART, CONTROL, HALLORAN, VOICE } from "../cast";

export default comicPage(
    chapterHeader({
        eyebrow: "Act one // The job",
        number: "01",
        title: "The reading",
        subtitle: "A voice on the guard channel, and not Demir's",
    }),
    grid(
        { columns: 2, rows: [1.15, "auto"] },
        panel(
            { variant: "crt", label: "The guard channel" },
            svgAsset(ART.theReading, {
                alt: "Cutter One's comms screen on the guard channel at twelve percent signal. A jagged trace, and in it one word comes through bright: ROSTER. Fainter fragments around it: MERIDIAN CONTR, SECTION N, NO REC. Below, a plot: Cutter One inbound, Meridian holding with its bay open, and a gold cross at the survey body's edge: CONTACT, NO CODE, drive plume clearing the body.",
            }),
            speech(VOICE, "...roster...", {
                at: "top-right",
                channel: "guard",
                tone: "danger",
            }),
            narration(
                "Inbound to the outer hold, the work channel caught Meridian's guard channel in fragments. Someone was reading something out, slowly. It was not Demir.",
                { at: "bottom-left" }
            )
        ),
        panel(
            { variant: "crt", label: "Out of the shadow" },
            svgAsset(ART.outOfTheShadow, {
                alt: "A long grey warship slides out of the moonlet's shadow, its drive plume faint behind it, two rail apertures dark at its bow. Across its flank, in gold company letters over a painted-out Fleet name: SEVERANCE.",
                focus: "right",
            }),
            speech(
                CONTROL,
                "Cutter One, hold at the outer mark. We have a drive plume clearing the large body and no transponder.",
                { at: "top-left", channel: "open" }
            ),
            speech(
                HALLORAN,
                "Control, that's a Fleet hull. It isn't broadcasting a fleet code. It's turning toward you.",
                { at: "bottom-right", channel: "open" }
            )
        ),
        widePanel(
            { variant: "crt", label: "The recorder" },
            transcript(
                [
                    [VOICE, "Meridian Control. Section nine.", "amber"],
                    [VOICE, "Roster closed. No recovery.", "danger"],
                    [
                        CONTROL,
                        "Unidentified warship, this is EarthWorks carrier Meridian. Civilian registry. We are unarmed. Identify yourself.",
                    ],
                    [CONTROL, "All stations, brace. Brace. Br", "danger"],
                ],
                {
                    title: "Meridian recorder // guard channel // recovered later",
                }
            )
        )
    )
);
