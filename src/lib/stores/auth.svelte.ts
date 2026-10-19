// Estado de la sesión de YouTube Music (se hidrata al montar; `auth://changed` lo mantiene).

import { commands, events, type AuthStatus } from "../bindings";

class AuthStore {
  status = $state<AuthStatus | null>(null);

  async init(): Promise<void> {
    await events.authChanged.listen((e) => {
      this.status = e.payload;
    });
    this.status = await commands.authStatus();
  }

  get signedIn(): boolean {
    return typeof this.status === "object" && this.status !== null && !!this.status.SignedIn;
  }

  get accountName(): string | null {
    const s = this.status;
    if (s && typeof s === "object" && s.SignedIn) return s.SignedIn.name;
    if (s && typeof s === "object" && "Expired" in s) return s.Expired ?? null;
    return null;
  }
}

export const auth = new AuthStore();
