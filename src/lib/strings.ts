// Todos los textos de la interfaz (constitución: UI en español, en un único módulo).

export const t = {
  appName: "XP Music",
  window: {
    minimize: "Minimizar",
    maximize: "Maximizar",
    restore: "Restaurar",
    close: "Cerrar",
  },
  menu: {
    file: "Archivo",
    view: "Ver",
    play: "Reproducir",
    tools: "Herramientas",
    help: "Ayuda",
  },
  themes: {
    title: "Combinación de colores",
    Blue: "Azul (predeterminado)",
    Olive: "Verde oliva",
    Silver: "Plateado",
  },
  status: {
    ready: "Listo",
    saving: "Guardando…",
    loadError: "No se pudieron cargar los ajustes.",
  },
  placeholder: "La biblioteca y el reproductor llegarán en las fases 002–004.",
} as const;
