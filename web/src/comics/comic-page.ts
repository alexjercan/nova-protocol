export type ComicTone = "default" | "amber" | "danger" | "muted";
export type PanelSize = "half" | "wide";
export type PanelVariant =
    | "default"
    | "work"
    | "orbit"
    | "impact"
    | "silent"
    | "recorders"
    | "contacts";

export type SvgNode =
    | {
          kind: "circle";
          center: [number, number];
          radius: number;
          tone: ComicTone;
          fill: boolean;
      }
    | {
          kind: "line";
          from: [number, number];
          to: [number, number];
          tone: ComicTone;
          width: number;
      }
    | {
          kind: "svgText";
          at: [number, number];
          text: string;
          tone: ComicTone;
      };

export type ComicNode =
    | {
          kind: "header";
          number: string;
          title: string;
          subtitle: string;
      }
    | { kind: "grid"; children: ComicNode[] }
    | {
          kind: "panel";
          size: PanelSize;
          variant: PanelVariant;
          label?: string;
          children: ComicNode[];
      }
    | { kind: "svgAsset"; source: string; alt: string; focus?: string }
    | { kind: "speech"; speaker: string; text: string; tone: ComicTone }
    | { kind: "caption"; text: string; tone: ComicTone }
    | { kind: "readout"; lines: string[]; tone: ComicTone }
    | {
          kind: "svg";
          viewBox: [number, number, number, number];
          label: string;
          children: SvgNode[];
      };

export type ComicPage =
    | { layout: "standard"; children: ComicNode[] }
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

export function comicPage(...children: ComicNode[]): ComicPage {
    return { layout: "standard", children };
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

export function chapterHeader(options: {
    number: string;
    title: string;
    subtitle: string;
}): ComicNode {
    return { kind: "header", ...options };
}

export function grid(...children: ComicNode[]): ComicNode {
    return { kind: "grid", children };
}

export function panel(
    options: {
        size?: PanelSize;
        variant?: PanelVariant;
        label?: string;
    },
    ...children: ComicNode[]
): ComicNode {
    return {
        kind: "panel",
        size: options.size ?? "half",
        variant: options.variant ?? "default",
        label: options.label,
        children,
    };
}

export function svgAsset(
    source: string,
    options: { alt: string; focus?: string }
): ComicNode {
    return { kind: "svgAsset", source, ...options };
}

export function speech(
    speaker: string,
    text: string,
    tone: ComicTone = "default"
): ComicNode {
    return { kind: "speech", speaker, text, tone };
}

export function caption(text: string, tone: ComicTone = "default"): ComicNode {
    return { kind: "caption", text, tone };
}

export function readout(lines: string[], tone: ComicTone = "muted"): ComicNode {
    return { kind: "readout", lines, tone };
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
    center: [number, number];
    radius: number;
    tone?: ComicTone;
    fill?: boolean;
}): SvgNode {
    return {
        kind: "circle",
        center: options.center,
        radius: options.radius,
        tone: options.tone ?? "default",
        fill: options.fill ?? false,
    };
}

export function line(options: {
    from: [number, number];
    to: [number, number];
    tone?: ComicTone;
    width?: number;
}): SvgNode {
    return {
        kind: "line",
        from: options.from,
        to: options.to,
        tone: options.tone ?? "default",
        width: options.width ?? 2,
    };
}

export function svgText(
    text: string,
    options: { at: [number, number]; tone?: ComicTone }
): SvgNode {
    return {
        kind: "svgText",
        at: options.at,
        text,
        tone: options.tone ?? "default",
    };
}
