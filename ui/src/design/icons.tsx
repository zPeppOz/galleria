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
};
