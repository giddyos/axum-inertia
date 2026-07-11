# Server-side rendering

## Quick start

Enable the `ssr` feature, configure a bundle, and use asynchronous startup:

```rust
let inertia = InertiaApp::vite("frontend")
    .ssr("dist/ssr/ssr.js")
    .start()
    .await?;
```

## Default SSR behavior

Once configured, every initial non-Inertia `GET` page is rendered by SSR unless route policy disables it. Inertia JSON visits and non-page responses never invoke SSR.

## Frontend Vite configuration

Install `@inertiajs/vite`, configure its `ssr.entry`, and make the production build run both `vite build` and `vite build --ssr`. See `examples/axum-svelte/svelte-app` for a complete Svelte setup.

## Managed Node mode

`Ssr::node(path)`, including the `.ssr(path)` shorthand, validates Node 22+, validates the bundle, starts one long-lived child without a shell, drains its output, supervises it, and shuts it down when the last application handle is dropped.

## External Node mode

Use `Ssr::external("http://node:13714")` for a separately supervised server. Add `.bundle(path)` for local validation or `.skip_bundle_check()` when Rust and Node use different containers.

## Opting routes out

Use `.without_ssr()` on a `MethodRouter` or `Router`. Disabled pages are ordinary CSR responses and do not affect health.

## Explicit opt-in mode

Use `Ssr::node(path).opt_in()` and mark selected routes or groups with `.with_ssr()`.

## Nested router behavior

The policy nearest the handler wins. A method route can override its nested group, and a nested group can override an outer router.

## Conditional SSR with `ssr_when`

`ssr_when` synchronously inspects the method, URI, headers, and request extensions:

```rust
get(account).ssr_when(|context| context.extension::<AuthUser>().is_none())
```

## Preparing async condition data through middleware

Perform database or network work in normal async Axum middleware, insert the result into request extensions, and read that extension from `ssr_when`.

## Development HMR

Set `VITE_DEV_SERVER_URL=http://127.0.0.1:5173`. SSR automatically uses `/__inertia_ssr`; a production bundle is not required, and Vite warmup `null` responses fall back to CSR.

## Production build output

The managed bundle path is resolved relative to the Vite root. Absolute paths are preserved. Ensure the SSR build emits `dist/ssr/ssr.js` or configure the emitted path explicitly.

## Graceful fallback

Fallback mode is the default. Overload, timeout, transport, malformed response, and backend render failures produce a usable CSR document with the same serialized page.

## Strict mode

Use `.strict()` when SSR failure must produce an internal error. Backend details and raw JavaScript errors are never returned to the client.

## Health reporting

`inertia.ssr_health()` returns `Disabled`, `Starting`, `Ready`, `Degraded`, or `Unavailable` from local state without a network probe.

## Node clustering

For multi-process Node clustering, run an external clustered server and configure `Ssr::external`. Rust’s concurrency limit should reflect the cluster’s measured capacity.

## Docker and process managers

Managed mode suits a single container containing Rust, Node, and the bundle. For separate containers, systemd, Kubernetes, or another process manager, use external mode and a private HTTP endpoint.

## Troubleshooting

Check Node is at least version 22, the resolved bundle is a file, the endpoint is absolute `http://`, `/health` becomes ready inside `startup_timeout`, and response sizes fit `max_response_bytes`.

## Testing

`inertia-axum-test::TestSsr` provides Node-free `/health`, `/render`, and `/shutdown` behavior, recorded calls, and `assert_ssr`, `assert_csr`, and head assertions. Keep real Node lifecycle coverage in a dedicated integration job.

## Performance and capacity guidance

Connections are pooled, renders have a timeout, bodies and concurrency are bounded, and saturation load-sheds immediately. Tune concurrency from production measurements. The adapter deliberately does not cache personalized SSR documents.
