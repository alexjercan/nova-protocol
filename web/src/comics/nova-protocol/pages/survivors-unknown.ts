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
import { ART, BEACON, HALLORAN, OKORO } from "../cast";

export default comicPage(
    chapterHeader({
        eyebrow: "Act one // The job",
        number: "01",
        title: "Survivors unknown",
        subtitle: "The junk, the skiffs, the beacon",
    }),
    grid(
        { columns: 2, rows: [1, 1.3] },
        panel(
            { variant: "crt", label: "Halloran" },
            svgAsset(ART.halloran, {
                alt: "Nadia Halloran in Cutter One's cabin, lit from below by the console. A swept dark fringe, a slim headset, a teal jacket. The rings show through the canopy behind her.",
            }),
            narration("Nadia Halloran, copilot. Flew for Kestrel once.", {
                at: "top-left",
                tone: "muted",
            }),
            speech(HALLORAN, "Carrier channel is gone.", {
                at: "bottom-right",
                channel: "cabin",
            })
        ),
        panel(
            { variant: "crt", label: "Okoro" },
            svgAsset(ART.okoro, {
                alt: "Bastian Okoro in Cutter One's cabin, goggles pushed up on his forehead, a boom mic at his cheek, an amber work coverall. The same canopy and console light.",
            }),
            narration(
                "Bastian Okoro, engineer. Learned the boat deck on Meridian, under Brandt.",
                { at: "top-left", tone: "muted" }
            ),
            speech(
                OKORO,
                "Wait. I still have one carrier signal. Weak, but it's running.",
                { at: "bottom-right", channel: "cabin" }
            )
        ),
        widePanel(
            { variant: "crt", label: "The sweep" },
            svgAsset(ART.hidingInTheJunk, {
                alt: "Three scrap-built skiffs sweep the junk with amber searchlights. Meridian's stern hangs dark in the background with embers in it and pulse rings around it. Under a broken barge in the foreground, Cutter One lies dark and unlit.",
            }),
            speech(
                BEACON,
                "Any vessel. Any vessel. This is Meridian. Hull breach all decks. Survivors unknown.",
                { at: "top-right", channel: "open", tone: "danger" }
            ),
            narration(
                "Skiffs came to sweep the field before the wreck had cooled. Cutter One went dark in the junk and waited. 412 dead. Brandt among them. Nobody at Saturn had been told a warship was missing.",
                { at: "bottom-left" }
            )
        )
    )
);
