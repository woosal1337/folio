// Hand-written companion to `src-tauri/src/commands/auth.rs::AuthStatus`.
// Too small to be worth wiring through ts-rs.

import type { UserIdentity } from "./UserIdentity";

export interface AuthStatus {
  signed_in: boolean;
  identity: UserIdentity | null;
}
