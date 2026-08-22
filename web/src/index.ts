import "./style.css";
import { initLandingHandoff, initSite } from "./site";
import { warnIfNoWebGpu } from "./webgpu";
import { enhanceDownloadButtons } from "./downloads";

initSite();
initLandingHandoff();
warnIfNoWebGpu();
void enhanceDownloadButtons();
