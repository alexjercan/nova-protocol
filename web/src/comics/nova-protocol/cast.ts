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
export const CAPTAIN = "Captain";
export const HALLORAN = "Halloran";
export const OKORO = "Okoro";
export const GUARD = "Guard channel";
export const VOICE = "Unknown voice";
export const BEACON = "Meridian beacon";

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
    junkSite: "junk-site.svg",
    boatBayShift: "boat-bay-shift.svg",
    cutterReleased: "cutter-released.svg",
    handlingCard: "handling-card.svg",
    cratesOnPlateSeven: "crates-on-plate-seven.svg",
    theDonut: "the-donut.svg",
    demirAtControl: "demir-at-control.svg",
    thirdCrateHome: "third-crate-home.svg",
    theReading: "the-reading.svg",
    outOfTheShadow: "out-of-the-shadow.svg",
    theStrike: "the-strike.svg",
    hidingInTheJunk: "hiding-in-the-junk.svg",
    halloran: "halloran.svg",
    okoro: "okoro.svg",
} as const;
