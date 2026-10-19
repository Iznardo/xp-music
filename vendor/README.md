# vendor/

Copias locales parcheadas de dependencias.

## ytmapi-rs 0.3.3 (MIT)
Original: https://crates.io/crates/ytmapi-rs

Parche (`src/auth/browser.rs`): campo `auth_user` + `BrowserToken::with_auth_user(n)`, que envía la cabecera `X-Goog-AuthUser`. Sin esto, con varias cuentas de Google en la misma sesión, la librería siempre usa la primera (índice 0).

Pendiente: proponer el cambio upstream y eliminar esta copia cuando se publique.

Otro cambio: `#![allow(deprecated, unused_imports, clippy::all)]` en `src/lib.rs`, porque a las dependencias por ruta no se les rebajan los avisos y romperían `cargo clippy -D warnings` de la app.
