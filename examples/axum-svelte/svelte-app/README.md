# Svelte Frontend

Vite frontend for the Axum Inertia example.

From the repository root:

```sh
pnpm --dir examples/axum-svelte/svelte-app install --frozen-lockfile
pnpm --dir examples/axum-svelte/svelte-app build
```

The production build writes assets and a Vite manifest to `../public/build`.
The Axum server reads that manifest to choose the script path and asset
version. The same build writes the managed Node bundle to `dist/ssr/app.js`.
Both outputs are required by the production example; Vite development needs
neither production artifact.
