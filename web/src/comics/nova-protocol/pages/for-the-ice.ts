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
import { ART, CONTROL } from "../cast";

export default comicPage(
    chapterHeader({
        eyebrow: "Prologue // Four years ago",
        number: "00",
        title: "For the ice",
        subtitle: "Kestrel claim K-7, Saturn's rings",
    }),
    grid(
        { columns: 2, rows: [1.25, 1] },
        widePanel(
            { variant: "crt", label: "The Shelter on its rock" },
            svgAsset(ART.shelterWide, {
                alt: "The Shelter, a bone-white tower and habitat ring on a ring rock, its windows amber. The supply boat Harrier leaves it. Meridian waits small and far on the other side of a dashed survey line.",
            }),
            narration(
                "Kestrel mined ice around Saturn and named its stations after the places birds live. The Shelter was the largest. EarthWorks Industrial held the claim next door.",
                { at: "bottom-left" }
            ),
            speech(
                CONTROL,
                "Harrier, Meridian Control. You are clear of the survey lane. Safe run.",
                { at: "top-right", channel: "open" }
            )
        ),
        panel(
            { variant: "crt", label: "Meridian on station" },
            svgAsset(ART.meridianOnStation, {
                alt: "EWI's carrier Meridian broadside, khaki hull and phosphor windows, its boat bay open on a gold banner reading WE ARE EXPANDING. A survey tender leaves it.",
                focus: "left",
            }),
            speech(
                CONTROL,
                "Survey Three, Control. The charge site is plotted. You are cleared to the ice.",
                { at: "top-right", channel: "open" }
            )
        ),
        panel(
            { variant: "crt", label: "The survey plot" },
            svgAsset(ART.surveyPlot, {
                alt: "A console plot of claim K-7. A gold diamond marked DEMOLITION, FOR THE ICE sits on the survey line, 240 metres from the Shelter, inside a dashed blast radius that covers the station.",
            }),
            narration(
                "Everyone at working level read the plot the same way. Demolition, for the ice.",
                { tone: "amber" }
            )
        )
    )
);
