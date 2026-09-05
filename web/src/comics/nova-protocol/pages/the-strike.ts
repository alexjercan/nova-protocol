import { bleedPage, caption, chapterHeader, hud } from "../../comic-page";
import { ART } from "../cast";

export default bleedPage(
    {
        image: ART.theStrike,
        alt: "A phosphor flash in the junk. Meridian breaks in two, its stern and bow tumbling apart in khaki shards. Severance stands off to the right under the moonlet, rail apertures blazing, two bolts crossing to the carrier. Cutter One is a dark speck under a dead barge at the bottom left.",
        focus: "center",
    },
    chapterHeader({
        eyebrow: "Act one // The job",
        number: "01",
        title: "The strike",
        subtitle: "Out of the moonlet's shadow",
    }),
    hud(
        [
            ["Meridian", "no signal", "danger"],
            ["Contact", "no code", "danger"],
            ["Cutter One", "dark", "amber"],
        ],
        { at: "top-right" }
    ),
    caption(
        "The hull came out of the moonlet's shadow with no fleet code, read Meridian its clause, and killed it in the time the reading took. Then it left, because every second on the air was a second for Fleet to fix it.",
        "danger"
    )
);
