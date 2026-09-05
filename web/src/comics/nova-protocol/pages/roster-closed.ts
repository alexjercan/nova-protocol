import { bleedPage, caption, chapterHeader, hud } from "../../comic-page";
import { ART } from "../cast";

export default bleedPage(
    {
        image: ART.theBlast,
        alt: "A phosphor flash on the survey line. The Shelter's tower breaks away from its ring in bone-white shards. A dark tender tumbles in the light with a suited figure beside it. Meridian is a silhouette at the edge.",
        focus: "right",
    },
    chapterHeader({
        eyebrow: "Prologue // Four years ago",
        number: "00",
        title: "Roster closed",
        subtitle: "The Shelter opens",
    }),
    hud(
        [
            ["Site", "K-7"],
            ["Survey 3", "no signal", "danger"],
            ["Kestrel Shelter", "no signal", "danger"],
        ],
        { at: "top-right" }
    ),
    caption(
        "The charge went off on the mark. The Shelter opened along its ring. Survey Three was still beside the charge. Nobody had meant to bring them back.",
        "danger"
    )
);
