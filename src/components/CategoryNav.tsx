import { useEffect, useRef } from "react";
import type { Category } from "../data/loadEmojis";

type CategoryNavProps = {
  categories: Category[];
  activeId: number;
  onSelect: (id: number) => void;
};

export function CategoryNav({ categories, activeId, onSelect }: CategoryNavProps) {
  const navRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const nav = navRef.current;
    if (!nav) return;

    const onWheel = (event: WheelEvent) => {
      if (event.ctrlKey) return;

      const styles = getComputedStyle(nav);
      const horizontal = styles.flexDirection === "row";
      const delta = event.deltaY !== 0 ? event.deltaY : event.deltaX;
      if (delta === 0) return;

      if (horizontal) {
        if (nav.scrollWidth <= nav.clientWidth) return;
        event.preventDefault();
        nav.scrollLeft += delta;
        return;
      }

      if (nav.scrollHeight <= nav.clientHeight) return;
      if (event.deltaX !== 0 && event.deltaY === 0) {
        event.preventDefault();
        nav.scrollTop += event.deltaX;
      }
    };

    nav.addEventListener("wheel", onWheel, { passive: false });
    return () => nav.removeEventListener("wheel", onWheel);
  }, []);

  return (
    <nav
      ref={navRef}
      className="category-nav"
      aria-label="Emoji categories"
      role="tablist"
    >
      {categories.map((category) => {
        const selected = category.id === activeId;
        return (
          <button
            key={category.key}
            type="button"
            role="tab"
            className="category-btn"
            title={category.label}
            aria-label={category.label}
            aria-selected={selected}
            tabIndex={selected ? 0 : -1}
            onClick={() => onSelect(category.id)}
          >
            {category.icon}
          </button>
        );
      })}
    </nav>
  );
}
