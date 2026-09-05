import {
    chapterHeader,
    comicPage,
    divider,
    feed,
    grid,
    narration,
    panel,
    readout,
    svgAsset,
    widePanel,
} from "../../comic-page";
import { ART } from "../cast";

export default comicPage(
    chapterHeader({
        eyebrow: "Prologue // Four years ago",
        number: "00",
        title: "The plaque",
        subtitle: "The press, the file, and the wall",
    }),
    grid(
        { columns: 2, rows: [1.3, "auto"] },
        widePanel(
            { variant: "crt", label: "The boat bay wall" },
            svgAsset(ART.thePlaque, {
                alt: "The boat bay wall. The gold WE ARE EXPANDING banner, and beside it a metal plaque: DORIAN PELL, BOAT DECK, EWI MERIDIAN, LOST IN RESCUE, ALL HANDS, TENDERS AWAY, WE REMEMBER OUR OWN. A crewman with a drill bolts it on.",
            }),
            narration(
                "The company's two faces went up side by side, and every shift reported under both.",
                { at: "bottom-left", tone: "amber" }
            )
        ),
        panel(
            { variant: "crt", label: "The press" },
            feed("Saturn wire // EWI press desk", [
                "Meridian boat-deck chief lost in rescue attempt at Kestrel site",
                "Kestrel Shelter destroyed by reactor failure. No EWI casualties.",
                "EWI: a hero. We are expanding, and we remember our own.",
            ])
        ),
        panel(
            { variant: "crt", label: "The file" },
            readout(
                [
                    "EWI personnel // Pell, D.",
                    "Status: lost. Section nine.",
                    "Roster closed. Costs settled against term.",
                    "Line 1 of 1.",
                ],
                "muted"
            )
        ),
        divider(
            "Kestrel folded in weeks. EWI bought the survivors. Calloway took the desk."
        )
    )
);
