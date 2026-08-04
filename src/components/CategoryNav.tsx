import type { Category } from "../data/loadEmojis";

type CategoryNavProps = {
  categories: Category[];
  activeId: number;
  onSelect: (id: number) => void;
};

export function CategoryNav({ categories, activeId, onSelect }: CategoryNavProps) {
  return (
    <nav className="category-nav" aria-label="Emoji categories">
      {categories.map((category) => (
        <button
          key={category.key}
          type="button"
          className="category-btn"
          title={category.label}
          aria-label={category.label}
          aria-selected={category.id === activeId}
          onClick={() => onSelect(category.id)}
        >
          {category.icon}
        </button>
      ))}
    </nav>
  );
}
