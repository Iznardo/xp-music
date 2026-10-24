# vendor/

Local patched copies of dependencies.

## ytmapi-rs 0.3.3 (MIT)
Original: https://crates.io/crates/ytmapi-rs

Patch (`src/auth/browser.rs`): `auth_user` field + `BrowserToken::with_auth_user(n)`, which sends the `X-Goog-AuthUser` header. Without it, with several Google accounts in the same session, the library always uses the first one (index 0).

To do: propose the change upstream and remove this copy once it is released.

Another change: `#![allow(deprecated, unused_imports, clippy::all)]` in `src/lib.rs`, because warnings are not downgraded for path dependencies and they would break the app's `cargo clippy -D warnings`.
