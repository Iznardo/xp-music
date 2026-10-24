// Types shared by the XP components.

export type IconName = "library" | "folder" | "playlist" | "note" | "heart" | "disc" | "person" | "clock" | "home" | "search" | "radio";

export type Column<T> = {
  key: string;
  label: string;
  /// Fraction (`2fr`) or pixels (`60px`).
  width: string;
  align?: "left" | "right" | "center";
  text: (row: T, index: number) => string;
  /// Sort value (if missing, sorts by `text`).
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
