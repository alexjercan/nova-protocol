/**
 * Speakers and art of the Nova Protocol comic. Every page names its people
 * and its panels through these constants, so a rename lands in one place and
 * the build sees every asset the comic uses.
 */

export const CONTROL = "Meridian Control";
export const CALLOWAY = "Cmdr Calloway";
export const BOARD = "EWI Board";
export const PELL = "Pell";
export const BRANDT = "Brandt";
export const RUIZ = "Ruiz";
export const TAMM = "Tamm";

/** Panel art under `web/src/assets/story/nova-protocol/`, drawn by `art/comics/nova_protocol.py`. */
export const ART = {
    cover: "cover.svg",
    shelterWide: "shelter-wide.svg",
    meridianOnStation: "meridian-on-station.svg",
    surveyPlot: "survey-plot.svg",
    tenderAtTheCharge: "tender-at-the-charge.svg",
    chargeArmed: "charge-armed.svg",
    holdAtTheSite: "hold-at-the-site.svg",
    callowayBridge: "calloway-bridge.svg",
    theBlast: "the-blast.svg",
    boatDeck: "boat-deck.svg",
    pellRefuses: "pell-refuses.svg",
    brandtFitsTheBoat: "brandt-fits-the-boat.svg",
    tenderTowardTheShelter: "tender-toward-the-shelter.svg",
    signalLost: "signal-lost.svg",
    thePlaque: "the-plaque.svg",
} as const;
