// User visualizations (`visualizations/*.frag`), read by the backend.

import { commands, type CustomViz } from "../bindings";

class CustomVizStore {
  list = $state<CustomViz[]>([]);

  async refresh() {
    try {
      const r = await commands.vizCustomList();
      if (r.status === "ok") this.list = r.data;
    } catch (e) {
      void commands.uiLog("warn", `viz_custom_list: ${String(e)}`);
    }
  }

  find(name: string | null | undefined) {
    return name ? this.list.find((v) => v.name === name) : undefined;
  }
}

export const customViz = new CustomVizStore();
