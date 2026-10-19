// Acción `use:focusOnMount`: da el foco al montar (alternativa accesible a `autofocus`
// para diálogos modales, donde el foco inicial es lo esperado).
export function focusOnMount(node: HTMLElement) {
  queueMicrotask(() => node.focus());
}
