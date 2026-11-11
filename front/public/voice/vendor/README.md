# Local WebRTC VAD

Vendored from `@ozymandiasthegreat/vad` **2.0.7**:
https://github.com/OzymandiasTheGreat/libfvad-wasm

Package: https://registry.npmjs.org/@ozymandiasthegreat/vad/-/vad-2.0.7.tgz

Verified npm SHA-512 integrity:
`a7MCJ/8ohhyjDFBAktVJsEA9tUwB7Le2+SpKrZp0sFhKyvdFyxZDI6G0PZHCxlFChHvyH+yK7PpKEdov7wEH8g==`

Only `lib/index.js`, `lib/vad.js`, `lib/math.js`, `dist/libfvad.wasm` and the
original LICENSE, PATENTS and AUTHORS are included. The upstream BSD license
and patent notice apply. These files are unmodified. The 15 KB WASM is loaded
from this site's origin when continuous voice is explicitly enabled. No CDN,
remote model download or browser speech recognition service is involved in VAD.
