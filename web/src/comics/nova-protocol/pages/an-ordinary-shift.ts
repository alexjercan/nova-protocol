import {
    chapterHeader,
    comicPage,
    grid,
    locationCard,
    narration,
    panel,
    speech,
    svgAsset,
    widePanel,
} from "../../comic-page";
import { ART, CONTROL, HALLORAN, OKORO } from "../cast";

export default comicPage(
    chapterHeader({
        eyebrow: "Act one // The job",
        number: "01",
        title: "An ordinary shift",
        subtitle: "The junk site, Saturn's rings. Today.",
    }),
    grid(
        { columns: 2, rows: [1.25, 1] },
        widePanel(
            { variant: "crt", label: "The boat bay" },
            svgAsset(ART.boatBayShift, {
                alt: "Meridian's boat bay. The gold WE ARE EXPANDING banner and the plaque for Dorian Pell hang side by side on the back wall, a deck screen beside them counting down to under way. Three crew walk toward Cutter One on its clamps: a helmet, a jacket, a tool bag.",
            }),
            narration(
                "Every shift reported under the banner and the plaque. Section nine was the standing joke. The captain still laughed at it. Halloran and Okoro had stopped years ago.",
                { at: "bottom-left" }
            ),
            speech(HALLORAN, "Section nine says good morning, Captain.", {
                at: "top-right",
                channel: "cabin",
            })
        ),
        panel(
            { variant: "crt", label: "The junk site" },
            svgAsset(ART.junkSite, {
                alt: "A ring moonlet under dead hulls and scrap: a broken mining barge with its ribs showing, a dead bone-white Kestrel boat, plates and spars. Meridian sits among them with its boat bay open on the gold banner, and Cutter One leaves it on a dotted track.",
            }),
            locationCard("The junk site", "Today, shift three", {
                at: "top-right",
            }),
            narration(
                "Dead hulls and scrap nobody can name, left by the small miners EWI finished. Meridian came to clear it for the new stations.",
                { at: "bottom-left", tone: "muted" }
            )
        ),
        panel(
            { variant: "crt", label: "Released" },
            svgAsset(ART.cutterReleased, {
                alt: "Meridian broadside and close, its bay open. Cutter One, a small glass-nosed boat with an open truss aft, drops away from it toward the junk on a dotted track.",
                focus: "left",
            }),
            speech(
                CONTROL,
                "Cutter One, Control. Bay is clear, you are released. Under way in fifty-six minutes. Don't make me come looking for you.",
                { at: "top-left", channel: "open" }
            ),
            speech(OKORO, "They'd leave without you.", {
                at: "bottom-right",
                channel: "cabin",
            })
        )
    )
);
