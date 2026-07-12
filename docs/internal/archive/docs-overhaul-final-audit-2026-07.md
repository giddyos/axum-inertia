# Documentation overhaul final audit

This record audits the repository against
`specs/20260711-201348-inertia-axum-docs-overhaul/00-overview.md` and Phases
01 through 07. The audit completed on 2026-07-11.

## Result

The documentation overhaul is implemented and its automated, source, protocol,
fresh-project, SSR, and browser-hydration gates pass. The audit found and fixed
three final issues:

1. strict Clippy rejected four unnecessarily hashed raw strings in the CLI
   scaffold templates;
2. a fresh Svelte scaffold failed under pnpm 11 because it did not declare a
   build policy for the pinned `svelte-preprocess` install script;
3. generated `docs/next-env.d.ts` was still tracked despite being build output.

Visual-only documentation-site inspection could not be performed because the
in-app browser runtime reported no available browser backends. That limitation
is recorded below; link, metadata, type, static-generation, code-rendering, and
responsive-style contracts remain covered by automated checks and source
inspection.

## Full-spec traceability

### Overview, policy, and audit findings

| Requirement | Implementation evidence | Verification evidence | Status |
| --- | --- | --- | --- |
| Concise, example-first `1.0.0-alpha.1` guide with the existing getting-started, concepts, guides, SSR, frontend, recipe, advanced, reference, and migration architecture | 60 MDX routes under `docs/content/docs`; existing navigation hierarchy retained | `docs:check-links`, `docs:check-content`, Next static build | Pass |
| First application shows a normal route, immediate greeting, slow asynchronous deferred object, loading boundary, and resolved value in Svelte, React, and Vue | canonical server and three `docs/snippets/*/Home.*` files; quick start uses `Snippet` | snippet checks; fresh-project matrix; initial request ~2 ms, deferred request ~754 ms | Pass |
| Generated files are readable and synchronized with onboarding | multiline constants and complete snapshots in `cargo-inertia`; canonical Home equality assertion | CLI tests and snapshots; byte-for-byte fresh-project comparison | Pass |
| Getting-started duplication removed and each fact has a canonical page | routing index, installation, quick start, frontend workflow, and project-structure pages | content/source review, links and build | Pass |
| Deferred guide and dashboard recipe include maintained Svelte, React, and Vue syntax | frontend pages, props overview, deferred guide, deferred-dashboard recipe | framework-tab checker and snippet compilation | Pass |
| Scaffold React uses `Home.tsx`; maintained SSR example is explicitly JSX | quick-start paths, generator, CLI reference, React frontend/SSR copy | path checker, snapshots, source inspection | Pass |
| Workspace, MSRV, adapter, framework, and Vite versions cannot silently drift | `docs/scripts/check-content.mjs` reads workspace and maintained manifests | `docs:check-content` | Pass |
| No visible Rustdoc-only `# ` scaffolding in MDX | cleaned MDX examples; executable fixtures for long snippets | content checker across all 60 pages | Pass |
| Standalone Rust and frontend documentation snippets are executable | quick-start Cargo fixture and deterministic framework snippet checker | independent Cargo check and `snippets:check` | Pass |
| Public behavior follows maintained implementation and tests, not documentation-only APIs | links and examples trace to crate modules, examples, protocol tests, and helper crate | architecture verification and full Rust/SSR suites | Pass |
| Global task-page order, short prose, early code, readable titled blocks, domain names, Rust-outside-tabs rule, and no repeated stock headings | rewritten getting-started, guide, and recipe pages; content checker | source audit, content checker, lint, site build | Pass |
| Framework parity for Deferred, forms, InfiniteScroll, partial reloads, and shared/flash APIs | frontend guides, forms guide/recipe, infinite-scroll recipe, partial reload and flash pages | framework-tab checker and frontend snippet compilation | Pass |
| Exact Rust prerelease pins and single Node requirement | canonical manifests/reference; installation owns Node `22.12+` explanation | version/content checker | Pass |
| Stable routes and unique capabilities retained; exhaustive detail moved to reference | current route tree, reference pages, cross-links | 60-page link/anchor/navigation validation | Pass |

### Phase 01 — correctness infrastructure

| Requirement or acceptance criterion | Evidence | Status |
| --- | --- | --- |
| Exact canonical snippet tree: Rust Cargo/main plus Svelte, React TSX, and Vue Home files | `docs/snippets`; generated-artifact cleanup confirms only required source files remain | Pass |
| Rust fixture has exact pin/dependency set and independently checks | `docs/snippets/quick-start-server/Cargo.toml`; local crates.io patch used only for unpublished-package audit | Pass with publication limitation |
| Displayed quick-start source is exactly the checked file; missing source fails | `Snippet` component and quick-start imports | content check, snippet check, build | Pass |
| Content checker covers versions, MSRV, paths, navigation existence, old paths, hidden lines, tabs, and file titles | `docs/scripts/check-content.mjs` | `docs:check-content` | Pass |
| Complete generated-file and output snapshots; `.tsx`, Deferred, stats, formatting, and no-overwrite assertions | `crates/cargo-inertia/src/init.rs` and snapshot directory | 6 CLI tests pass; overwrite test included | Pass |
| Docs CI retains lint, types, links, content, snippets, build, Rust, frontend, SSR, live SSR, and hydration gates | `.github/workflows/ci.yaml` | workflow/source audit and local equivalents | Pass |

### Phase 02 — onboarding and scaffold

| Requirement or acceptance criterion | Evidence | Status |
| --- | --- | --- |
| Small README: description, core capabilities, one deferred server preview, primary links, stability notice | root `README.md` | source review, link check | Pass |
| Landing page has two-sentence explanation, four primary paths, example chooser, and bottom stability/version notice | `docs/content/docs/index.mdx` | content/build review | Pass |
| Getting-started index routes rather than duplicates; installation owns supported tools, canonical manifest, CLI, defaults, feature link | getting-started index and installation | content/link checks | Pass |
| Quick start follows create/add/generate/server/Home/install-build/run order and ends with only three requested next steps | `getting-started/quick-start.mdx` | source review/build | Pass |
| Slow `load_stats` is visibly simulated and deferred so initial load is not delayed | canonical `src/main.rs` uses a concise 750 ms sleep and comments | fresh HTTP timing matrix | Pass |
| Development/production workflow uses Vite URL, manifest, default entry, and custom-entry note once | `getting-started/frontend-setup.mdx` | fresh dev/prod matrix | Pass |
| Project structure is limited to tree, component-to-page mapping, manifest, and nested example | `getting-started/project-structure.mdx` | source review | Pass |
| Scaffold is readable, CSR-only, pinned, deferred, complete, non-overwriting, and prints concise next steps | CLI implementation and snapshots | fresh generation/build and CLI tests | Pass |
| Fresh pnpm 11 Svelte install is deterministic | generated `pnpm-workspace.yaml`; maintained Svelte policy | clean pnpm 11 install/build | Pass |

### Phase 03 — frontends and deferred data

| Requirement or acceptance criterion | Evidence | Status |
| --- | --- | --- |
| Frontend overview distinguishes minimal scaffold from maintained production/SSR examples and lists exact entry/page extensions | frontend index | content/path/version checks | Pass |
| Svelte, React, and Vue guides use maintained adapter APIs, equivalent Home data, and framework-specific syntax | three frontend pages and canonical snippets | deterministic snippet compilation | Pass |
| Props overview leads immediate → optional → deferred and sends exact policy detail to reference | concepts props page and props/policies reference | content review/links | Pass |
| Deferred guide includes server once, three client tabs, grouping/partial behavior, run step, notes, and one next step | deferred/lazy guide | content/tab/link checks | Pass |
| Deferred dashboard recipe is complete and behaviorally equivalent in all three adapters | recipe page | content/tab checks | Pass |

### Phase 04 — concepts, guides, and recipes

| Requirement or acceptance criterion | Evidence | Status |
| --- | --- | --- |
| Concept pages are compact mental models; task guides each lead one workflow rather than full API surveys | concepts and guides trees | prose/source review and build | Pass |
| Todo typed-page flow remains coherent from derive through test | typed pages and testing guides; `examples/todo` | source/API audit and Rust tests | Pass |
| Forms lead with custom validator, redirect validation, old input/errors, and equal adapter UIs | forms guide and validation recipe | parity checker/snippet build | Pass |
| Partial reload, flash/shared access, and InfiniteScroll show each meaningful adapter difference | relevant guides and paginated recipe | parity checker | Pass |
| Authentication, root-document, deployment, CLI, transient, testing, and merge/scroll/once workflows remain discoverable | retained routes and links | navigation/link check | Pass |
| Detailed matrices remain in reference; no hidden lines or compressed file examples remain | reference tree and content rules | content checker/lint/build | Pass |

### Phase 05 — SSR

| Requirement or acceptance criterion | Evidence | Status |
| --- | --- | --- |
| SSR index gives feature/build/start in three steps and reaches production setup within three pages | SSR index/build pages | source/link review | Pass |
| Managed and external ownership are explicit | managed-node and external-server pages | content review | Pass |
| Default-on, opt-out, opt-in, and conditional route policies each have an example | route-policies page | API/source check | Pass |
| Manifest, configured default-on, and fallback explanations each have one canonical location | SSR index/build/fallback pages and cross-links | structural content review | Pass |
| Fallback versus strict table and both builders remain | fallback-and-strict page | implementation/API audit | Pass |
| Complete health defaults/model retained without repeated prerequisites | health-and-operations page | implementation comparison | Pass |
| Fake `TestSsr`, live Node, and browser hydration tests are separated and linked | SSR testing page | test/link audit | Pass |
| Troubleshooting uses symptom/cause/fix sections | SSR troubleshooting page | source review | Pass |
| All SSR routes remain and runtime semantics match tests | nine SSR pages and reference configuration | live SSR, production SSR, Rust SSR, hydration suites | Pass |

### Phase 06 — advanced, reference, and migration

| Requirement or acceptance criterion | Evidence | Status |
| --- | --- | --- |
| Each advanced page starts with the common path; internals and trust boundaries follow | six advanced pages | source review | Pass |
| Error handling retains startup, request, form, transient, root, and SSR classes | advanced error page | API/error-source audit | Pass |
| Protocol page remains complete with tables and ordered processing | advanced protocol page | protocol tests/snapshots | Pass |
| Compatibility page starts with warning and immediate before/after migration while retaining exact legacy surface | compatibility page | source/API audit | Pass |
| Reference index, features, configuration, macros, policies, headers, protocol, testing, CLI, and examples remain exhaustive/searchable | ten reference pages | exact-name/content/link searches | Pass |
| Configuration retains constructors/builders/defaults/precedence/lifecycle; policies and testing retain complete matrices | corresponding reference pages | API and helper-crate comparison | Pass |
| CLI reference matches generated paths, contents, versions, output, overwrite behavior, and checker limitations | CLI reference, implementation, snapshots | CLI tests/content check | Pass |
| Migration retains build/page/shared/validation/SSR/test six-step order and exact pins | migration page | source/link/version review | Pass |

## Architecture and contract verification

| Contract | Canonical evidence inspected | Result |
| --- | --- | --- |
| Public names, signatures, and exports | `lib.rs`, `prelude.rs`, public modules, Rustdoc/doctests | Match |
| Cargo feature defaults and edges | `crates/inertia-axum/Cargo.toml`, feature reference | Match |
| Workspace version and MSRV | root `Cargo.toml`, content checker | `1.0.0-alpha.1`, Rust 1.88 |
| CLI files, versions, completion, overwrite behavior | `init.rs`, full snapshots, CLI reference/tests | Match |
| Svelte/React/Vue syntax | maintained example apps and canonical snippets | Match |
| Typed pages and test patterns | `examples/todo`, testing/helper APIs | Match |
| Advanced prop behavior | `examples/incident-board`, `examples/observatory`, policy reference | Match |
| Protocol semantics | `tests/protocol_v3` and snapshots, advanced/reference pages | Match |
| SSR startup/default/fallback/strict/policies/health/failures | SSR implementation, integration/live/browser suites | Match |
| Testing API | `inertia-axum-test` exports/tests and testing reference | Match |
| Routes/navigation/links/anchors/search metadata/code titles | MDX tree, navigation, content/link checks, Next build | Match |
| Canonical snippet inclusion and React `.tsx` distinction | `Snippet`, content checker, generator equality test | Match |
| Minimal CSR scaffold versus production examples | quick start, frontend overview, CLI and SSR references | Explicit |

## Validation record

The mandatory CI-equivalent checks passed:

- `pnpm --dir docs install --frozen-lockfile`
- `pnpm --dir docs lint`
- `pnpm --dir docs typecheck`
- `pnpm --dir docs docs:check-links` — 60 pages, links, metadata order,
  and anchors validated
- `pnpm --dir docs docs:check-content` — versions, navigation, paths,
  framework parity, snippet sources, and 60 MDX pages validated
- `pnpm --dir docs snippets:check` — Svelte, React, and Vue compiled
- `pnpm --dir docs build` — 188 static pages generated
- `cargo fmt --all -- --check`
- `cargo test --locked --workspace --all-targets` — 255 passed, 10 ignored
- `cargo test --locked --workspace --all-features` — 258 passed, 10 ignored
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`
- `cargo test --locked --workspace --doc` — 2 passed across nine suites
- `cargo check --locked --workspace --all-targets --all-features`
- independent quick-start Cargo check with only
  `patch.crates-io.inertia-axum.path` redirected to this checkout
- `cargo test --locked -p cargo-inertia` — 6 passed, 1 ignored
- `./scripts/test-live-ssr.sh` — six managed lifecycle tests, clean client/SSR
  builds, and production SSR integration for all three adapters
- clean pnpm 11 Svelte install/build after the audit policy fix
- clean production hydration Playwright tests for Svelte, React 19.2.7, and
  Vue, clearing the shared managed-SSR port between local runs

The Vite SSR builds emit the known plugin sourcemap warning; builds and all
runtime tests succeed.

## Fresh-project quick-start matrix

Each row started from a new `/tmp/inertia-quickstart-<framework>` Cargo project,
ran the locally built `cargo-inertia init`, used the documented manifest and
canonical `src/main.rs`, installed with pnpm 11, and used a Cargo invocation-only
local patch for the unpublished crate.

| Check | Svelte | React | Vue |
| --- | --- | --- | --- |
| Empty directory and scaffold generation | Pass | Pass | Pass |
| Generated Home equals canonical snippet byte-for-byte | Pass | Pass (`Home.tsx`) | Pass |
| pnpm 11 install and production Vite build | Pass | Pass | Pass |
| Rust application compiles and starts | Pass | Pass | Pass |
| Initial response contains `greeting`, omits `stats`, advertises `deferredProps.default` | Pass, 2.5 ms | Pass, 2.3 ms | Pass, 2.2 ms |
| Version-matched Inertia partial request returns JSON `stats` after slow work | Pass, 754 ms | Pass, 754 ms | Pass, 756 ms |
| Resolved values are `projects: 3`, `tasks: 12` | Pass | Pass | Pass |
| Loading fallback exists in compiled canonical component | Pass | Pass | Pass |
| Production manifest/assets load | Pass | Pass | Pass |
| `VITE_DEV_SERVER_URL` emits Vite client and `src/main.ts` tags | Pass | Pass | Pass |
| Inertia navigation/partial request remains JSON | Pass | Pass | Pass |
| Documentation sources copy without correction | Pass | Pass | Pass |

## Documentation-site QA

| Item | Evidence | Status |
| --- | --- | --- |
| Routes, previous URLs, headings, and anchors | link checker validated all 60 pages | Pass |
| Descriptive search titles/descriptions | content checker metadata validation and `/api/search` build | Pass |
| File titles and source-only copy payload | titled-fence checker and `Snippet` rendering contract | Pass |
| Tabs, narrow-width behavior, page-wide overflow, and light/dark visual readability | in-app browser selection returned `No browser is available`; browser list was empty | Unverifiable in this session |

No visual pass is claimed for the unavailable items. The responsive Tabs and
code-overflow components were not changed by the content overhaul, and the site
compiled successfully, but those are supporting facts rather than a substitute
for visual evidence.

## Publication and environment limitations

- `inertia-axum = "=1.0.0-alpha.1"` and `cargo-inertia 1.0.0-alpha.1` are not
  currently resolvable from crates.io. The exact documented manifest was retained;
  Cargo's invocation-only patch and the locally built CLI verified the checkout.
  The literal `cargo add`/`cargo install` registry resolution remains dependent on
  publication.
- Local Playwright web-server shutdown can leave its managed Node child on the
  shared default SSR port. A stale Svelte renderer initially caused React/Vue
  mismatches during this audit. Clean per-framework runs all pass, matching CI's
  isolated matrix jobs; the audit cleared the port between local runs.
- The in-app browser had no available backend, so the visual-only site checks above
  remain explicitly unverifiable rather than reported as passing.

## Version-control record

Phases 01 through 06 were committed separately as required:

- `1c728e5 docs: add content correctness infrastructure`
- `f6fb951 docs: rebuild quick start and frontend scaffold`
- `e14aa48 docs: add complete deferred frontend workflows`
- `7c6c71b docs: streamline concepts guides and recipes`
- `6b94d0a docs: streamline server-side rendering guide`
- `8b8a080 docs: refine advanced reference and migration material`

The generated `next-env.d.ts` tracking cleanup was committed separately at the
user's request as `40e5935 docs: remove obsolete TypeScript environment file`.
The final audit commit contains this record and the confirmed audit fixes. No
changes were pushed.
