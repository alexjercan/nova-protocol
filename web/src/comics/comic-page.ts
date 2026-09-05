/**
 * Typed page language for HUD comics.
 *
 * Page modules build data with these helpers only. Class names, markup and
 * attribute strings live in the renderer, so a page module cannot inject
 * anything into the document.
 */

export type ComicTone = "default" | "amber" | "danger" | "muted" | "blue";
/** Where a chapter stands: playable in the game, planned, or comic frame matter. */
export type ChapterState = "playable" | "planned" | "frame";
export type Corner = "bottom-right" | "bottom-left" | "top-left" | "top-right";
export type Channel = "open" | "cabin" | "rec" | "feed" | "guard";
export type Focus = "left" | "center" | "right";
/** How a panel is dressed. "crt" draws the panel as a phosphor screen in a bezel. */
export type PanelVariant =
    | "default"
    | "crt"
    | "work"
    | "orbit"
    | "impact"
    | "silent"
    | "recorders"
    | "contacts"
    | "dark";
export type TextAnchor = "start" | "middle" | "end";
export type Point = [number, number];
/** A grid row: a flex weight, or "auto" to size the row to its content. */
export type GridRow = number | "auto";

export type SvgNode =
    | {
          kind: "circle";
          center: Point;
          radius: number;
          tone: ComicTone;
          fill: boolean;
          width: number;
      }
    | {
          kind: "ellipse";
          center: Point;
          radii: Point;
          tone: ComicTone;
          fill: boolean;
          width: number;
      }
    | {
          kind: "rect";
          at: Point;
          size: Point;
          tone: ComicTone;
          fill: boolean;
          width: number;
          radius: number;
      }
    | {
          kind: "line";
          from: Point;
          to: Point;
          tone: ComicTone;
          width: number;
          dash?: [number, number];
      }
    | {
          kind: "polyline";
          points: Point[];
          tone: ComicTone;
          width: number;
          closed: boolean;
          fill: boolean;
          dash?: [number, number];
      }
    | {
          kind: "path";
          d: string;
          tone: ComicTone;
          width: number;
          fill: boolean;
          dash?: [number, number];
      }
    | {
          kind: "svgText";
          at: Point;
          text: string;
          tone: ComicTone;
          size: number;
          anchor: TextAnchor;
          spacing: number;
      }
    | {
          kind: "group";
          translate: Point;
          rotate: number;
          scale: number;
          opacity: number;
          children: SvgNode[];
      };

export interface TranscriptLine {
    speaker: string;
    text: string;
    tone: ComicTone;
}

export interface HudItem {
    label: string;
    value: string;
    tone: ComicTone;
}

export interface TerminalOption {
    key: string;
    label: string;
    detail: string;
    tone: ComicTone;
}

export type ComicNode =
    | {
          kind: "header";
          number: string;
          title: string;
          subtitle: string;
          eyebrow?: string;
      }
    | { kind: "grid"; columns: number; rows: GridRow[]; children: ComicNode[] }
    | {
          kind: "panel";
          span: number;
          rowSpan: number;
          variant: PanelVariant;
          frame: ComicTone;
          label?: string;
          children: ComicNode[];
      }
    | { kind: "svgAsset"; source: string; alt: string; focus: Focus }
    | {
          kind: "speech";
          speaker: string;
          text: string;
          tone: ComicTone;
          at: Corner;
          channel?: Channel;
      }
    | { kind: "narration"; text: string; tone: ComicTone; at: Corner }
    | { kind: "locationCard"; place: string; time: string; at: Corner }
    | { kind: "caption"; text: string; tone: ComicTone }
    | { kind: "readout"; lines: string[]; tone: ComicTone }
    | { kind: "transcript"; title?: string; lines: TranscriptLine[] }
    | {
          kind: "portrait";
          source: string;
          alt: string;
          name: string;
          role: string;
          tone: ComicTone;
      }
    | { kind: "divider"; text: string; tone: ComicTone }
    | { kind: "hud"; items: HudItem[]; at: Corner }
    | { kind: "feed"; source: string; lines: string[] }
    | { kind: "terminal"; prompt: string; options: TerminalOption[] }
    | { kind: "inset"; at: Corner; children: ComicNode[] }
    | {
          kind: "svg";
          viewBox: [number, number, number, number];
          label: string;
          children: SvgNode[];
      };

export type ComicPage =
    | { layout: "standard"; children: ComicNode[] }
    | {
          layout: "bleed";
          image: string;
          alt: string;
          focus: Focus;
          children: ComicNode[];
      }
    | {
          layout: "cover";
          image: string;
          alt: string;
          eyebrow: string;
          title: string;
          accent: string;
          tagline: string[];
      }
    | {
          layout: "end";
          eyebrow: string;
          title: string;
          body: string;
          action?: { label: string; href: string };
      };

/** A page with a header and a panel grid. */
export function comicPage(...children: ComicNode[]): ComicPage {
    return { layout: "standard", children };
}

/** A page filled by one image, with overlays placed at its corners. */
export function bleedPage(
    options: { image: string; alt: string; focus?: Focus },
    ...children: ComicNode[]
): ComicPage {
    return {
        layout: "bleed",
        image: options.image,
        alt: options.alt,
        focus: options.focus ?? "center",
        children,
    };
}

export function coverPage(
    options: Omit<Extract<ComicPage, { layout: "cover" }>, "layout">
): ComicPage {
    return { layout: "cover", ...options };
}

export function endPage(
    options: Omit<Extract<ComicPage, { layout: "end" }>, "layout">
): ComicPage {
    return { layout: "end", ...options };
}

/** A page header. The eyebrow reads "Chapter <number>" unless given. */
export function chapterHeader(options: {
    number: string;
    title: string;
    subtitle: string;
    eyebrow?: string;
}): ComicNode {
    return { kind: "header", ...options };
}

/**
 * A panel grid. `columns` is the number of tracks; `rows` lists the weight
 * of each explicit row, or "auto" for a row that takes its content's height,
 * which suits a transcript, feed or terminal. Captions and dividers take auto
 * rows after them.
 */
export function grid(
    options: { columns?: number; rows?: GridRow[] },
    ...children: ComicNode[]
): ComicNode {
    return {
        kind: "grid",
        columns: options.columns ?? 2,
        rows: options.rows ?? [2, 1],
        children,
    };
}

export function panel(
    options: {
        span?: number;
        rowSpan?: number;
        variant?: PanelVariant;
        frame?: ComicTone;
        label?: string;
    },
    ...children: ComicNode[]
): ComicNode {
    return {
        kind: "panel",
        span: options.span ?? 1,
        rowSpan: options.rowSpan ?? 1,
        variant: options.variant ?? "default",
        frame: options.frame ?? "default",
        label: options.label,
        children,
    };
}

/** A panel that spans every column of its grid. */
export function widePanel(
    options: {
        variant?: PanelVariant;
        frame?: ComicTone;
        label?: string;
        rowSpan?: number;
    },
    ...children: ComicNode[]
): ComicNode {
    return panel({ ...options, span: Number.POSITIVE_INFINITY }, ...children);
}

export function svgAsset(
    source: string,
    options: { alt: string; focus?: Focus }
): ComicNode {
    return {
        kind: "svgAsset",
        source,
        alt: options.alt,
        focus: options.focus ?? "center",
    };
}

/** A speech box. The speaker label is printed on the box. */
export function speech(
    speaker: string,
    text: string,
    options: { tone?: ComicTone; at?: Corner; channel?: Channel } = {}
): ComicNode {
    return {
        kind: "speech",
        speaker,
        text,
        tone: options.tone ?? "default",
        at: options.at ?? "bottom-right",
        channel: options.channel,
    };
}

/** A narration box inside a panel. */
export function narration(
    text: string,
    options: { tone?: ComicTone; at?: Corner } = {}
): ComicNode {
    return {
        kind: "narration",
        text,
        tone: options.tone ?? "default",
        at: options.at ?? "top-left",
    };
}

/** A location and time card inside a panel. */
export function locationCard(
    place: string,
    time: string,
    options: { at?: Corner } = {}
): ComicNode {
    return { kind: "locationCard", place, time, at: options.at ?? "top-left" };
}

/** A caption bar that spans the grid below its panels. */
export function caption(text: string, tone: ComicTone = "default"): ComicNode {
    return { kind: "caption", text, tone };
}

export function readout(lines: string[], tone: ComicTone = "muted"): ComicNode {
    return { kind: "readout", lines, tone };
}

/** A radio transcript filling a panel: several speakers in order. */
export function transcript(
    lines: [speaker: string, text: string, tone?: ComicTone][],
    options: { title?: string } = {}
): ComicNode {
    return {
        kind: "transcript",
        title: options.title,
        lines: lines.map(([speaker, text, tone]) => ({
            speaker,
            text,
            tone: tone ?? "default",
        })),
    };
}

/** A portrait card: image, name and role. */
export function portrait(options: {
    source: string;
    alt: string;
    name: string;
    role: string;
    tone?: ComicTone;
}): ComicNode {
    return { kind: "portrait", tone: "default", ...options };
}

/** A transition band that spans the grid: "one hour later". */
export function divider(text: string, tone: ComicTone = "amber"): ComicNode {
    return { kind: "divider", text, tone };
}

/** HUD chips at a panel corner: label and value pairs. */
export function hud(
    items: [label: string, value: string, tone?: ComicTone][],
    options: { at?: Corner } = {}
): ComicNode {
    return {
        kind: "hud",
        at: options.at ?? "top-right",
        items: items.map(([label, value, tone]) => ({
            label,
            value,
            tone: tone ?? "default",
        })),
    };
}

/** A public feed or bulletin block filling a panel. */
export function feed(source: string, lines: string[]): ComicNode {
    return { kind: "feed", source, lines };
}

/** A terminal choice filling a panel. */
export function terminal(
    prompt: string,
    options: [key: string, label: string, detail: string, tone?: ComicTone][]
): ComicNode {
    return {
        kind: "terminal",
        prompt,
        options: options.map(([key, label, detail, tone]) => ({
            key,
            label,
            detail,
            tone: tone ?? "default",
        })),
    };
}

/** A small box at a panel corner holding any nodes, such as a mini map. */
export function inset(
    options: { at?: Corner },
    ...children: ComicNode[]
): ComicNode {
    return { kind: "inset", at: options.at ?? "top-right", children };
}

export function svg(
    options: {
        viewBox: [number, number, number, number];
        label: string;
    },
    ...children: SvgNode[]
): ComicNode {
    return { kind: "svg", ...options, children };
}

export function circle(options: {
    center: Point;
    radius: number;
    tone?: ComicTone;
    fill?: boolean;
    width?: number;
}): SvgNode {
    return {
        kind: "circle",
        center: options.center,
        radius: options.radius,
        tone: options.tone ?? "default",
        fill: options.fill ?? false,
        width: options.width ?? 2,
    };
}

export function ellipse(options: {
    center: Point;
    radii: Point;
    tone?: ComicTone;
    fill?: boolean;
    width?: number;
}): SvgNode {
    return {
        kind: "ellipse",
        center: options.center,
        radii: options.radii,
        tone: options.tone ?? "default",
        fill: options.fill ?? false,
        width: options.width ?? 2,
    };
}

export function rect(options: {
    at: Point;
    size: Point;
    tone?: ComicTone;
    fill?: boolean;
    width?: number;
    radius?: number;
}): SvgNode {
    return {
        kind: "rect",
        at: options.at,
        size: options.size,
        tone: options.tone ?? "default",
        fill: options.fill ?? false,
        width: options.width ?? 2,
        radius: options.radius ?? 0,
    };
}

export function line(options: {
    from: Point;
    to: Point;
    tone?: ComicTone;
    width?: number;
    dash?: [number, number];
}): SvgNode {
    return {
        kind: "line",
        from: options.from,
        to: options.to,
        tone: options.tone ?? "default",
        width: options.width ?? 2,
        dash: options.dash,
    };
}

export function polyline(options: {
    points: Point[];
    tone?: ComicTone;
    width?: number;
    closed?: boolean;
    fill?: boolean;
    dash?: [number, number];
}): SvgNode {
    return {
        kind: "polyline",
        points: options.points,
        tone: options.tone ?? "default",
        width: options.width ?? 2,
        closed: options.closed ?? false,
        fill: options.fill ?? false,
        dash: options.dash,
    };
}

/** A path from SVG path data. The renderer rejects anything but path commands and numbers. */
export function path(options: {
    d: string;
    tone?: ComicTone;
    width?: number;
    fill?: boolean;
    dash?: [number, number];
}): SvgNode {
    return {
        kind: "path",
        d: options.d,
        tone: options.tone ?? "default",
        width: options.width ?? 2,
        fill: options.fill ?? false,
        dash: options.dash,
    };
}

export function svgText(
    text: string,
    options: {
        at: Point;
        tone?: ComicTone;
        size?: number;
        anchor?: TextAnchor;
        spacing?: number;
    }
): SvgNode {
    return {
        kind: "svgText",
        at: options.at,
        text,
        tone: options.tone ?? "default",
        size: options.size ?? 14,
        anchor: options.anchor ?? "middle",
        spacing: options.spacing ?? 0,
    };
}

/** A transformed group. Transforms are numbers, never strings. */
export function group(
    options: {
        translate?: Point;
        rotate?: number;
        scale?: number;
        opacity?: number;
    },
    ...children: SvgNode[]
): SvgNode {
    return {
        kind: "group",
        translate: options.translate ?? [0, 0],
        rotate: options.rotate ?? 0,
        scale: options.scale ?? 1,
        opacity: options.opacity ?? 1,
        children,
    };
}
