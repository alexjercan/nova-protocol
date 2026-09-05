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
import { ART, CONTROL, RUIZ, TAMM } from "../cast";

export default comicPage(
    chapterHeader({
        eyebrow: "Prologue // Four years ago",
        number: "00",
        title: "The charge",
        subtitle: "Fleet surplus on a company budget line",
    }),
    grid(
        { columns: 2, rows: [1.2, 1] },
        widePanel(
            { variant: "crt", label: "Survey Three at the charge" },
            svgAsset(ART.tenderAtTheCharge, {
                alt: "Two suited crew on the ice set a demolition charge between them while their tender floats above on tethers. Across the survey line, the Shelter fills the sky.",
            }),
            speech(
                RUIZ,
                "Fleet surplus. Who buys a demolition charge to crack ice?",
                { at: "top-left" }
            ),
            speech(TAMM, "Somebody with a budget line. Set the clamps.")
        ),
        panel(
            { variant: "crt", label: "The charge, armed" },
            svgAsset(ART.chargeArmed, {
                alt: "Close on the charge: a khaki canister with gold hazard bands, stencilled EF DEMO-12, SURPLUS, FLEET, and a marker note reading FOR THE ICE. A gloved hand rests on its clamp and its arming lamp is lit.",
            }),
            speech(
                TAMM,
                "Control, Survey Three. Charge is set and armed. Request pickup.",
                { channel: "open" }
            )
        ),
        panel(
            { variant: "crt", label: "Hold at the site" },
            svgAsset(ART.holdAtTheSite, {
                alt: "The tender and its two crew wait on the ice beside the charge. Across the line the Shelter stands close, every window lit.",
            }),
            speech(
                CONTROL,
                "Survey Three, hold at the site. Pickup is scheduled.",
                { at: "top-left", channel: "open" }
            ),
            narration("They held. The Shelter filled the window.", {
                at: "bottom-right",
                tone: "muted",
            })
        )
    )
);
