// YouTube Music session state (hydrated on mount; kept up to date by `auth://changed`).

import { commands, events, type AuthStatus } from "../bindings";
import { clearResourceCache } from "../resource.svelte";

class AuthStore {
  status = $state<AuthStatus | null>(null);

  async init(): Promise<void> {
    await events.authChanged.listen((e) => {
      const before = this.accountName;
      this.status = e.payload;
      // A different account (or none): whatever was cached belonged to the previous one.
      if (this.accountName !== before || !this.signedIn) clearResourceCache();
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
