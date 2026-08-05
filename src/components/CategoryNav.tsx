import type { Category } from "../data/loadEmojis";

type CategoryNavProps = {
  categories: Category[];
  activeId: number;
  onSelect: (id: number) => void;
};

export function CategoryNav({ categories, activeId, onSelect }: CategoryNavProps) {
  return (
    <nav className="category-nav" aria-label="Emoji categories" role="tablist">
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
