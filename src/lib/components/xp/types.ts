// Tipos compartidos de los componentes XP.

export type IconName = "library" | "folder" | "playlist" | "note" | "heart" | "disc" | "person" | "clock" | "home" | "search" | "radio";

export type Column<T> = {
  key: string;
  label: string;
  /// Fracción (`2fr`) o píxeles (`60px`).
  width: string;
  align?: "left" | "right" | "center";
  text: (row: T, index: number) => string;
  /// Valor para ordenar (si falta, se ordena por `text`).
  sort?: (row: T) => string | number;
};

export type TreeNode = {
  id: string;
  label: string;
  icon: IconName;
  children?: TreeNode[];
  onselect?: () => void;
  onmenu?: (e: MouseEvent) => void;
};
