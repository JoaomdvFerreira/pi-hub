import { lazy } from "react";

/** Code-split: xterm.js is only pulled into the bundle once a terminal is actually opened. */
export const LazyTerminalModal = lazy(() =>
  import("./TerminalModal").then((module) => ({ default: module.TerminalModal })),
);
