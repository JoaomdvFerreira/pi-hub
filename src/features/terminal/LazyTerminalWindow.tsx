import { lazy } from "react";

/** Code-split: xterm.js is only pulled into the bundle once a terminal is actually opened. */
export const LazyTerminalWindow = lazy(() =>
  import("./TerminalWindow").then((module) => ({ default: module.TerminalWindow })),
);
