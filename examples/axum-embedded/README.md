# axum-embedded

This example uses Vite in debug builds and compiles the complete production
frontend into the release executable with `inertia-embed`.

From the repository root:

```bash
pnpm --dir examples/axum-embedded/frontend install --frozen-lockfile
pnpm --dir examples/axum-embedded/frontend build
cargo run --release -p axum-embedded
```

The frontend build must exist while Cargo compiles the release executable. It
is not needed when that executable runs. Verify this invariant with:

```bash
mise run ci:embedded
```
