import "./style.css";
import { initSite } from "./site";
import { warnIfNoWebGpu } from "./webgpu";

initSite();
warnIfNoWebGpu();
