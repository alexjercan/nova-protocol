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
import { ART, CALLOWAY, CONTROL, PELL } from "../cast";

export default comicPage(
    chapterHeader({
        eyebrow: "Prologue // Four years ago",
        number: "00",
        title: "Tenders away",
        subtitle: "One boat, volunteers, and the dark",
    }),
    grid(
        { columns: 2, rows: [1.2, 1] },
        widePanel(
            { variant: "crt", label: "Tender Four crosses" },
            svgAsset(ART.tenderTowardTheShelter, {
                alt: "Tender Four, small and lit, crosses the dark toward the wreck of the Shelter, where embers still glow in the broken ring. Meridian holds far behind it.",
            }),
            speech(
                PELL,
                "Meridian, Tender Four. We are going to the Shelter. Log it however you like.",
                { at: "top-left", channel: "open" }
            ),
            speech(
                CALLOWAY,
                "Tender Four, you are off the roster. Meridian will not follow.",
                { channel: "open" }
            )
        ),
        panel(
            { variant: "crt", label: "Signal lost" },
            svgAsset(ART.signalLost, {
                alt: "The boat deck plot at forty-one minutes. A gold track runs from Meridian toward the Shelter's dead mark and ends in a cross: TENDER 4, SIGNAL LOST. Below, the log: section nine applied, no search ordered, Kestrel reactor failure reported.",
            }),
            narration(
                "Forty-one minutes out, Tender Four went dark. Meridian held position.",
                { at: "bottom-left", tone: "muted" }
            )
        ),
        panel(
            { variant: "crt", label: "The desk closes the file" },
            transcript(
                [
                    [
                        CALLOWAY,
                        "Log Tender Four under section nine. No search.",
                        "danger",
                    ],
                    [CONTROL, "And Kestrel, sir?"],
                    [CALLOWAY, "Reactor failure. My voice.", "danger"],
                    [CONTROL, "Logged."],
                ],
                { title: "Meridian recorder // T+00:44" }
            )
        )
    )
);
