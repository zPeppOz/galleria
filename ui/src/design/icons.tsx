import type { ReactNode } from "react";

// Our own SVG icons (spec §12: no icon package, no text glyphs, no emoji). Decorative by default: the text or the
// accessible name of the control that holds an icon says what it is, so the icon is hidden from assistive technology.
// 16 × 16 grid, drawn with currentColor.
function icon(path: ReactNode): ReactNode {
  return (
    <svg className="gx-icon" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
      {path}
    </svg>
  );
}

export const icons = {
  close: icon(<path d="M4 4l8 8M12 4l-8 8" />),
  check: icon(<path d="M3.5 8.5l3 3 6-7" />),
  chevron: icon(<path d="M6 3.5l4.5 4.5L6 12.5" />),
  more: icon(<path d="M3.5 8h.01M8 8h.01M12.5 8h.01" strokeWidth="2.5" />),
  warning: icon(<path d="M8 2.5l6 10.5H2L8 2.5zM8 6.5v3M8 11.5h.01" />),
  error: icon(<path d="M8 2.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM8 5v3.5M8 10.75h.01" />),
  info: icon(<path d="M8 2.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM8 7.25v3.5M8 5.25h.01" />),
  star: icon(<path d="M8 2l1.9 3.9 4.3.6-3.1 3 .7 4.3L8 11.8l-3.8 2 .7-4.3-3.1-3 4.3-.6L8 2z" />),
  copy: icon(<path d="M5.5 5.5h7v8h-7zM3.5 10.5v-8h7" />),
};
