import {
    chapterHeader,
    comicPage,
    grid,
    narration,
    panel,
    speech,
    svgAsset,
    widePanel,
} from "../../comic-page";
import { ART, CONTROL, GUARD, HALLORAN } from "../cast";

export default comicPage(
    chapterHeader({
        eyebrow: "Act one // The job",
        number: "01",
        title: "The donut",
        subtitle: "Behind the survey body, out of Meridian's sightline",
    }),
    grid(
        { columns: 2, rows: [1.3, 1] },
        widePanel(
            { variant: "crt", label: "The survey body" },
            svgAsset(ART.theDonut, {
                alt: "A ring moonlet with a dashed orbit track around it. Cutter One rides the track on the lit side. Meridian is small and far to the left beyond the junk. On the moonlet's dark side, where the shadow falls, a long shape with no lights lies against the black.",
            }),
            speech(
                GUARD,
                "...Earth Fleet... any vessel... Resolute... report contact to...",
                { at: "top-right", channel: "guard", tone: "muted" }
            ),
            narration(
                "Orbit hold was not on the release. The captain flew a donut around the survey body anyway and logged it as a gravity check. Fleet crackled on the guard channel, looking for something. Fleet is always looking for something.",
                { at: "bottom-left" }
            )
        ),
        panel(
            { variant: "crt", label: "Meridian Control" },
            svgAsset(ART.demirAtControl, {
                alt: "Yusra Demir at Meridian Control, lit from below by her console, a slim headset on. Screens beside her read CUTTER ONE, ORBIT HOLD, SIGHTLINE RESTORED, and MERIDIAN, UNDER WAY IN 00:31, RELEASE 7-R FILED.",
            }),
            narration("Yusra Demir, Meridian Control.", {
                at: "top-left",
                tone: "muted",
            }),
            speech(
                CONTROL,
                "Cutter One, Meridian Control. Explain the orbit.",
                {
                    at: "bottom-right",
                    channel: "open",
                }
            )
        ),
        panel(
            { variant: "crt", label: "The third crate" },
            svgAsset(ART.thirdCrateHome, {
                alt: "Cutter One with three crates in its rack, heading back toward Meridian, whose bay is open. A mark ahead reads OUTER HOLD.",
                focus: "left",
            }),
            speech(
                CONTROL,
                "Your release was filed six minutes ago. We get under way inside the hour and the bonus needs a clean manifest. Bring me that third crate.",
                { at: "top-left", channel: "open" }
            ),
            speech(HALLORAN, "Three for three, and minutes to spare.", {
                at: "bottom-right",
                channel: "cabin",
            })
        )
    )
);
