import {
    chapterHeader,
    comicPage,
    grid,
    narration,
    panel,
    readout,
    speech,
    svgAsset,
    transcript,
    widePanel,
} from "../../comic-page";
import { ART, CALLOWAY, CONTROL } from "../cast";

export default comicPage(
    chapterHeader({
        eyebrow: "Prologue // Four years ago",
        number: "00",
        title: "For the Shelter",
        subtitle: "The order, the voice, the record",
    }),
    grid(
        { columns: 2, rows: [1.3, "auto"] },
        panel(
            { variant: "crt", label: "The Board's order" },
            readout(
                [
                    "EWI Board // priority // eyes only",
                    "To: Meridian, Cmdr Calloway",
                    "Re: Kestrel claim K-7",
                    "Finish the Shelter. Ice accident.",
                    "No witnesses at working level.",
                    "Section nine applies to the placement crew.",
                    "Acknowledge by voice.",
                ],
                "amber"
            )
        ),
        panel(
            { variant: "crt", label: "Calloway on the bridge" },
            svgAsset(ART.callowayBridge, {
                alt: "Ines Calloway on Meridian's bridge, lit from below by a console. A slim headset, a gold-edged officer's collar, a level face. The Shelter shows through the window behind him.",
            }),
            narration("Ines Calloway, commanding Meridian.", {
                tone: "muted",
            }),
            speech(
                CALLOWAY,
                "Board, Meridian. Calloway. Understood. Survey Three holds at the site.",
                { channel: "rec" }
            )
        ),
        widePanel(
            { variant: "crt", label: "The recorder" },
            transcript(
                [
                    [
                        CALLOWAY,
                        "Detonation on my mark. Log the event as ice.",
                        "amber",
                    ],
                    [
                        CONTROL,
                        "Logged. Survey Three is still at the site, sir.",
                    ],
                    [CALLOWAY, "I know where Survey Three is. Mark.", "danger"],
                ],
                {
                    title: "Meridian recorder // bridge channel // kept four years",
                }
            )
        )
    )
);
