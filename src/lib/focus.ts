// `use:focusOnMount` action: gives focus on mount (an accessible alternative to
// `autofocus` for modal dialogs, where initial focus is expected).
export function focusOnMount(node: HTMLElement) {
  queueMicrotask(() => node.focus());
}
