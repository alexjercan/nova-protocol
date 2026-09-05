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
import { ART, BRANDT, CALLOWAY, PELL } from "../cast";

export default comicPage(
    chapterHeader({
        eyebrow: "Prologue // Four years ago",
        number: "00",
        title: "Nobody searched",
        subtitle: "The desk speaks, the boat deck listens",
    }),
    grid(
        { columns: 2, rows: [1.2, 1] },
        widePanel(
            { variant: "crt", label: "Meridian's boat deck" },
            svgAsset(ART.boatDeck, {
                alt: "Meridian's boat bay. Under the WE ARE EXPANDING banner a wall screen reads SECTION NINE, ROSTER CLOSED, NO RECOVERY. Deck crew turn toward the screen. A helmeted figure in the foreground watches them.",
            }),
            speech(
                CALLOWAY,
                "All stations, Meridian. Kestrel has suffered a reactor failure. Survey Three is lost with the site. Section nine applies. No recovery. Hold your posts.",
                { at: "top-left", channel: "open" }
            ),
            narration("Meridian's boat deck. Dorian Pell ran it.", {
                at: "bottom-right",
            })
        ),
        panel(
            { variant: "crt", label: "Pell refuses" },
            svgAsset(ART.pellRefuses, {
                alt: "Dorian Pell, close: a weathered face under a raised helmet visor, a heavy deck headset, a set mouth. The banner hangs cut off behind him.",
            }),
            narration(
                "Against the desk, against the roster, on the open channel.",
                {
                    tone: "amber",
                }
            ),
            speech(PELL, "All hands. Tenders away.", { channel: "open" })
        ),
        panel(
            { variant: "crt", label: "Brandt fits the boat" },
            svgAsset(ART.brandtFitsTheBoat, {
                alt: "Tender Four in its cradle under the bay lamps. Brandt works the clamps with a tool while a helmeted hand climbs aboard. A screen reads TENDER 4, CLAMPS RELEASED.",
            }),
            speech(BRANDT, "Clamps free. She's fitted, Dorian. Go.", {
                at: "bottom-left",
            }),
            speech(
                PELL,
                "Stay on the deck, Brandt. Somebody has to fit the next one.",
                { at: "top-right" }
            )
        )
    )
);
