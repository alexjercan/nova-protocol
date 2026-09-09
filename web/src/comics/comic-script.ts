/** Author-facing story language. Dialogue is independent of art and placement. */
export type XY = [number, number];
/** One spoken line, identified independently of its speaker and balloon position. */
export interface Dialogue {
    id: string;
    speaker: string;
    text: string;
}
/** Panel-local lettering. Empty breakAfter uses automatic measured wrapping. */
export interface Balloon {
    at: XY;
    width: number;
    tail: XY;
    side: "top" | "bottom" | "left" | "right";
    breakAfter: number[];
}
/** Text placed in a named blank slot in the scene's painter order. */
export interface SceneLabel {
    id: string;
    text: string;
    slot: string;
    at: XY;
    size: number;
    color: string;
    weight: "normal" | "bold";
    spacing: number;
    anchor: "start" | "middle" | "end";
}
/** An authored location/time card, drawn by the shared compositor. */
export interface LocationCard {
    id: string;
    name: string;
    place: string;
    date: string;
    note: string;
    at: XY;
    width: number;
}
/** Story content retained even before a panel is illustrated. */
export interface ScriptPanel {
    id: string;
    action: string;
    dialogue: Dialogue[];
}
/** A panel's story, art reference, and separately addressable lettering. */
export interface Panel extends ScriptPanel {
    art: string;
    lettering: Record<string, Balloon>;
    labels: SceneLabel[];
    cards: LocationCard[];
}
/** Page placement, independent of the panel's own illustration coordinates. */
export interface Placement {
    at: XY;
    size: XY;
}
/** An illustrated page; its layout must account for every panel exactly once. */
export interface IllustratedPage {
    kind: "illustrated";
    id: string;
    title: string;
    purpose: string;
    panels: Panel[];
    layout: Record<string, Placement>;
}
/** A complete script page with no invented art references or placeholder geometry. */
export interface ScriptPage {
    kind: "script";
    id: string;
    title: string;
    purpose: string;
    panels: ScriptPanel[];
}
/** One maintained episode script in reading order. */
export interface EpisodeScript {
    heading: string;
    footer: string;
    pages: (IllustratedPage | ScriptPage)[];
}

/** Name a scene registered by an art module. Unknown names fail the build. */
export function scene(id: string): string {
    return id;
}
/** Write dialogue once, as a searchable whole sentence or paragraph. */
export function say(id: string, speaker: string, text: string): Dialogue {
    return { id, speaker, text };
}
/** Defaults mean a bottom tail and automatic wrapping; overrides retain deliberate breaks. */
export function balloon(
    options: Pick<Balloon, "at" | "width" | "tail"> &
        Partial<Pick<Balloon, "side" | "breakAfter">>
): Balloon {
    return { side: "bottom", breakAfter: [], ...options };
}
/** Default label styling is normal weight, unspaced, start-anchored text. */
export function label(
    id: string,
    text: string,
    options: Omit<SceneLabel, "id" | "text" | "weight" | "spacing" | "anchor"> &
        Partial<Pick<SceneLabel, "weight" | "spacing" | "anchor">>
): SceneLabel {
    return {
        id,
        text,
        weight: "normal",
        spacing: 0,
        anchor: "start",
        ...options,
    };
}
/** Place a location card without maintaining its drawing implementation in a story. */
export function location(
    id: string,
    options: Omit<LocationCard, "id">
): LocationCard {
    return { id, ...options };
}
/** Empty label/card arrays mean there are no such story items in this panel. */
export function panel(
    id: string,
    options: Omit<Panel, "id" | "labels" | "cards"> &
        Partial<Pick<Panel, "labels" | "cards">>
): Panel {
    return { id, labels: [], cards: [], ...options };
}
/** Keep action and dialogue available before art exists. */
export function scriptPanel(
    id: string,
    action: string,
    dialogue: Dialogue[]
): ScriptPanel {
    return { id, action, dialogue };
}
/** Declare the story and layout of an illustrated page. */
export function page(options: Omit<IllustratedPage, "kind">): IllustratedPage {
    return { kind: "illustrated", ...options };
}
/** Declare an unillustrated page; it cannot enter a released reader. */
export function scriptPage(options: Omit<ScriptPage, "kind">): ScriptPage {
    return { kind: "script", ...options };
}
/** Assemble the ordered page scripts without an episode-specific export wrapper. */
export function episode(options: EpisodeScript): EpisodeScript {
    return options;
}

/** A selected panel with generated art metadata, ready for the shared compositor. */
export interface CompiledPanel extends Omit<Panel, "art"> {
    image: string;
    sceneSize: XY;
    placement: Placement;
}
/** Full-color panels composed in the browser, not precomposed by Python. */
export interface PanelPage {
    layout: "panels";
    title: string;
    alt: string;
    number: number;
    width: number;
    height: number;
    heading: string;
    footer: string;
    palette: Record<string, string>;
    panels: CompiledPanel[];
}
