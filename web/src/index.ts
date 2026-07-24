import "./style.css";
import { initSite } from "./site";
import { warnIfNoWebGpu } from "./webgpu";
import { enhanceDownloadButtons } from "./downloads";

initSite();
warnIfNoWebGpu();
void enhanceDownloadButtons();
