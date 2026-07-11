# Server-side rendering

## Quick start

Enable the `ssr` feature, configure a bundle, and use asynchronous startup:

```rust
let inertia = InertiaApp::vite("frontend")
    .ssr("dist/ssr/app.js")
    .start()
    .await?;
```

## Default SSR behavior

Once configured, every initial non-Inertia `GET` page is rendered by SSR unless route policy disables it. Inertia JSON visits and non-page responses never invoke SSR.

## Frontend Vite configuration

Install `@inertiajs/vite` and make the production build run both `vite build` and `vite build --ssr`. The plugin auto-detects `src/app.js`, `src/app.ts`, and conventional dedicated SSR entries. Setting `ssr.entry` is optional; use it only when auto-detection is unsuitable. The Svelte example intentionally uses one universal `src/app.js` entry.

## Client build versus SSR build

A production Inertia application normally has two outputs:

1. The client build, including `.vite/manifest.json`, browser JavaScript,
   stylesheets, and imported chunks.
2. The SSR bundle executed by Node.

The SSR protocol does not consume the Vite client manifest. However,
`InertiaApp::vite(...)` requires it in production so inertia-axum can inject
and serve the browser assets. Vite development mode does not require either
production output.

| Configuration | Client manifest | SSR bundle |
| --- | ---: | ---: |
| Vite production + managed Node | Required | Required |
| Vite production + external SSR | Required | Only required locally when `.bundle(...)` validation is enabled |
| Vite development server | Not required | Not required |
| Custom `AssetProvider` + managed Node | Not required | Required |
| `default_root()` + managed Node | Not required | Required |
| `default_root()` + external SSR | Not required | Not required locally |

## Managed Node mode

`Ssr::node(path)`, including the `.ssr(path)` shorthand, validates Node 22+, validates the bundle, starts one long-lived child without a shell, drains its output, supervises it, and shuts it down when the last application handle is dropped.

Managed SSR should bind to a loopback address unless external access is intentionally required.

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

The managed bundle path is resolved relative to the absolute Vite root. Absolute paths are preserved, and Node always receives an absolute bundle path. With the auto-detected `src/app.js` entry, the example emits `dist/ssr/app.js`; projects with another explicit input or output name must configure the corresponding path.

`Ssr::node(...).endpoint(...)` changes the URL used by Rust. It does not rewrite the port compiled into the official Inertia SSR bundle. When using a custom port, configure the same port in `inertia({ ssr: { port } })`.

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

A core SSR test using `default_root()` can render successfully without browser assets because it exercises the Inertia SSR protocol, not production Vite asset injection. Use `./scripts/test-live-ssr.sh` to clean-build and verify the client manifest, official SSR bundle, real example application, and static assets together.

## Performance and capacity guidance

Connections are pooled, renders have a timeout, bodies and concurrency are bounded, and saturation load-sheds immediately. Tune concurrency from production measurements. The adapter deliberately does not cache personalized SSR documents.
