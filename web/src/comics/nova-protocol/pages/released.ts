import {
    caption,
    chapterHeader,
    circle,
    comicPage,
    grid,
    line,
    panel,
    speech,
    svg,
    svgAsset,
    svgText,
} from "../../comic-page";

const trimRoute = svg(
    { viewBox: [0, 0, 300, 180], label: "Four-mark RCS handling route" },
    line({ from: [55, 135], to: [245, 135], tone: "muted" }),
    line({ from: [245, 135], to: [245, 45], tone: "muted" }),
    line({ from: [245, 45], to: [55, 45], tone: "muted" }),
    line({ from: [55, 45], to: [55, 135], tone: "muted" }),
    ...(["A", "B", "C", "D"] as const).map((label, index) => {
        const points: [number, number][] = [
            [55, 135],
            [245, 135],
            [245, 45],
            [55, 45],
        ];
        return svgText(label, { at: points[index], tone: "default" });
    })
);

const orbit = svg(
    { viewBox: [0, 0, 300, 180], label: "Cutter One orbiting the survey body" },
    circle({ center: [150, 90], radius: 52, tone: "muted", fill: true }),
    circle({ center: [150, 90], radius: 78, tone: "amber" }),
    line({ from: [218, 52], to: [258, 38], tone: "default", width: 5 }),
    svgText("CUTTER ONE", { at: [178, 28], tone: "default" })
);

export default comicPage(
    chapterHeader({
        number: "01",
        title: "First Shift",
        subtitle: "Work // trust // home",
    }),
    grid(
        panel(
            { size: "wide" },
            svgAsset("first-shift.svg", {
                alt: "The industrial carrier Meridian and Cutter One in the belt.",
                focus: "carrier",
            }),
            speech(
                "control",
                "Cutter One, Meridian Control. Bay is clear. You are released."
            )
        ),
        panel(
            { variant: "work" },
            trimRoute,
            speech("copilot", "Four marks. Then I clear the handling card.")
        ),
        panel(
            { variant: "orbit" },
            orbit,
            speech("captain", "It is going in the log as a donut.", "amber")
        ),
        caption(
            "The senior crew takes Cutter One through ordinary work: a repaired manifold, three recoveries, and one orbit nobody assigned."
        )
    )
);
