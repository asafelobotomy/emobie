import { useEffect, useState } from "react";

export type LayoutMode = "horizontal" | "vertical" | "square";
export type ScrollAxis = "x" | "y";

export type LayoutInfo = {
  mode: LayoutMode;
  scrollAxis: ScrollAxis;
  compact: boolean;
};

/** Below this content height, chrome collapses so 2 emoji rows stay visible. */
const COMPACT_HEIGHT = 300;

function layoutFromSize(width: number, height: number): LayoutInfo {
  const ratio = width / Math.max(height, 1);
  const mode: LayoutMode =
    ratio >= 1.35 ? "horizontal" : ratio <= 0.75 ? "vertical" : "square";
  const scrollAxis: ScrollAxis = width >= height ? "x" : "y";
  const compact = height < COMPACT_HEIGHT || width < 280;
  return { mode, scrollAxis, compact };
}

export function useLayoutMode(root: HTMLElement | null): LayoutInfo {
  const [layout, setLayout] = useState<LayoutInfo>({
    mode: "square",
    scrollAxis: "y",
    compact: false,
  });

  useEffect(() => {
    if (!root) return;

    const update = () => {
      setLayout(layoutFromSize(root.clientWidth, root.clientHeight));
    };

    update();
    const observer = new ResizeObserver(update);
    observer.observe(root);
    return () => observer.disconnect();
  }, [root]);

  return layout;
}
