import type { MouseEvent as ReactMouseEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

type ResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

const EDGES: { dir: ResizeDirection; className: string }[] = [
  { dir: "North", className: "resize-edge north" },
  { dir: "South", className: "resize-edge south" },
  { dir: "East", className: "resize-edge east" },
  { dir: "West", className: "resize-edge west" },
  { dir: "NorthEast", className: "resize-edge corner ne" },
  { dir: "NorthWest", className: "resize-edge corner nw" },
  { dir: "SouthEast", className: "resize-edge corner se" },
  { dir: "SouthWest", className: "resize-edge corner sw" },
];

type WindowResizeHandlesProps = {
  enabled: boolean;
};

export function WindowResizeHandles({ enabled }: WindowResizeHandlesProps) {
  if (!enabled) return null;

  const beginResize =
    (direction: ResizeDirection) => (event: ReactMouseEvent) => {
      if (event.button !== 0) return;
      event.preventDefault();
      event.stopPropagation();
      void getCurrentWindow()
        .startResizeDragging(direction)
        .catch((error) => {
          console.error("Failed to start window resize", error);
        });
    };

  return (
    <div className="resize-handles" aria-hidden="true">
      {EDGES.map((edge) => (
        <div
          key={edge.dir}
          className={edge.className}
          data-tauri-drag-region="false"
          onMouseDown={beginResize(edge.dir)}
        />
      ))}
    </div>
  );
}
