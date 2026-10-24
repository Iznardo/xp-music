import { commands } from "./bindings";

// Minimal hash router (no dependencies). Routes:
//   #/now                     Now playing
//   #/home                    Home (recommendations)
//   #/library/<section>       Library: playlists | songs | liked | albums | artists | history
//   #/playlist/<id>  #/album/<id>  #/artist/<id>
//   #/search/<filter>/<text>

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
      // The backend stores it for when the WebView is recreated (AC-004-05).
      void commands.uiSetLastRoute(location.hash);
    });
  }

  /// On startup with no route (WebView recreated), goes back to the last view.
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

  /// Taskbar tab the current route belongs to.
  get tab(): "now" | "library" | "home" | "search" {
    const n = this.route.name;
    if (n === "now" || n === "home" || n === "search") return n;
    return "library";
  }
}

export const router = new Router();
