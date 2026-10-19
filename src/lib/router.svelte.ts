import { commands } from "./bindings";

// Router mínimo por hash (sin dependencias). Rutas:
//   #/now                     Reproducción actual
//   #/home                    Inicio (recomendaciones)
//   #/library/<sección>       Biblioteca: playlists | songs | liked | albums | artists | history
//   #/playlist/<id>  #/album/<id>  #/artist/<id>
//   #/search/<filtro>/<texto>

export type Route =
  | { name: "now" }
  | { name: "home" }
  | { name: "library"; section: LibrarySection }
  | { name: "playlist"; id: string }
  | { name: "album"; id: string }
  | { name: "artist"; id: string }
  | { name: "search"; filter: string; query: string };

export type LibrarySection = "playlists" | "songs" | "liked" | "albums" | "artists" | "history";
const SECTIONS: LibrarySection[] = ["playlists", "songs", "liked", "albums", "artists", "history"];

function parse(hash: string): Route {
  const [, name = "", a = "", ...rest] = hash.replace(/^#/, "").split("/");
  const dec = (s: string) => decodeURIComponent(s);
  switch (name) {
    case "now":
      return { name: "now" };
    case "home":
      return { name: "home" };
    case "library":
      return { name: "library", section: (SECTIONS as string[]).includes(a) ? (a as LibrarySection) : "playlists" };
    case "playlist":
    case "album":
    case "artist":
      return a ? { name, id: dec(a) } : { name: "library", section: "playlists" };
    case "search":
      return { name: "search", filter: a || "All", query: dec(rest.join("/")) };
    default:
      return { name: "library", section: "playlists" };
  }
}

export function href(r: Route): string {
  const enc = encodeURIComponent;
  switch (r.name) {
    case "library":
      return `#/library/${r.section}`;
    case "playlist":
    case "album":
    case "artist":
      return `#/${r.name}/${enc(r.id)}`;
    case "search":
      return `#/search/${r.filter}/${enc(r.query)}`;
    default:
      return `#/${r.name}`;
  }
}

class Router {
  route = $state<Route>(parse(location.hash));

  constructor() {
    addEventListener("hashchange", () => {
      this.route = parse(location.hash);
      // El backend la guarda para cuando la WebView se recree (AC-004-05).
      void commands.uiSetLastRoute(location.hash);
    });
  }

  /// Al arrancar sin ruta (WebView recreada), vuelve a la última vista.
  async restore(): Promise<void> {
    if (location.hash) return;
    const last = await commands.uiGetLastRoute();
    if (last) location.hash = last;
  }

  go(r: Route) {
    const h = href(r);
    if (location.hash !== h) {
      void commands.uiPlaySound("Navigate");
      location.hash = h;
    }
  }

  /// Pestaña de la barra de tareas a la que pertenece la ruta actual.
  get tab(): "now" | "library" | "home" | "search" {
    const n = this.route.name;
    if (n === "now" || n === "home" || n === "search") return n;
    return "library";
  }
}

export const router = new Router();
