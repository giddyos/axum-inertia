# `inertia-axum` DX source of truth

The GitHub connector did not expose the latest `inertia-axum` repository tree in this session, so I could not verify any changes made after the previously inspected Axum implementation. That earlier implementation establishes the important baseline: handlers explicitly extract `InertiaRequest`, call `request.render(...)`, provide an HTML closure, register shared props through an Axum extension, and install a separate version layer.  The full-stack example also manually loads the Vite manifest, computes the asset version, builds asset tags, renders the document shell, mounts static assets, and installs multiple layers.

The design below replaces that application-facing surface. It should be treated as the intended public API, behavior contract, workspace layout, and implementation boundary.

---

# 1. The intended experience

## Minimal application

```toml
[dependencies]
axum = "0.8"
inertia-axum = { version = "0.1", features = ["vite", "macros"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

```rust
use axum::{routing::get, Router};
use inertia_axum::prelude::*;

async fn home() -> Page {
    page!("Home", {
        greeting: "Hello from Axum",
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(home))
        .inertia(Inertia::vite("frontend").build()?);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

That is the complete server-side setup.

There should be:

* No `InertiaRequest` extractor in ordinary handlers.
* No `Response` return type in ordinary handlers.
* No HTML-rendering closure in every handler.
* No manual Vite manifest parsing.
* No separate asset-version layer.
* No `Extension<SharedProps>`.
* No protocol header handling in application code.
* No framework-specific response conversion after redirects.

## Typed application page

```rust
#[derive(InertiaPage)]
#[inertia(component = "Todos/Index", rename_all = "camelCase")]
struct TodosIndexPage {
    todos: Vec<TodoDto>,
    filters: TodoFilters,
    stats: Prop<TodoStats>,
}

async fn index(
    State(app): State<AppState>,
    Query(filters): Query<TodoFilters>,
) -> Result<TodosIndexPage, AppError> {
    let todos = app.todos.search(&filters).await?;

    Ok(TodosIndexPage {
        todos,
        filters,
        stats: defer({
            let todos = app.todos.clone();

            move || async move {
                todos.stats().await
            }
        })
        .group("summary"),
    })
}
```

The page type itself is a valid Axum response. The derive generates the component name, prop registration, response conversion, page metadata, and typed prop keys used by tests.

## Typed test

```rust
#[tokio::test]
async fn index_returns_todos_without_loading_deferred_stats() {
    let app = TestApp::new(test_app().await);

    let response = app
        .inertia_get("/todos")
        .send()
        .await
        .assert_ok();

    let page = response.assert_page::<TodosIndexPage>();

    let todos = page.prop(TodosIndexPage::TODOS);
    assert_eq!(todos.len(), 2);

    page.assert_missing(TodosIndexPage::STATS);
}
```

---

# 2. Core design decisions

## 2.1 Keep Axum visible

The crate should enhance Axum rather than replace it.

Applications continue using:

```rust
Router::new()
    .route("/todos", get(index).post(store))
    .route("/todos/{id}", patch(update).delete(destroy))
    .with_state(state)
```

The crate should not introduce:

```rust
#[controller]
#[get]
#[post]
routes![]
inertia_routes![]
```

Route macros duplicate Axum, complicate tooling, make handler errors harder to understand, and make testing less direct. Standard Axum routing is already concise.

## 2.2 One application object

All application-wide Inertia behavior should be owned by one immutable, cloneable object:

```rust
pub struct Inertia {
    inner: Arc<InertiaInner>,
}
```

It owns:

```rust
struct InertiaInner {
    assets: Arc<dyn AssetProvider>,
    root: Arc<dyn RootView>,
    shared: Option<Arc<dyn ErasedShare>>,
    transient: Option<Arc<dyn TransientStore>>,
    error_handler: Arc<dyn ErrorHandler>,
    config: InertiaConfig,
}
```

Applications install it once:

```rust
Router::new()
    .route(...)
    .with_state(state)
    .inertia(inertia)
```

## 2.3 Direct response values

Pages, redirects, external locations, validation failures, and flash data should be returned as ordinary Axum response values:

```rust
async fn index() -> TodosIndexPage
async fn store() -> Result<Redirect, AppError>
async fn portal() -> Result<Location, AppError>
```

The response objects initially create an internal pending response. The Inertia layer finalizes it after the handler returns.

Conceptually:

```rust
enum PendingResponse {
    Page(PendingPage),
    Redirect(PendingRedirect),
    Location(PendingLocation),
    InvalidForm(PendingValidation),
}
```

This internal mechanism is what makes direct returns possible without losing access to request headers, method, URI, shared data, version information, or the initial HTML renderer.

Normal Axum responses remain untouched:

```rust
async fn health() -> &'static str {
    "ok"
}
```

## 2.4 Macros should describe data, not control flow

The core macro budget should be deliberately small:

```rust
#[derive(InertiaPage)]
#[derive(InertiaProps)]
page!()
```

An optional validation feature adds:

```rust
#[derive(InertiaForm)]
```

There should be no route macro, handler macro, `main` macro, application macro, shared-data macro, or testing assertion macro.

## 2.5 Protocol behavior belongs in values

Deferred, optional, once, merge, and scroll behavior should be visible where a value is constructed:

```rust
stats: defer(load_stats).group("summary"),
plans: once(load_plans).key("plans:v2"),
audit: optional(load_audit),
feed: merge(feed).append_at("data").match_on("data", "id"),
```

It should not be spread across page-level strings:

```rust
// Avoid this style as the primary API.
#[inertia(deferred = "stats")]
#[inertia(merge = "feed")]
#[inertia(match_on = "feed.data.id")]
```

The field type tells the derive that a dynamic prop exists. The constructed value describes its policy.

---

# 3. Public module organization

The crate root should remain small.

```rust
pub mod prelude;
pub mod form;
pub mod transient;
pub mod assets;
pub mod testing_support;

pub mod advanced {
    pub use crate::{
        AssetProvider,
        Component,
        ErrorHandler,
        InertiaRequestContext,
        PageOptions,
        PropOptions,
        RootContext,
        RootView,
        ShareContext,
        TransientStore,
        Visit,
    };
}
```

## Prelude

```rust
pub mod prelude {
    pub use crate::{
        always,
        defer,
        lazy,
        merge,
        once,
        optional,
        page,
        scroll,
        Inertia,
        InertiaForm,
        InertiaPage,
        InertiaProps,
        Location,
        Page,
        Prop,
        Redirect,
        RouterInertiaExt,
        Share,
        ShareContext,
        Validated,
    };
}
```

Low-level protocol headers and serialized page-object structs should not dominate root-level autocomplete.

---

# 4. Application setup

## 4.1 `Inertia`

```rust
#[derive(Clone)]
pub struct Inertia {
    inner: Arc<InertiaInner>,
}

impl Inertia {
    /// Starts a convention-based Vite setup.
    pub fn vite(root: impl Into<PathBuf>) -> InertiaBuilder;

    /// Starts a setup with an application-defined root renderer.
    pub fn builder(root: impl RootView) -> InertiaBuilder;

    /// Starts a deterministic setup intended for tests.
    #[cfg(feature = "test-support")]
    pub fn testing() -> InertiaBuilder;
}
```

## 4.2 Builder

```rust
pub struct InertiaBuilder {
    assets: Option<Arc<dyn AssetProvider>>,
    root: Option<Arc<dyn RootView>>,
    shared: Option<Arc<dyn ErasedShare>>,
    transient: Option<Arc<dyn TransientStore>>,
    error_handler: Option<Arc<dyn ErrorHandler>>,
    config: InertiaConfig,
}

impl InertiaBuilder {
    pub fn entry(self, entry: impl Into<PathBuf>) -> Self;
    pub fn build_dir(self, path: impl Into<PathBuf>) -> Self;
    pub fn public_path(self, path: impl Into<String>) -> Self;
    pub fn dev_server(self, url: impl Into<String>) -> Self;

    pub fn root(self, root: impl RootView) -> Self;
    pub fn share<S: Share>(self, shared: S) -> Self;
    pub fn transient<T: TransientStore>(self, store: T) -> Self;
    pub fn error_handler<E: ErrorHandler>(self, handler: E) -> Self;

    pub fn build(self) -> Result<Inertia, ConfigError>;
}
```

## 4.3 Router extension

```rust
pub trait RouterInertiaExt<S> {
    fn inertia(self, inertia: Inertia) -> Self;
}
```

Usage:

```rust
let inertia = Inertia::vite("frontend")
    .share(AppShare {
        config: state.config.clone(),
    })
    .transient(CookieTransient::encrypted(app_key))
    .build()?;

let app = Router::new()
    .route("/", get(home))
    .route("/todos", get(todos::index).post(todos::store))
    .route("/todos/{id}", patch(todos::update).delete(todos::destroy))
    .with_state(state)
    .inertia(inertia);
```

`.inertia(...)` installs:

1. Request-header parsing.
2. Asset-version checking.
3. Request context extensions.
4. Pending-response finalization.
5. Initial HTML rendering.
6. JSON Inertia responses.
7. Shared props.
8. Flash data.
9. Validation errors.
10. Static Vite asset serving when configured.

---

# 5. Convention-based Vite setup

## Common convention

```rust
let inertia = Inertia::vite("frontend").build()?;
```

Expected project:

```text
frontend/
├── package.json
├── vite.config.ts
├── src/
│   ├── main.ts
│   └── Pages/
│       └── Home.svelte
└── dist/
    └── .vite/
        └── manifest.json
```

Defaults:

```text
entry:          src/main.ts
pages:          src/Pages
build output:   dist
manifest:       dist/.vite/manifest.json
public prefix:  /build
root element:   app
```

## Customized setup

```rust
let inertia = Inertia::vite("web")
    .entry("src/app.ts")
    .build_dir("public/build")
    .public_path("/assets")
    .build()?;
```

## Development behavior

In development, the adapter should:

* Use `VITE_DEV_SERVER_URL` when present.
* Add the Vite client script.
* Add the configured entry script.
* Avoid requiring a production manifest.
* Leave static source delivery to Vite.
* Produce a precise startup error when the URL is malformed.

## Production behavior

In production, the adapter should:

* Read the manifest once during `build()`.
* Resolve the entry and imported CSS.
* Calculate a stable asset version.
* Serve the configured build directory.
* Fail during startup when the manifest or entry is missing.
* Never read or parse the manifest on every request.

Example startup error:

```text
inertia-axum Vite configuration error

Entry "src/main.ts" was not found in:
frontend/dist/.vite/manifest.json

Available entries:
  - src/admin.ts
  - src/app.ts
```

## Advanced asset interface

```rust
pub trait AssetProvider: Clone + Send + Sync + 'static {
    fn version(&self) -> &str;

    fn render_tags(
        &self,
        context: AssetContext<'_>,
    ) -> Result<AssetTags, AssetError>;

    fn static_service(&self) -> Option<StaticAssetService>;
}
```

This permits non-Vite asset pipelines without changing page handlers.

---

# 6. Root document rendering

The default Vite setup should supply a safe document automatically.

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <!-- resolved asset tags -->
</head>
<body>
  <!-- safely escaped Inertia page object and root element -->
</body>
</html>
```

Applications should not manually interpolate raw page JSON.

## Root trait

```rust
pub trait RootView: Clone + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn render(
        &self,
        context: RootContext<'_>,
    ) -> Result<String, Self::Error>;
}
```

## Safe rendering context

```rust
pub struct RootContext<'a> {
    title: Option<&'a str>,
    locale: Option<&'a str>,
    assets: &'a AssetTags,
    mount: &'a MountMarkup,
    nonce: Option<&'a str>,
}

impl RootContext<'_> {
    pub fn title(&self) -> Option<&str>;
    pub fn locale(&self) -> Option<&str>;

    /// Pre-rendered, safe HTML for scripts and styles.
    pub fn assets(&self) -> &AssetTags;

    /// Pre-rendered, safe HTML containing the root and page data.
    pub fn mount(&self) -> &MountMarkup;

    pub fn nonce(&self) -> Option<&str>;
}
```

## Custom root

```rust
#[derive(Clone)]
struct AppRoot;

impl RootView for AppRoot {
    type Error = std::convert::Infallible;

    fn render(
        &self,
        context: RootContext<'_>,
    ) -> Result<String, Self::Error> {
        Ok(format!(
            r#"<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="theme-color" content="#111827">
    {assets}
</head>
<body>
    <header id="site-header"></header>
    {mount}
</body>
</html>"#,
            assets = context.assets(),
            mount = context.mount(),
        ))
    }
}
```

Setup:

```rust
let inertia = Inertia::vite("frontend")
    .root(AppRoot)
    .build()?;
```

Root rendering remains application-wide.

---

# 7. Dynamic pages with `page!`

`page!` is the concise API for small pages and prototypes.

```rust
async fn home() -> Page {
    page!("Home", {
        greeting: "Hello",
        signed_in: false,
    })
}
```

Shorthand fields:

```rust
async fn dashboard(
    State(app): State<AppState>,
) -> Result<Page, AppError> {
    let user = app.current_user().await?;
    let projects = app.projects.for_user(user.id).await?;

    Ok(page!("Dashboard", {
        user,
        projects,
        stats: defer({
            let analytics = app.analytics.clone();

            move || async move {
                analytics.dashboard().await
            }
        }),
    }))
}
```

The macro should expand to ordinary public builder APIs. It must not depend on hidden global state.

Conceptually:

```rust
Page::new("Dashboard")
    .prop("user", user)
    .prop("projects", projects)
    .prop("stats", defer(...))
```

## Page-level modifiers

```rust
page!("Checkout/Complete", {
    order,
})
.clear_history()
.encrypt_history()
```

Flash data:

```rust
page!("Projects/Index", {
    projects,
})
.flash("highlight", project_id)
```

Custom status:

```rust
page!("Errors/NotFound", {
    message: "Todo not found",
})
.status(StatusCode::NOT_FOUND)
```

---

# 8. Typed page structs

## Trait

```rust
pub trait InertiaPage: IntoInertiaProps + Send + 'static {
    const COMPONENT: Component;

    fn options() -> PageOptions {
        PageOptions::default()
    }
}
```

## Props trait

```rust
pub trait IntoInertiaProps: Send + 'static {
    fn into_inertia_props(self) -> Props;
}
```

## Component type

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Component(&'static str);

impl Component {
    pub const fn new(name: &'static str) -> Self;
    pub const fn as_str(self) -> &'static str;
}
```

## Typed prop keys

```rust
pub struct PropKey<T> {
    component: Component,
    name: &'static str,
    marker: PhantomData<fn() -> T>,
}
```

The derive generates typed constants:

```rust
impl TodosIndexPage {
    pub const TODOS: PropKey<Vec<TodoDto>>;
    pub const FILTERS: PropKey<TodoFilters>;
    pub const STATS: PropKey<TodoStats>;
}
```

These are used by tests and partial reload helpers.

## Derive example

```rust
#[derive(InertiaPage)]
#[inertia(
    component = "Todos/Index",
    rename_all = "camelCase",
    encrypt_history
)]
struct TodosIndexPage {
    todos: Vec<TodoDto>,
    filters: TodoFilters,
    stats: Prop<TodoStats>,

    #[inertia(rename = "canCreate")]
    can_create: bool,
}
```

The derive generates:

* `IntoInertiaProps`.
* `InertiaPage`.
* `IntoResponse` for the concrete page type.
* `COMPONENT`.
* Typed `PropKey<T>` constants.
* Static page options.
* Top-level prop-name conversion.
* Compile-time diagnostics.

## Supported page attributes

```rust
#[inertia(component = "Todos/Index")]
#[inertia(rename_all = "camelCase")]
#[inertia(encrypt_history)]
#[inertia(clear_history)]
#[inertia(preserve_fragment)]
```

## Supported field attributes

```rust
#[inertia(rename = "canCreate")]
#[inertia(skip)]
```

Prop policies should not be field attributes. They belong to `Prop<T>` values.

## Compile-time diagnostics

```rust
#[derive(InertiaPage)]
struct MissingComponent {
    todos: Vec<Todo>,
}
```

```text
error: InertiaPage requires #[inertia(component = "...")]
```

```rust
#[derive(InertiaPage)]
#[inertia(component = "")]
struct EmptyComponent {}
```

```text
error: Inertia component names cannot be empty
```

```rust
#[derive(InertiaPage)]
#[inertia(component = "Todos/Index")]
struct ReservedProp {
    errors: ErrorBag,
}
```

```text
error: "errors" is reserved by the Inertia validation protocol
```

---

# 9. One composable prop type

The earlier design introduced separate public types such as `Deferred<T>`, `Optional<T>`, `Once<T>`, and `Merge<T>`. A smaller and more flexible API uses one type:

```rust
pub struct Prop<T> {
    resolver: Resolver<T>,
    options: PropOptions,
}
```

Users construct it through focused functions:

```rust
lazy(...)
defer(...)
optional(...)
always(...)
once(...)
merge(...)
scroll(...)
```

Every helper returns `Prop<T>`, so page fields remain simple:

```rust
struct DashboardPage {
    stats: Prop<Stats>,
    plans: Prop<Vec<Plan>>,
    audit: Prop<Vec<AuditEntry>>,
}
```

## Conceptual prop options

```rust
pub struct PropOptions {
    load: LoadPolicy,
    once: Option<OncePolicy>,
    merge: Option<MergePolicy>,
    rescue: bool,
}

pub enum LoadPolicy {
    Standard,
    Lazy,
    Always,
    Optional,
    Deferred {
        group: Cow<'static, str>,
    },
}

pub struct OncePolicy {
    key: Option<Cow<'static, str>>,
    expires_at: Option<SystemTime>,
    fresh: bool,
}

pub enum MergePolicy {
    Append {
        path: Option<Cow<'static, str>>,
        match_on: Option<Cow<'static, str>>,
    },
    Prepend {
        path: Option<Cow<'static, str>>,
        match_on: Option<Cow<'static, str>>,
    },
    Deep {
        match_on: Vec<Cow<'static, str>>,
    },
    Scroll(ScrollPolicy),
}
```

This unified policy model is important because Inertia prop behaviors are composable. Deferred props may be grouped and rescued; merge and once behavior can be combined with other prop types. Official Inertia v3 treats deferred groups as separate follow-up requests and supports rescued deferred props. ([Inertia.js][1]) Merge behavior applies to partial reloads and includes append, prepend, nested paths, and matching. ([Inertia.js][2]) Once props support expiration, custom keys, explicit refreshes, and composition with deferred, optional, and merge props. ([Inertia.js][3])

## Immediate field

```rust
struct Page {
    users: Vec<UserDto>,
}
```

The handler computes the value immediately. Serialization can still be skipped when a matching partial reload excludes the field.

## Lazy field

```rust
struct Page {
    companies: Prop<Vec<CompanyDto>>,
}
```

```rust
companies: lazy({
    let companies = app.companies.clone();

    move || async move {
        companies.list().await
    }
}),
```

Behavior:

* Resolve on initial visits.
* Resolve on full reloads.
* Resolve on partial reloads that include the prop.
* Do not execute when a matching partial reload omits it.

## Always field

```rust
csrf_token: always(csrf_token),
```

Behavior:

* Include during initial requests.
* Include during matching partial reloads even when not explicitly selected.

## Optional field

```rust
audit: optional({
    let audit = app.audit.clone();

    move || async move {
        audit.for_user(user_id).await
    }
}),
```

Behavior:

* Do not resolve on an initial visit.
* Do not resolve on an ordinary reload.
* Resolve only when explicitly requested.

## Deferred field

```rust
stats: defer({
    let analytics = app.analytics.clone();

    move || async move {
        analytics.stats().await
    }
}),
```

Grouped:

```rust
stats: defer(load_stats).group("dashboard"),
projects: defer(load_projects).group("dashboard"),
notifications: defer(load_notifications).group("notifications"),
```

The client requests `stats` and `projects` together while `notifications` can load in parallel as a separate deferred group. That follows the Inertia deferred-group model. ([Inertia.js][1])

Rescue:

```rust
stats: defer(load_stats)
    .group("dashboard")
    .rescue(),
```

`rescue()` should mean:

* Report the resolver error through the configured error handler.
* Omit the prop from the response.
* Add the prop to `rescuedProps`.
* Allow the client’s deferred rescue UI to handle the missing prop.

It should not silently substitute an arbitrary server fallback because that would blur the protocol’s rescued state.

## Once field

```rust
plans: once({
    let billing = app.billing.clone();

    move || async move {
        billing.plans().await
    }
})
.key("billing-plans:v2")
.expires_in(Duration::from_secs(60 * 60)),
```

Conditional refresh:

```rust
plans: once(load_plans)
    .key("billing-plans:v2")
    .fresh_if(admin_forced_refresh),
```

Composition:

```rust
stats: defer(load_stats)
    .group("dashboard")
    .once()
    .key("dashboard-stats:v4")
    .expires_in(Duration::from_secs(300)),
```

## Merge field

Append at the root:

```rust
notifications: merge(notifications)
    .append()
    .match_on("id"),
```

Append a nested path:

```rust
users: merge(paginated_users)
    .append_at("data")
    .match_on_at("data", "id"),
```

Prepend:

```rust
events: merge(new_events)
    .prepend()
    .match_on("id"),
```

Deep merge:

```rust
chat: merge(chat)
    .deep()
    .match_on("messages.id"),
```

## Infinite scroll

Application pagination types should implement a small adapter trait:

```rust
pub trait IntoScrollPage {
    type Item: Serialize + Send + 'static;

    fn into_scroll_page(self) -> ScrollPage<Self::Item>;
}
```

Framework type:

```rust
pub struct ScrollPage<T> {
    data: Vec<T>,
    page_name: Cow<'static, str>,
    current: Cursor,
    previous: Option<Cursor>,
    next: Option<Cursor>,
}
```

Usage:

```rust
feed: scroll(page)
    .page_name("feed")
    .match_on("id"),
```

The helper automatically adds the correct merge and scroll metadata. Inertia v3’s scroll helper is intended to normalize pagination data and configure merging together, rather than making application code coordinate the two manually. ([Inertia.js][4])

---

# 10. Shared props

Shared data should be represented by one typed provider, not a registry of independently named closures.

Inertia recommends keeping shared data small because it is merged into every response. ([Inertia.js][5]) The API should reinforce that by returning one clear shared-data object.

## Trait

```rust
pub trait Share: Clone + Send + Sync + 'static {
    type Props: IntoInertiaProps;
    type Error: std::error::Error + Send + Sync + 'static;

    async fn share(
        &self,
        context: ShareContext<'_>,
    ) -> Result<Self::Props, Self::Error>;
}
```

## Context

```rust
pub struct ShareContext<'a> {
    method: &'a Method,
    uri: &'a Uri,
    headers: &'a HeaderMap,
    extensions: &'a Extensions,
    visit: &'a Visit,
}

impl ShareContext<'_> {
    pub fn method(&self) -> &Method;
    pub fn uri(&self) -> &Uri;
    pub fn headers(&self) -> &HeaderMap;
    pub fn visit(&self) -> &Visit;

    pub fn extension<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static;
}
```

## Typed shared props

```rust
#[derive(InertiaProps)]
#[inertia(rename_all = "camelCase")]
struct AppSharedProps {
    app_name: String,
    environment: String,
    auth: Option<AuthShared>,
    countries: Prop<Vec<CountryDto>>,
}
```

Provider:

```rust
#[derive(Clone)]
struct AppShare {
    config: Arc<AppConfig>,
    countries: CountryRepository,
}

impl Share for AppShare {
    type Props = AppSharedProps;
    type Error = RepositoryError;

    async fn share(
        &self,
        context: ShareContext<'_>,
    ) -> Result<Self::Props, Self::Error> {
        let auth = context
            .extension::<CurrentUser>()
            .map(|user| AuthShared {
                user: user.summary(),
                permissions: user.permissions(),
            });

        Ok(AppSharedProps {
            app_name: self.config.name.clone(),
            environment: self.config.environment.clone(),
            auth,
            countries: once({
                let countries = self.countries.clone();

                move || async move {
                    countries.all().await
                }
            })
            .key("countries:v1")
            .expires_in(Duration::from_secs(24 * 60 * 60)),
        })
    }
}
```

Registration:

```rust
let inertia = Inertia::vite("frontend")
    .share(AppShare {
        config: state.config.clone(),
        countries: state.countries.clone(),
    })
    .build()?;
```

Rules:

* There is one shared provider per application.
* Route props win on top-level collisions.
* Shared prop names are generated through the same derive logic as page prop names.
* Shared props may use `Prop<T>`, including once props.
* Validation errors and flash data are not ordinary shared props. They have dedicated protocol fields and dedicated runtime handling.

---

# 11. Flash data and transient state

Inertia v3 flash values live in `page.flash` and are not persisted in browser history. They can be attached to redirects or page responses. ([Inertia.js][6]) The server adapter should represent them separately from page props.

## Redirect flash

```rust
Ok(
    Redirect::to("/todos")
        .flash("toast", Toast::success("Todo created")),
)
```

Multiple values:

```rust
Redirect::to("/projects")
    .flash("toast", Toast::success("Project created"))
    .flash("newProjectId", project.id)
```

## Page flash

```rust
page!("Projects/Index", {
    projects,
})
.flash("highlight", project.id)
```

## Transient store

Redirected flash values and validation errors require request-to-request storage.

```rust
pub trait TransientStore: Clone + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn load(
        &self,
        request: TransientRequest<'_>,
    ) -> Result<TransientData, Self::Error>;

    async fn commit(
        &self,
        response: &mut Response,
        data: TransientData,
    ) -> Result<(), Self::Error>;
}
```

Built-in adapters:

```rust
CookieTransient::encrypted(key)
MemoryTransient::new()       // tests only
TowerSessionTransient::new() // optional tower-sessions feature
```

Setup:

```rust
let inertia = Inertia::vite("frontend")
    .transient(CookieTransient::encrypted(app_key))
    .build()?;
```

There should be no insecure production default. Pages and redirects without flash or validation work without a transient store. Trying to use flash or redirected validation without one produces a clear configuration error.

---

# 12. Forms and validation

Inertia validation should not return a `422` JSON response. The protocol expects the server to redirect back, store errors transiently, and expose them through `page.props.errors`; the client determines failure by inspecting the errors prop. Error bags namespace errors for pages containing multiple forms. ([Inertia.js][7]) ([Inertia.js][7])

The adapter should automate this.

## Optional form derive

```rust
#[derive(Debug, Deserialize, garde::Validate, InertiaForm)]
#[inertia(
    validator = "garde",
    error_bag = "createTodo"
)]
struct CreateTodo {
    #[garde(length(min = 1, max = 200))]
    title: String,
}
```

## Validated extractor

```rust
async fn store(
    State(app): State<AppState>,
    Validated(input): Validated<CreateTodo>,
) -> Result<Redirect, AppError> {
    app.todos.create(input).await?;

    Ok(
        Redirect::to("/todos")
            .flash("toast", Toast::success("Todo created")),
    )
}
```

When validation fails, `Validated<T>` should reject before the handler executes. The Inertia layer turns the rejection into:

1. Errors converted to the standard error map.
2. Errors scoped to the request’s `X-Inertia-Error-Bag`, when present.
3. The derive’s `error_bag` used as a fallback.
4. Errors stored through `TransientStore`.
5. A `303 See Other` redirect to the previous location.
6. Errors automatically added to the next page’s `errors` prop.

The adapter should not build a server-side “old input” system by default. Inertia preserves component state for write requests when validation redirects back, so form state normally remains on the client. ([Inertia.js][7])

## Local validation trait

The derive implements:

```rust
pub trait Validate {
    fn validate(&self) -> Result<(), Errors>;
}
```

Generated conceptually:

```rust
impl inertia_axum::form::Validate for CreateTodo {
    fn validate(&self) -> Result<(), Errors> {
        garde::Validate::validate(self, &())
            .map_err(Errors::from_garde)
    }
}
```

Supported derive configurations:

```rust
#[inertia(validator = "garde")]
#[inertia(validator = "validator")]
#[inertia(validate_with = "crate::validation::validate_create_todo")]
```

## Non-derived form

Applications must also be able to use the lower-level extractor:

```rust
async fn store(
    form: InertiaForm<CreateTodo>,
) -> Result<Redirect, FormError> {
    let input = form.validate_with(|input| {
        domain_validation::validate_todo(input)
    })?;

    save(input).await?;

    Ok(Redirect::to("/todos"))
}
```

## Content types

The initial form implementation should support:

* `application/json`
* `application/x-www-form-urlencoded`

Multipart file upload support should be a separate feature and extractor rather than making the common form extractor substantially more complex.

---

# 13. Redirects and external locations

## Internal redirect

```rust
async fn store() -> Redirect {
    Redirect::to("/todos")
}
```

The finalizer chooses:

* `302 Found` for read requests.
* `303 See Other` for write requests.

## Back

```rust
Redirect::back()
```

Fallback:

```rust
Redirect::back_or("/todos")
```

## External location

```rust
async fn billing(
    State(app): State<AppState>,
) -> Result<Location, AppError> {
    let url = app.billing.portal_url().await?;
    Ok(Location::external(url))
}
```

The layer chooses the appropriate Inertia conflict response or normal browser redirect.

## Fragment handling

```rust
Location::external("https://docs.example.com/start#authentication")
```

Fragment-specific protocol behavior remains internal to the finalizer.

---

# 14. Advanced request access

Ordinary pages should not require an Inertia extractor. An advanced extractor remains available for diagnostics and unusual protocol-aware behavior.

```rust
pub struct Visit {
    context: InertiaRequestContext,
    method: Method,
    uri: Uri,
}

impl Visit {
    pub fn is_inertia(&self) -> bool;
    pub fn is_partial(&self) -> bool;
    pub fn is_prefetch(&self) -> bool;
    pub fn is_reload(&self) -> bool;
    pub fn version(&self) -> Option<&str>;
    pub fn requested_props(&self) -> &[String];
    pub fn excluded_props(&self) -> &[String];
    pub fn error_bag(&self) -> Option<&str>;
}
```

Usage:

```rust
async fn debug(visit: Visit) -> String {
    format!(
        "inertia={}, partial={}, prefetch={}",
        visit.is_inertia(),
        visit.is_partial(),
        visit.is_prefetch(),
    )
}
```

This is an escape hatch, not the primary response API.

---

# 15. Error pages

A special error-controller abstraction is unnecessary. Standard Axum `IntoResponse` remains sufficient.

```rust
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("todo not found")]
    TodoNotFound,

    #[error("permission denied")]
    Forbidden,

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::TodoNotFound => page!("Errors/NotFound", {
                message: "The requested todo does not exist",
            })
            .status(StatusCode::NOT_FOUND)
            .into_response(),

            Self::Forbidden => page!("Errors/Forbidden", {
                message: "You do not have access to this resource",
            })
            .status(StatusCode::FORBIDDEN)
            .into_response(),

            Self::Database(error) => {
                tracing::error!(%error, "database request failed");

                page!("Errors/Server", {
                    message: "An unexpected error occurred",
                })
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .into_response()
            }
        }
    }
}
```

The Inertia layer finalizes pages produced by error responses exactly as it finalizes successful pages.

---

# 16. Missing-layer diagnostics

A page accidentally returned without installing the Inertia layer should produce an actionable development response:

```text
An Inertia page was returned, but the Inertia layer is not installed.

Install it on the router:

let app = Router::new()
    .route("/", get(index))
    .inertia(inertia);
```

It should not silently return an empty body or cryptic serialization error.

---

# 17. Testing API

Testing support should be a separate crate so production dependency trees remain focused:

```toml
[dev-dependencies]
inertia-axum-test = "0.1"
```

## Test application

```rust
use inertia_axum_test::TestApp;

let app = TestApp::new(router);
```

`TestApp` should use Tower directly. Tests do not need to bind a TCP port.

It maintains:

* Cookies.
* Transient validation state.
* Flash state.
* The configured test asset version.
* Redirect history.

## Initial HTML request

```rust
let response = app
    .get("/todos")
    .send()
    .await
    .assert_ok()
    .assert_html();

response.assert_html_page::<TodosIndexPage>();
```

## Inertia request

```rust
let page = app
    .inertia_get("/todos")
    .send()
    .await
    .assert_ok()
    .assert_page::<TodosIndexPage>();
```

## Typed prop access

```rust
let todos: Vec<TodoDto> = page.prop(TodosIndexPage::TODOS);
let filters: TodoFilters = page.prop(TodosIndexPage::FILTERS);
```

Missing optional or deferred prop:

```rust
page.assert_missing(TodosIndexPage::STATS);
```

## Partial reload

A typed prop key contains its component and prop name, so one key is enough to configure the headers correctly:

```rust
let page = app
    .inertia_get("/todos")
    .only(TodosIndexPage::STATS)
    .send()
    .await
    .assert_ok()
    .assert_page::<TodosIndexPage>();

let stats = page.prop(TodosIndexPage::STATS);

page.assert_missing(TodosIndexPage::TODOS);
page.assert_missing(TodosIndexPage::FILTERS);
```

Multiple keys can be chained without requiring a heterogeneous array:

```rust
let page = app
    .inertia_get("/todos")
    .only(TodosIndexPage::STATS)
    .only(TodosIndexPage::ARCHIVED)
    .send()
    .await
    .assert_page::<TodosIndexPage>();
```

## Deferred resolver execution

```rust
#[tokio::test]
async fn deferred_prop_is_not_resolved_on_initial_visit() {
    let calls = Arc::new(AtomicUsize::new(0));

    let page = TestPage::new(DashboardPage {
        stats: defer({
            let calls = calls.clone();

            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, Infallible>(Stats::default())
            }
        }),
    });

    page.initial().await;

    assert_eq!(calls.load(Ordering::SeqCst), 0);

    page.partial(DashboardPage::STATS).await;

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
```

This must be a fundamental test invariant: an unselected lazy, optional, deferred, or once resolver is never created or polled.

## Validation

```rust
#[tokio::test]
async fn invalid_todo_is_redirected_with_bagged_errors() {
    let app = TestApp::new(test_app().await);

    let response = app
        .inertia_post("/todos")
        .json(&serde_json::json!({
            "title": ""
        }))
        .error_bag("createTodo")
        .send()
        .await
        .assert_see_other("/todos");

    let page = response
        .follow()
        .await
        .assert_page::<TodosIndexPage>();

    page.assert_error("createTodo.title");
}
```

## Flash

```rust
#[tokio::test]
async fn create_flashes_a_success_toast_once() {
    let app = TestApp::new(test_app().await);

    let page = app
        .inertia_post("/todos")
        .json(&serde_json::json!({
            "title": "Ship the adapter"
        }))
        .send()
        .await
        .follow()
        .await
        .assert_page::<TodosIndexPage>();

    let toast: Toast = page.flash("toast");
    assert_eq!(toast.message, "Todo created");

    let next = app
        .inertia_get("/todos")
        .send()
        .await
        .assert_page::<TodosIndexPage>();

    next.assert_no_flash("toast");
}
```

## Merge metadata

```rust
page.assert_appends(NotificationsPage::NOTIFICATIONS);
page.assert_matches_on(NotificationsPage::NOTIFICATIONS, "id");
```

## Version conflicts

```rust
app.inertia_get("/todos")
    .version("stale-version")
    .send()
    .await
    .assert_location_conflict("/todos");
```

---

# 18. Macro testing

The proc-macro crate should use compile-pass and compile-fail tests.

```text
crates/inertia-axum-macros/
├── src/
│   ├── lib.rs
│   ├── page.rs
│   ├── props.rs
│   ├── form.rs
│   ├── attributes.rs
│   └── diagnostics.rs
└── tests/
    ├── trybuild.rs
    └── ui/
        ├── pass/
        │   ├── page.rs
        │   ├── generic_page.rs
        │   ├── shared_props.rs
        │   └── garde_form.rs
        └── fail/
            ├── missing_component.rs
            ├── empty_component.rs
            ├── reserved_errors_prop.rs
            ├── unsupported_enum.rs
            └── invalid_validator.rs
```

Generated code should use absolute paths and resolve a renamed runtime dependency correctly. Procedural macros must live in a separate `proc-macro` crate and cannot be used from the same crate where they are defined. ([Rust Documentation][8])

The runtime crate reexports the derives:

```rust
#[cfg(feature = "macros")]
pub use inertia_axum_macros::{
    InertiaForm,
    InertiaPage,
    InertiaProps,
};
```

Users should never need to add `inertia-axum-macros` directly.

---

# 19. Cargo workspace structure

A virtual workspace is the cleanest arrangement because there are several peer packages rather than one root package. Cargo workspaces share one lockfile, target directory, package metadata, dependencies, and lint configuration; virtual workspaces should specify their resolver explicitly. ([Rust Documentation][9]) ([Rust Documentation][9])

```text
inertia-axum/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── LICENSE
│
├── crates/
│   ├── inertia-axum/
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── prelude.rs
│   │   │   ├── app.rs
│   │   │   ├── config.rs
│   │   │   ├── layer.rs
│   │   │   ├── engine.rs
│   │   │   ├── visit.rs
│   │   │   ├── error.rs
│   │   │   │
│   │   │   ├── response/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── pending.rs
│   │   │   │   ├── page.rs
│   │   │   │   ├── redirect.rs
│   │   │   │   └── location.rs
│   │   │   │
│   │   │   ├── props/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── prop.rs
│   │   │   │   ├── policy.rs
│   │   │   │   ├── resolver.rs
│   │   │   │   ├── merge.rs
│   │   │   │   └── scroll.rs
│   │   │   │
│   │   │   ├── assets/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── vite.rs
│   │   │   │   ├── manifest.rs
│   │   │   │   └── test.rs
│   │   │   │
│   │   │   ├── root/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── default.rs
│   │   │   │   └── context.rs
│   │   │   │
│   │   │   ├── shared.rs
│   │   │   │
│   │   │   ├── form/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── extractor.rs
│   │   │   │   ├── validation.rs
│   │   │   │   └── errors.rs
│   │   │   │
│   │   │   ├── transient/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── cookie.rs
│   │   │   │   └── memory.rs
│   │   │   │
│   │   │   └── protocol/
│   │   │       ├── mod.rs
│   │   │       ├── headers.rs
│   │   │       ├── page_object.rs
│   │   │       └── filtering.rs
│   │   │
│   │   └── tests/
│   │       ├── integration.rs
│   │       └── integration/
│   │           ├── pages.rs
│   │           ├── partials.rs
│   │           ├── deferred.rs
│   │           ├── once.rs
│   │           ├── merging.rs
│   │           ├── redirects.rs
│   │           ├── forms.rs
│   │           ├── flash.rs
│   │           ├── shared.rs
│   │           └── vite.rs
│   │
│   ├── inertia-axum-macros/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── tests/
│   │
│   ├── inertia-axum-test/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── app.rs
│   │   │   ├── request.rs
│   │   │   ├── response.rs
│   │   │   ├── page.rs
│   │   │   └── cookie_jar.rs
│   │   └── tests/
│   │
│   └── cargo-inertia/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── init.rs
│           ├── dev.rs
│           └── check.rs
│
├── examples/
│   ├── todo/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   ├── frontend/
│   │   └── tests/
│   │
│   └── incident-board/
│       ├── Cargo.toml
│       ├── src/
│       ├── frontend/
│       └── tests/
│
└── docs/
    ├── design.md
    ├── protocol-support.md
    ├── migration.md
    └── testing.md
```

## Why no `inertia-axum-core` crate

The initial workspace should not add a separate core crate.

The project is Axum-only, so splitting protocol code into another published package would introduce:

* Another versioned public package.
* More cross-crate visibility.
* More release coordination.
* A larger compile graph.
* Pressure to stabilize internal protocol types prematurely.

Keep protocol logic in a private `protocol` module. Split it only when another independently published consumer genuinely requires it.

## Root `Cargo.toml`

```toml
[workspace]
resolver = "3"
members = [
    "crates/inertia-axum",
    "crates/inertia-axum-macros",
    "crates/inertia-axum-test",
    "crates/cargo-inertia",
    "examples/todo",
    "examples/incident-board",
]
default-members = [
    "crates/inertia-axum",
    "crates/inertia-axum-macros",
    "crates/inertia-axum-test",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.88"
license = "MIT"
repository = "https://github.com/<owner>/inertia-axum"
homepage = "https://github.com/<owner>/inertia-axum"
authors = ["inertia-axum contributors"]

[workspace.dependencies]
# Public runtime
axum = "0.8"
bytes = "1"
futures-util = "0.3"
http = "1"
http-body-util = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
tower = { version = "0.5", features = ["util"] }
tower-http = { version = "0.6", features = ["fs"] }
tracing = "0.1"

# Assets and transient state
base64 = "0.22"
cookie = { version = "0.18", features = ["secure"] }
sha2 = "0.10"

# Proc macros
proc-macro2 = "1"
proc-macro-crate = "3"
quote = "1"
syn = { version = "2", features = ["full", "extra-traits"] }

# CLI
cargo_metadata = "0.20"
clap = { version = "4", features = ["derive"] }

# Testing
insta = { version = "1", features = ["json"] }
trybuild = "1"

# Workspace packages
inertia-axum = { path = "crates/inertia-axum", version = "0.1.0" }
inertia-axum-macros = { path = "crates/inertia-axum-macros", version = "0.1.0" }
inertia-axum-test = { path = "crates/inertia-axum-test", version = "0.1.0" }

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
module_name_repetitions = "allow"
must_use_candidate = "allow"

[profile.release]
lto = "thin"
codegen-units = 1
```

Cargo supports `default-members`, inherited package metadata, inherited dependencies, and workspace-level lint configuration. ([Rust Documentation][9]) ([Rust Documentation][9])

## Runtime crate manifest

```toml
[package]
name = "inertia-axum"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "An Inertia.js server adapter for Axum"

[lints]
workspace = true

[features]
default = ["macros", "vite"]

macros = ["dep:inertia-axum-macros"]
vite = ["dep:sha2", "dep:tower-http"]
cookies = ["dep:base64", "dep:cookie"]
garde = []
validator = []
tower-sessions = []
test-support = []

[dependencies]
axum.workspace = true
bytes.workspace = true
futures-util.workspace = true
http.workspace = true
http-body-util.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tower.workspace = true
tracing.workspace = true

inertia-axum-macros = {
    workspace = true,
    optional = true,
}

base64 = {
    workspace = true,
    optional = true,
}

cookie = {
    workspace = true,
    optional = true,
}

sha2 = {
    workspace = true,
    optional = true,
}

tower-http = {
    workspace = true,
    optional = true,
}
```

## Macro crate manifest

```toml
[package]
name = "inertia-axum-macros"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Procedural macros for inertia-axum"

[lib]
proc-macro = true

[lints]
workspace = true

[dependencies]
proc-macro2.workspace = true
proc-macro-crate.workspace = true
quote.workspace = true
syn.workspace = true

[dev-dependencies]
inertia-axum = {
    workspace = true,
    features = ["macros", "test-support"],
}
trybuild.workspace = true
```

## Testing crate manifest

```toml
[package]
name = "inertia-axum-test"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Testing utilities for inertia-axum applications"

[lints]
workspace = true

[dependencies]
axum.workspace = true
http.workspace = true
http-body-util.workspace = true
inertia-axum = {
    workspace = true,
    features = ["test-support"],
}
serde.workspace = true
serde_json.workspace = true
tower.workspace = true
```

---

# 20. CLI scope

The CLI should remain optional and small.

## Initialize

```bash
cargo inertia init --frontend svelte
```

Supported values:

```text
svelte
react
vue
```

It creates:

```text
frontend/
├── package.json
├── vite.config.ts
└── src/
    ├── main.ts
    └── Pages/
        └── Home.*
```

It also prints the exact Rust setup:

```rust
.inertia(Inertia::vite("frontend").build()?)
```

## Development

```bash
cargo inertia dev
```

It should:

* Start Vite.
* Set `VITE_DEV_SERVER_URL`.
* Start `cargo run`.
* Forward process output without hiding it.
* Shut down both processes when either exits.

It should not invent its own hot-reload protocol.

## Check

```bash
cargo inertia check
```

It validates:

* Frontend directory exists.
* Entry file exists.
* Production manifest can be parsed when present.
* Every literal `#[inertia(component = "...")]` has a matching frontend page.
* Duplicate component declarations.
* Invalid component paths.
* Vite entry consistency.

The CLI should not be required to build or run an application.

No initial `types`, `generate`, `routes`, or migration command is necessary. Those can be added only after a concrete need is demonstrated.

---

# 21. Todo example: minimal version

## `src/main.rs`

```rust
use axum::{routing::get, Router};
use inertia_axum::prelude::*;

async fn index() -> Page {
    let todos = [
        "Design the public API",
        "Build the response finalizer",
        "Add integration tests",
    ];

    page!("Todos/Index", {
        todos,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/todos", get(index))
        .inertia(Inertia::vite("frontend").build()?);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

This should be the README’s first complete example.

---

# 22. Todo example: production-style application

## State

```rust
#[derive(Clone)]
struct AppState {
    todos: TodoRepository,
}
```

## DTOs

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TodoDto {
    id: Uuid,
    title: String,
    completed: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TodoFilters {
    search: Option<String>,
    completed: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TodoStats {
    total: u64,
    completed: u64,
    remaining: u64,
}
```

## Shared data

```rust
#[derive(InertiaProps)]
#[inertia(rename_all = "camelCase")]
struct SharedProps {
    app_name: &'static str,
    auth: Option<AuthShared>,
}

#[derive(Clone)]
struct AppShare;

impl Share for AppShare {
    type Props = SharedProps;
    type Error = Infallible;

    async fn share(
        &self,
        context: ShareContext<'_>,
    ) -> Result<Self::Props, Self::Error> {
        let auth = context
            .extension::<CurrentUser>()
            .map(|user| AuthShared {
                user: user.summary(),
            });

        Ok(SharedProps {
            app_name: "Tasks",
            auth,
        })
    }
}
```

## Index page

```rust
#[derive(InertiaPage)]
#[inertia(
    component = "Todos/Index",
    rename_all = "camelCase"
)]
struct TodosIndexPage {
    todos: Vec<TodoDto>,
    filters: TodoFilters,
    stats: Prop<TodoStats>,
    archived: Prop<Vec<TodoDto>>,
    can_create: bool,
}
```

## Form

```rust
#[derive(Debug, Deserialize, garde::Validate, InertiaForm)]
#[inertia(
    validator = "garde",
    error_bag = "createTodo"
)]
struct CreateTodo {
    #[garde(length(min = 1, max = 200))]
    title: String,
}
```

## Index handler

```rust
async fn index(
    State(app): State<AppState>,
    Query(filters): Query<TodoFilters>,
) -> Result<TodosIndexPage, AppError> {
    let todos = app.todos.search(&filters).await?;

    Ok(TodosIndexPage {
        todos,
        filters,
        can_create: true,

        stats: defer({
            let todos = app.todos.clone();

            move || async move {
                todos.stats().await
            }
        })
        .group("summary"),

        archived: optional({
            let todos = app.todos.clone();

            move || async move {
                todos.archived().await
            }
        }),
    })
}
```

## Create handler

```rust
async fn store(
    State(app): State<AppState>,
    Validated(input): Validated<CreateTodo>,
) -> Result<Redirect, AppError> {
    let todo = app.todos.create(input.title).await?;

    Ok(
        Redirect::to(format!("/todos/{}", todo.id))
            .flash("toast", Toast::success("Todo created"))
            .flash("newTodoId", todo.id),
    )
}
```

## Show page

```rust
#[derive(InertiaPage)]
#[inertia(
    component = "Todos/Show",
    rename_all = "camelCase"
)]
struct TodoShowPage {
    todo: TodoDto,
}
```

```rust
async fn show(
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<TodoShowPage, AppError> {
    Ok(TodoShowPage {
        todo: app
            .todos
            .find(id)
            .await?
            .ok_or(AppError::TodoNotFound)?,
    })
}
```

## Update form

```rust
#[derive(Debug, Deserialize, garde::Validate, InertiaForm)]
#[inertia(
    validator = "garde",
    error_bag = "updateTodo"
)]
struct UpdateTodo {
    #[garde(length(min = 1, max = 200))]
    title: String,

    completed: bool,
}
```

## Update handler

```rust
async fn update(
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
    Validated(input): Validated<UpdateTodo>,
) -> Result<Redirect, AppError> {
    app.todos.update(id, input).await?;

    Ok(
        Redirect::back_or(format!("/todos/{id}"))
            .flash("toast", Toast::success("Todo updated")),
    )
}
```

## Delete handler

```rust
async fn destroy(
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Redirect, AppError> {
    app.todos.delete(id).await?;

    Ok(
        Redirect::to("/todos")
            .flash("toast", Toast::success("Todo deleted")),
    )
}
```

## Complete router

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = AppState {
        todos: TodoRepository::connect().await?,
    };

    let app_key = load_app_key()?;

    let inertia = Inertia::vite("frontend")
        .share(AppShare)
        .transient(CookieTransient::encrypted(app_key))
        .build()?;

    let app = Router::new()
        .route("/todos", get(index).post(store))
        .route("/todos/{id}", get(show).patch(update).delete(destroy))
        .with_state(state)
        .inertia(inertia);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

## Todo tests

```rust
#[tokio::test]
async fn partial_stats_request_does_not_load_todos() {
    let repository = FakeTodoRepository::with_todos([
        TodoDto::new("Write design"),
        TodoDto::new("Implement layer"),
    ]);

    let app = TestApp::new(test_app(repository).await);

    let page = app
        .inertia_get("/todos")
        .only(TodosIndexPage::STATS)
        .send()
        .await
        .assert_page::<TodosIndexPage>();

    let stats = page.prop(TodosIndexPage::STATS);

    assert_eq!(stats.total, 2);
    page.assert_missing(TodosIndexPage::TODOS);
    page.assert_missing(TodosIndexPage::ARCHIVED);
}
```

```rust
#[tokio::test]
async fn archived_is_only_loaded_when_explicitly_requested() {
    let repository = SpyTodoRepository::default();
    let calls = repository.archived_calls();

    let app = TestApp::new(test_app(repository).await);

    app.inertia_get("/todos")
        .send()
        .await
        .assert_page::<TodosIndexPage>();

    assert_eq!(calls.load(Ordering::SeqCst), 0);

    app.inertia_get("/todos")
        .only(TodosIndexPage::ARCHIVED)
        .send()
        .await
        .assert_page::<TodosIndexPage>();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
```

---

# 23. Unique example: factory incident command board

This example exercises deferred groups, rescued props, optional diagnostics, once props, merging, infinite scroll, validation, flash, history encryption, and external locations.

## Page

```rust
#[derive(InertiaPage)]
#[inertia(
    component = "Incidents/Show",
    rename_all = "camelCase",
    encrypt_history
)]
struct IncidentShowPage {
    incident: IncidentDto,

    timeline: Prop<ScrollPage<IncidentEventDto>>,

    telemetry: Prop<TelemetrySummary>,

    affected_machines: Prop<Vec<MachineSummary>>,

    raw_controller_payloads: Prop<Vec<ControllerPayload>>,

    playbooks: Prop<Vec<PlaybookSummary>>,

    participants: Prop<Vec<IncidentParticipant>>,
}
```

## Handler

```rust
async fn show_incident(
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<TimelineQuery>,
) -> Result<IncidentShowPage, AppError> {
    let incident = app
        .incidents
        .find(id)
        .await?
        .ok_or(AppError::IncidentNotFound)?;

    let timeline = app
        .incidents
        .timeline(id, query.after.clone())
        .await?;

    let participants = app
        .incidents
        .participants(id)
        .await?;

    Ok(IncidentShowPage {
        incident,

        timeline: scroll(timeline)
            .page_name("timeline")
            .match_on("id"),

        telemetry: defer({
            let telemetry = app.telemetry.clone();

            move || async move {
                telemetry.incident_summary(id).await
            }
        })
        .group("telemetry")
        .rescue(),

        affected_machines: defer({
            let machines = app.machines.clone();

            move || async move {
                machines.affected_by_incident(id).await
            }
        })
        .group("telemetry"),

        raw_controller_payloads: optional({
            let telemetry = app.telemetry.clone();

            move || async move {
                telemetry.raw_payloads(id).await
            }
        }),

        playbooks: once({
            let playbooks = app.playbooks.clone();

            move || async move {
                playbooks.available_for_incident(id).await
            }
        })
        .key("incident-playbooks:v3")
        .expires_in(Duration::from_secs(60 * 60)),

        participants: merge(participants)
            .append()
            .match_on("id"),
    })
}
```

## Creation form

```rust
#[derive(Debug, Deserialize, garde::Validate, InertiaForm)]
#[inertia(
    validator = "garde",
    error_bag = "createIncident"
)]
struct CreateIncident {
    #[garde(length(min = 3, max = 120))]
    title: String,

    plant_id: Uuid,
    severity: Severity,

    #[garde(length(max = 4_000))]
    summary: String,

    affected_machine_ids: Vec<Uuid>,
}
```

## Create handler

```rust
async fn create_incident(
    State(app): State<AppState>,
    CurrentUser(user): CurrentUser,
    Validated(input): Validated<CreateIncident>,
) -> Result<Redirect, AppError> {
    let incident = app
        .incidents
        .create(user.id, input)
        .await?;

    Ok(
        Redirect::to(format!("/incidents/{}", incident.id))
            .flash(
                "toast",
                Toast::warning("Incident command room created"),
            )
            .flash("incidentId", incident.id),
    )
}
```

## Acknowledge handler

```rust
async fn acknowledge_incident(
    State(app): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Redirect, AppError> {
    app.incidents.acknowledge(id, user.id).await?;

    Ok(
        Redirect::back_or(format!("/incidents/{id}"))
            .flash("toast", Toast::success("Incident acknowledged")),
    )
}
```

## External maintenance portal

```rust
async fn maintenance_portal(
    State(app): State<AppState>,
    Path(machine_id): Path<Uuid>,
) -> Result<Location, AppError> {
    let url = app
        .maintenance
        .portal_url(machine_id)
        .await?;

    Ok(Location::external(url))
}
```

## Router

```rust
let app = Router::new()
    .route("/incidents", post(create_incident))
    .route("/incidents/{id}", get(show_incident))
    .route(
        "/incidents/{id}/acknowledge",
        post(acknowledge_incident),
    )
    .route(
        "/machines/{machine_id}/maintenance",
        get(maintenance_portal),
    )
    .with_state(state)
    .inertia(inertia);
```

## Selective-loading test

```rust
#[tokio::test]
async fn raw_payloads_are_not_loaded_during_normal_visits() {
    let telemetry = SpyTelemetryRepository::default();
    let calls = telemetry.raw_payload_calls();

    let app = TestApp::new(
        test_app()
            .telemetry(telemetry)
            .build()
            .await,
    );

    app.inertia_get("/incidents/8bb4d92b-5b8d-49f8-b99f-1dc8043f73e5")
        .send()
        .await
        .assert_page::<IncidentShowPage>();

    assert_eq!(calls.load(Ordering::SeqCst), 0);

    app.inertia_get("/incidents/8bb4d92b-5b8d-49f8-b99f-1dc8043f73e5")
        .only(IncidentShowPage::RAW_CONTROLLER_PAYLOADS)
        .send()
        .await
        .assert_page::<IncidentShowPage>();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
```

## Rescued telemetry test

```rust
#[tokio::test]
async fn unavailable_telemetry_is_rescued() {
    let app = TestApp::new(
        test_app()
            .telemetry(FailingTelemetryRepository)
            .build()
            .await,
    );

    let page = app
        .inertia_get("/incidents/8bb4d92b-5b8d-49f8-b99f-1dc8043f73e5")
        .only(IncidentShowPage::TELEMETRY)
        .send()
        .await
        .assert_page::<IncidentShowPage>();

    page.assert_missing(IncidentShowPage::TELEMETRY);
    page.assert_rescued(IncidentShowPage::TELEMETRY);
}
```

---

# 24. Features intentionally excluded from the first design

The following should not be part of the initial public API:

| Excluded feature                                  | Reason                                                     |
| ------------------------------------------------- | ---------------------------------------------------------- |
| Controller and route macros                       | Duplicate Axum and obscure routing                         |
| `#[inertia::main]`                                | Hides normal Tokio/Axum startup                            |
| Global `Inertia::share()` state                   | Introduces hidden process-wide state                       |
| Per-route HTML callbacks                          | Recreates the current boilerplate                          |
| Multiple string-keyed shared providers            | Makes collisions and execution order harder to understand  |
| Automatic ORM integration                         | Couples protocol concerns to persistence                   |
| Frontend framework-specific Rust features         | Vite and the Inertia protocol are framework-neutral        |
| TypeScript generation during proc-macro expansion | Proc macros should not mutate project files                |
| Mandatory CLI                                     | Library use must remain ordinary Rust                      |
| Mandatory session dependency                      | Pages without validation or flash do not need one          |
| Separate protocol crate                           | Premature for an Axum-only adapter                         |
| SSR in the initial DX redesign                    | Large independent concern with a separate runtime boundary |

---

# 25. Implementation architecture

The runtime should be divided into a pure engine and a thin Tower layer.

## Layer responsibilities

```rust
pub struct InertiaLayer {
    app: Inertia,
}
```

The layer:

1. Parses the request into `Visit`.
2. Performs pre-handler asset-version conflict checks.
3. Loads transient state.
4. Inserts request context.
5. Calls the Axum handler.
6. Detects a pending Inertia response.
7. Passes the response and request context to the engine.
8. Commits transient state.
9. Returns the finalized Axum response.

## Engine responsibilities

```rust
struct Engine {
    config: Inertia,
}
```

The engine should expose internal functions close to:

```rust
async fn finalize_page(
    &self,
    visit: &Visit,
    page: PendingPage,
    transient: &mut TransientData,
) -> Result<Response, Error>;

async fn finalize_redirect(
    &self,
    visit: &Visit,
    redirect: PendingRedirect,
    transient: &mut TransientData,
) -> Result<Response, Error>;

async fn resolve_props(
    &self,
    visit: &Visit,
    page: &PendingPage,
) -> Result<ResolvedProps, Error>;
```

The engine should have no Axum extractor logic and minimal Tower-specific code. This permits exhaustive unit tests against synthetic visits.

## Prop resolution order

For a page response:

1. Determine the component.
2. Determine matching partial-reload behavior.
3. Select eligible route props.
4. Select eligible shared props.
5. Apply once-prop exclusions.
6. Do not construct unselected resolver futures.
7. Resolve selected asynchronous props concurrently.
8. Record rescued failures.
9. Add validation errors.
10. Add flash data separately.
11. Calculate response metadata.
12. Build the page object.
13. Return JSON or root HTML.

Resolver selection must happen before future construction. This is essential for predictable performance and testability.

---

# 26. Implementation sequence

## Phase 1: response finalization

Implement:

```rust
Inertia
RouterInertiaExt
Page
page!()
Redirect
Location
PendingResponse
InertiaLayer
RootView
```

Acceptance example:

```rust
async fn home() -> Page {
    page!("Home", {
        message: "Hello",
    })
}
```

## Phase 2: Vite conventions

Implement:

```rust
Inertia::vite
Vite manifest loading
asset tags
asset versioning
static file service
default root document
```

Acceptance setup:

```rust
.inertia(Inertia::vite("frontend").build()?)
```

## Phase 3: unified prop engine

Implement:

```rust
Prop<T>
lazy
always
optional
defer
once
merge
scroll
partial filtering
async resolver selection
rescued props
```

## Phase 4: proc macros

Implement:

```rust
InertiaProps
InertiaPage
typed PropKey<T>
diagnostics
```

## Phase 5: shared data

Implement:

```rust
Share
ShareContext
shared prop resolution
collision rules
shared once props
```

## Phase 6: transient state

Implement:

```rust
TransientStore
MemoryTransient
CookieTransient
flash
errors
```

## Phase 7: validation

Implement:

```rust
InertiaForm
Validated<T>
InertiaForm derive
garde adapter
validator adapter
error bags
redirect-back behavior
```

## Phase 8: test crate

Implement:

```rust
TestApp
request builder
cookie persistence
typed page assertions
partial reloads
validation assertions
flash assertions
version assertions
```

## Phase 9: CLI

Implement only:

```text
cargo inertia init
cargo inertia dev
cargo inertia check
```

---

# 27. Definition of done

The DX redesign is complete when all of these examples compile and behave as shown:

```rust
Router::new()
    .route("/", get(home))
    .inertia(Inertia::vite("frontend").build()?);
```

```rust
async fn home() -> Page {
    page!("Home", {
        greeting: "Hello",
    })
}
```

```rust
async fn index() -> Result<TodosIndexPage, AppError> {
    Ok(TodosIndexPage {
        todos,
        filters,
        stats: defer(load_stats),
    })
}
```

```rust
async fn store(
    Validated(input): Validated<CreateTodo>,
) -> Result<Redirect, AppError> {
    save(input).await?;

    Ok(
        Redirect::to("/todos")
            .flash("toast", Toast::success("Todo created")),
    )
}
```

```rust
let page = app
    .inertia_get("/todos")
    .only(TodosIndexPage::STATS)
    .send()
    .await
    .assert_page::<TodosIndexPage>();

let stats = page.prop(TodosIndexPage::STATS);
```

The behavioral requirements are:

1. A standard page handler needs no Inertia extractor.
2. A standard page handler never constructs an Axum `Response`.
3. HTML rendering is configured once.
4. Vite configuration is loaded once.
5. Shared props are supplied by one typed provider.
6. Flash and validation errors are handled without manual shared props.
7. An unselected resolver is never constructed or polled.
8. Deferred groups resolve concurrently and independently.
9. Ordinary Axum responses pass through unchanged.
10. Missing setup produces actionable errors.
11. Every macro has compile-pass and compile-fail coverage.
12. The two full examples compile in workspace CI.
13. The public prelude remains small.
14. Advanced protocol access remains available without contaminating the common path.
15. The README can teach the complete common workflow without introducing protocol headers, Tower services, HTML serialization, or Vite manifest parsing.

[1]: https://inertiajs.com/deferred-props "https://inertiajs.com/deferred-props"
[2]: https://inertiajs.com/merging-props "https://inertiajs.com/merging-props"
[3]: https://inertiajs.com/docs/v3/data-props/once-props "https://inertiajs.com/docs/v3/data-props/once-props"
[4]: https://inertiajs.com/docs/v3/data-props/infinite-scroll "https://inertiajs.com/docs/v3/data-props/infinite-scroll"
[5]: https://inertiajs.com/docs/v3/data-props/shared-data "https://inertiajs.com/docs/v3/data-props/shared-data"
[6]: https://inertiajs.com/docs/v3/data-props/flash-data "https://inertiajs.com/docs/v3/data-props/flash-data"
[7]: https://inertiajs.com/docs/v3/the-basics/validation "https://inertiajs.com/docs/v3/the-basics/validation"
[8]: https://doc.rust-lang.org/reference/procedural-macros.html "https://doc.rust-lang.org/reference/procedural-macros.html"
[9]: https://doc.rust-lang.org/cargo/reference/workspaces.html "https://doc.rust-lang.org/cargo/reference/workspaces.html"


# `inertia-axum` DX source of truth

Everything below is a proposed API and repository design. It is intended to be the canonical implementation target rather than a collection of competing alternatives.

The current `0.9.0` API revolves around an `Inertia` extractor, `InertiaConfig` stored in Axum state, and `i.render("Component", props)` inside every page handler. Applications with their own state must also implement `FromRef<AppState> for InertiaConfig`. ([Docs.rs][1]) ([Docs.rs][1]) The current render path serializes props with `expect("serialization failure")`, while request parsing currently models only the core Inertia flag, asset version, URL, and basic partial-data/component headers. ([Docs.rs][2]) ([GitHub][3])

The target should be Inertia 3.x, whose protocol now includes partial `only` and `except`, reset props, error bags, infinite-scroll intent, once-prop exclusions, deferred and rescued props, merge metadata, shared props, flash data, and history flags. Version mismatches belong in middleware before handlers execute, and write redirects must become `303 See Other`. ([Inertia.js][4]) ([Inertia.js][4]) ([Inertia.js][4])

---

# 1. The target experience

This should be the normal application setup:

```rust
use axum::{Router, routing::get};
use axum_inertia::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = AppState::connect().await?;

    let app = Router::new()
        .merge(todos::router())
        .with_state(state.clone())
        .inertia(
            InertiaApp::vite()
                .title("Tasks")
                .share(AppShared::new(state))
                .flash(Flash::signed_cookie(
                    std::env::var("APP_KEY")?,
                )),
        )?;

    let listener =
        tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    axum::serve(listener, app).await?;

    Ok(())
}
```

A page handler should look like ordinary Axum:

```rust
async fn index(
    State(app): State<AppState>,
    Query(filters): Query<TodoFilters>,
) -> Result<TodosIndexPage, AppError> {
    let todos = app.todos.search(&filters).await?;

    Ok(TodosIndexPage {
        todos,
        filters,
        stats: Deferred::new({
            let todos = app.todos.clone();

            move || async move {
                todos.stats().await
            }
        })
        .group("summary"),

        archived: Optional::new({
            let todos = app.todos.clone();

            move || async move {
                todos.archived().await
            }
        }),
    })
}
```

The page declaration should be the only place that mentions the frontend component name:

```rust
#[derive(InertiaPage)]
#[inertia(component = "Todos/Index")]
struct TodosIndexPage {
    todos: Vec<TodoDto>,
    filters: TodoFilters,
    stats: Deferred<TodoStats>,
    archived: Optional<Vec<TodoDto>>,
}
```

The normal path must require:

* No `Inertia` extractor.
* No `InertiaConfig` inside application state.
* No `FromRef` implementation.
* No `render()` call.
* No raw Axum `Response`.
* No HTML-rendering closure in handlers.
* No repeated component strings.
* No hand-written partial-reload filtering.

---

# 2. Non-negotiable design rules

## One canonical setup path

The public setup API is:

```rust
Router::new()
    .with_state(state)
    .inertia(InertiaApp::vite())?;
```

`InertiaApp` owns all Inertia-specific application configuration. It must not become part of the user’s Axum state.

## Page handlers return page values

Handlers return an `InertiaPage`-derived struct directly:

```rust
async fn show(...) -> Result<UserShowPage, AppError>
```

The adapter layer decides whether that becomes:

* Initial HTML.
* Inertia JSON.
* A partial reload.
* A deferred response.
* A version conflict.

## Wrapper types describe execution policy

Protocol behavior should primarily be represented by field types:

```rust
users: Lazy<Vec<UserDto>>,
audit: Optional<Vec<AuditEntry>>,
stats: Deferred<Stats>,
plans: Once<Vec<PlanDto>>,
events: Scroll<EventDto>,
```

Attributes should be reserved for metadata that cannot naturally be represented by the field type:

```rust
#[inertia(always)]
csrf_token: String,

#[inertia(merge, match_on = "id")]
notifications: Vec<NotificationDto>,
```

## Macros remove Inertia boilerplate, not Axum boilerplate

The crate should provide:

```rust
#[derive(InertiaPage)]
#[derive(InertiaProps)]
#[derive(InertiaForm)]
page!(...)
```

It should not replace Axum routing with a controller DSL. Axum routing is already concise:

```rust
Router::new()
    .route("/todos", get(index).post(store))
    .route("/todos/{id}", patch(update).delete(destroy))
```

## Protocol details live in one layer

Request parsing, version checks, prop selection, shared props, flash handling, HTML rendering, JSON rendering, redirects, and response headers belong in `InertiaLayer`.

## No user-controlled failure may panic

Serialization errors, invalid manifest files, resolver failures, invalid redirects, shared-prop failures, and root-template failures must return typed errors. The replacement for the current serialization `expect` is an application-configurable error handler.

## Tests use the Router in-process

Tests should use `tower::ServiceExt::oneshot` through a purpose-built test client. No TCP listeners, ports, Vite processes, or `reqwest` should be required. The current repository tests start real ephemeral listeners, which should no longer be necessary. ([Docs.rs][2])

---

# 3. Package installation

```toml
[dependencies]
axum = "0.8"
inertia-axum = {
    version = "1",
    features = [
        "vite",
        "flash-cookie",
        "garde",
    ],
}
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }

[dev-dependencies]
inertia-axum-testing = "1"
```

Macros and Vite support should be enabled by default:

```toml
[features]
default = ["macros", "vite"]

macros = ["dep:inertia-axum-macros"]
vite = ["dep:tower-http", "dep:sha2"]

flash-cookie = [
    "dep:cookie",
    "dep:hmac",
    "dep:base64",
]

garde = ["dep:garde"]
validator = ["dep:validator"]
```

The common import should be:

```rust
use axum_inertia::prelude::*;
```

The prelude should contain only normal application-facing types:

```rust
pub mod prelude {
    pub use crate::{
        page,
        Deferred,
        Flash,
        InertiaApp,
        InertiaForm,
        InertiaPage,
        InertiaProps,
        InertiaResult,
        Lazy,
        Location,
        Once,
        Optional,
        Redirect,
        RouterInertiaExt,
        Scroll,
        SharedContext,
        SharedProps,
        Validated,
    };
}
```

Raw protocol types should remain outside the prelude:

```rust
use axum_inertia::advanced::{
    PageMetadata,
    RequestContext,
    PartialRequest,
    PropKey,
};
```

---

# 4. `InertiaApp`: one application-level object

## Public shape

```rust
#[derive(Clone)]
pub struct InertiaApp {
    runtime: Arc<Runtime>,
}

pub struct InertiaAppBuilder<A = ViteAssets> {
    assets: A,
    root: Box<dyn RootTemplate>,
    shared: Vec<Box<dyn SharedPropsProvider>>,
    flash: Option<Box<dyn FlashStore>>,
    error_handler: Box<dyn InertiaErrorHandler>,
    config: InertiaConfig,
}

pub trait IntoInertiaApp {
    fn build(self) -> Result<InertiaApp, InitError>;
}
```

Router integration:

```rust
pub trait RouterInertiaExt<S> {
    fn inertia<A>(
        self,
        app: A,
    ) -> Result<axum::Router<S>, InitError>
    where
        A: IntoInertiaApp;
}
```

The layer should be installed by:

```rust
let app = router.inertia(InertiaApp::vite())?;
```

Not by:

```rust
router
    .layer(InertiaLayer::new(...))
    .layer(Extension(...))
    .with_state(InertiaConfig { ... })
```

## Constructors

```rust
impl InertiaApp {
    pub fn vite() -> InertiaAppBuilder<ViteAssets>;

    pub fn testing() -> InertiaAppBuilder<TestAssets>;

    pub fn custom<A>(
        assets: A,
    ) -> InertiaAppBuilder<A>
    where
        A: AssetSource;
}
```

---

# 5. Convention-based Vite setup

The current Vite API has separate development and production builders and expects the application to branch between them. ([Docs.rs][5]) The new default should be one configuration.

```rust
InertiaApp::vite()
```

Default layout:

```text
frontend/
├── src/
│   ├── main.ts
│   └── Pages/
└── dist/
    └── .vite/
        └── manifest.json
```

Default values:

```rust
ViteAssets {
    frontend: "frontend",
    entry: "src/main.ts",
    build_dir: "dist",
    manifest: "dist/.vite/manifest.json",
    public_path: "/build",
    root_id: "app",
    mode: ViteMode::Auto,
}
```

## Automatic mode

`ViteMode::Auto` should have deterministic behavior:

```text
INERTIA_ENV=development → development server
INERTIA_ENV=production  → production manifest
INERTIA_ENV=test        → test assets
otherwise               → debug build means development,
                          release build means production
```

Development behavior:

* Use `VITE_DEV_SERVER_URL`, defaulting to `http://localhost:5173`.
* Inject `@vite/client`.
* Inject the configured entrypoint.
* Preserve HMR.
* Add the React refresh preamble only when `.react()` is configured.
* Do not read a production manifest.

Production behavior:

* Load the manifest once at startup.
* Resolve imported chunks and CSS.
* Generate all script and stylesheet tags.
* Compute the asset version from the resolved manifest graph.
* Serve the build directory under `public_path`.
* Fail at startup when the manifest or entrypoint is invalid.

Testing behavior:

* Never read the filesystem.
* Never contact a Vite server.
* Use a deterministic version such as `"test"`.
* Render a minimal valid HTML shell.

## Custom configuration

```rust
let inertia = InertiaApp::vite()
    .frontend("web")
    .entry("src/app.tsx")
    .build_dir("public/build")
    .manifest("public/build/.vite/manifest.json")
    .public_path("/assets")
    .react()
    .title("Operations");
```

## Startup diagnostics

A missing entry should result in:

```text
inertia-axum Vite initialization failed

Entry:
    src/app.tsx

was not found in:
    web/public/build/.vite/manifest.json

Available entries:
    src/admin.tsx
    src/main.tsx
```

No Vite error should first appear during an HTTP request.

## Asset abstraction

```rust
pub trait AssetSource: Clone + Send + Sync + 'static {
    fn initialize(
        self,
    ) -> Result<AssetRuntime, InitError>;
}

pub struct AssetRuntime {
    pub version: Option<String>,
    pub tags: AssetTags,
    pub static_service: Option<StaticAssetService>,
}

pub struct AssetTags {
    scripts: Vec<AssetScript>,
    styles: Vec<AssetStyle>,
}
```

This allows non-Vite integrations without affecting the normal API:

```rust
InertiaApp::custom(MyAssetSource)
```

---

# 6. Built-in root HTML

A standard Vite application should not need to supply an HTML closure.

```rust
InertiaApp::vite()
    .title("Tasks")
    .lang("en")
    .root_id("app")
```

The built-in root should render:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Tasks</title>
    <!-- resolved Vite assets -->
  </head>
  <body>
    <script data-page="app" type="application/json">
      <!-- script-safe page JSON -->
    </script>
    <div id="app"></div>
  </body>
</html>
```

The crate must handle script-safe JSON escaping internally. Custom templates must receive an opaque safe value rather than an arbitrary raw JSON string.

## Root-template trait

```rust
pub trait RootTemplate:
    Clone + Send + Sync + 'static
{
    fn render(
        &self,
        context: RootContext<'_>,
    ) -> Result<String, InertiaError>;
}

pub struct RootContext<'a> {
    pub title: &'a str,
    pub lang: &'a str,
    pub root_id: &'a str,
    pub assets: &'a AssetTags,
    pub page: &'a ScriptSafePageJson,
    pub nonce: Option<&'a str>,
}
```

Custom root:

```rust
#[derive(Clone)]
struct AppRoot;

impl RootTemplate for AppRoot {
    fn render(
        &self,
        context: RootContext<'_>,
    ) -> Result<String, InertiaError> {
        Ok(format!(
            r#"<!doctype html>
<html lang="{lang}">
<head>
    <meta charset="utf-8">
    <meta name="theme-color" content="#111827">
    {assets}
</head>
<body>
    <script data-page="app" type="application/json">{page}</script>
    <main id="{root_id}"></main>
</body>
</html>"#,
            lang = context.lang,
            assets = context.assets,
            page = context.page,
            root_id = context.root_id,
        ))
    }
}
```

Registration:

```rust
InertiaApp::vite()
    .root(AppRoot)
```

---

# 7. Direct typed page responses

## Core traits

```rust
pub trait IntoInertiaProps:
    Send + 'static
{
    fn into_props(self) -> Props;
}

pub trait InertiaPage:
    IntoInertiaProps + Send + 'static
{
    const COMPONENT: &'static str;

    fn metadata() -> PageMetadata {
        PageMetadata::default()
    }
}
```

Internal prop container:

```rust
pub struct Props {
    entries: Vec<PropEntry>,
}

impl Props {
    pub fn new() -> Self;

    pub fn insert<V>(
        self,
        name: &'static str,
        value: V,
    ) -> Self
    where
        V: IntoProp;
}
```

The page derive generates:

```rust
impl IntoInertiaProps for TodosIndexPage {
    fn into_props(self) -> Props {
        Props::new()
            .insert("todos", self.todos)
            .insert("filters", self.filters)
            .insert("stats", self.stats)
            .insert("archived", self.archived)
    }
}

impl InertiaPage for TodosIndexPage {
    const COMPONENT: &'static str = "Todos/Index";
}

impl axum::response::IntoResponse for TodosIndexPage {
    fn into_response(self) -> axum::response::Response {
        PendingPage::new(
            Self::COMPONENT,
            self.into_props(),
            Self::metadata(),
        )
        .into_response()
    }
}
```

Users never write these implementations.

## Why a pending page is required

`IntoResponse` is synchronous, but async props and shared props must be resolved after the handler returns. A page response therefore initially contains an internal `PendingPage` marker.

Conceptually:

```rust
pub struct PendingPage {
    component: &'static str,
    props: Props,
    metadata: PageMetadata,
}
```

`InertiaLayer` recognizes the marker, resolves it asynchronously, and replaces it with the final response.

A pending page that escapes without the layer should return a useful diagnostic:

```text
An Inertia page was returned without InertiaApp being installed.

Install it on the router:

Router::new()
    .route("/", get(index))
    .inertia(InertiaApp::vite())?
```

---

# 8. `InertiaLayer` runtime contract

The layer should perform this sequence for every request.

1. Parse the full Inertia 3 request context.
2. Check asset versions before calling the handler.
3. Capture method, original URI, headers, and relevant extensions.
4. Execute the Axum handler.
5. Pass ordinary non-Inertia responses through unchanged.
6. Detect pending page, redirect, external location, validation, and flash responses.
7. Select required props before creating or polling resolver futures.
8. Resolve selected page and shared props concurrently.
9. Add `errors: {}` when no validation errors are present.
10. Merge route props over shared props.
11. Attach flash data to `page.flash`.
12. Generate all page metadata.
13. Return JSON or initial HTML.
14. Commit or clear flash storage.
15. Add protocol headers, including `Vary: X-Inertia`.

## Full request context

```rust
#[derive(Clone, Debug, Default)]
pub struct RequestContext {
    pub is_inertia: bool,
    pub version: Option<String>,

    pub partial: PartialRequest,
    pub reset: BTreeSet<PropName>,

    pub error_bag: Option<String>,
    pub scroll_intent: Option<MergeIntent>,
    pub except_once: BTreeSet<OnceKey>,

    pub prefetch: bool,
    pub reload: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PartialRequest {
    pub component: Option<String>,
    pub only: BTreeSet<PropName>,
    pub except: BTreeSet<PropName>,
}
```

Rules:

* `except` takes precedence over `only`.
* Partial selection applies only when the requested component matches.
* `errors` is always included.
* Partial metadata is stripped when its corresponding prop is omitted.
* Explicit partial requests refresh once props.
* Non-GET requests do not apply normal partial filtering.
* `X-Inertia-Except-Once-Props` still applies independently.
* Asset conflicts happen before the handler.
* Flash data is reflashed after asset conflicts.

## Response metadata

```rust
#[derive(Clone, Debug, Default)]
pub struct PageMetadata {
    pub encrypt_history: bool,
    pub clear_history: bool,
    pub preserve_fragment: bool,

    pub merge_props: Vec<PropPath>,
    pub prepend_props: Vec<PropPath>,
    pub deep_merge_props: Vec<PropPath>,
    pub match_props_on: Vec<PropPath>,

    pub scroll_props: BTreeMap<PropName, ScrollMetadata>,
    pub deferred_props: BTreeMap<GroupName, Vec<PropName>>,
    pub rescued_props: Vec<PropName>,
    pub shared_props: Vec<PropName>,
    pub once_props: BTreeMap<OnceKey, OnceMetadata>,
}
```

---

# 9. The `InertiaPage` proc macro

## Basic page

```rust
#[derive(InertiaPage)]
#[inertia(component = "Users/Index")]
struct UsersIndexPage {
    users: Vec<UserDto>,
}
```

## Renamed page props

```rust
#[derive(InertiaPage)]
#[inertia(
    component = "Account/Settings",
    rename_all = "camelCase",
)]
struct SettingsPage {
    account_owner: UserDto,
    security_events: Vec<SecurityEventDto>,
}
```

Produces:

```json
{
  "accountOwner": {},
  "securityEvents": []
}
```

## History metadata

```rust
#[derive(InertiaPage)]
#[inertia(
    component = "Checkout/Payment",
    encrypt_history,
)]
struct PaymentPage {
    order: OrderDto,
}
```

Other page-level flags:

```rust
#[inertia(
    component = "Auth/Login",
    clear_history,
    preserve_fragment,
)]
```

## Field attributes

Always included:

```rust
#[inertia(always)]
csrf_token: String,
```

Append merge:

```rust
#[inertia(merge, match_on = "id")]
notifications: Vec<NotificationDto>,
```

Prepend merge:

```rust
#[inertia(prepend, match_on = "id")]
new_events: Vec<EventDto>,
```

Deep merge:

```rust
#[inertia(deep_merge, match_on = "data.id")]
conversations: ConversationCollection,
```

Explicit prop rename:

```rust
#[inertia(rename = "currentUser")]
user: UserDto,
```

## Generated typed prop keys

The derive should generate typed associated constants:

```rust
impl TodosIndexPage {
    pub const TODOS: PropKey<Self> =
        PropKey::new("todos");

    pub const FILTERS: PropKey<Self> =
        PropKey::new("filters");

    pub const STATS: PropKey<Self> =
        PropKey::new("stats");

    pub const ARCHIVED: PropKey<Self> =
        PropKey::new("archived");
}
```

These are used in tests:

```rust
page.has(TodosIndexPage::TODOS);
page.missing(TodosIndexPage::ARCHIVED);
```

And partial reload helpers:

```rust
page
    .reload_only([TodosIndexPage::ARCHIVED])
    .await;
```

The component name and root prop names should therefore appear as raw strings only in the page declaration.

## Compile-time validation

The macro should reject:

```rust
#[inertia(always)]
stats: Deferred<Stats>,
```

Diagnostic:

```text
error: deferred props cannot also be marked `always`
```

It should also reject:

* Empty component names.
* Duplicate serialized prop names.
* Multiple incompatible merge modes.
* `match_on` without a merge mode.
* Reserved names used incorrectly.
* Unsupported wrapper combinations.
* Duplicate once keys declared statically.
* Invalid rename syntax.

Compile errors should point at the field or attribute that caused the problem.

---

# 10. Prop execution types

| Field type    |                 Initial visit |                 Matching partial reload | Metadata        |
| ------------- | ----------------------------: | --------------------------------------: | --------------- |
| `T`           | Already computed and included |           Serialized only when selected | None            |
| `Lazy<T>`     |                      Resolved |             Resolved only when selected | None            |
| `Optional<T>` |                       Omitted | Resolved only when explicitly requested | None            |
| `Deferred<T>` |                       Omitted |      Resolved when explicitly requested | `deferredProps` |
| `Once<T>`     |    Resolved unless remembered |        Explicit request always resolves | `onceProps`     |
| `Scroll<T>`   |                      Included |     Included with append/prepend intent | `scrollProps`   |
| `Merge<T>`    |                      Included |                  Merge behavior applied | Merge metadata  |

## Common resolver representation

```rust
type Resolver<T> = Box<
    dyn FnOnce() -> BoxFuture<'static, InertiaResult<T>>
        + Send
>;

pub struct Lazy<T> {
    resolver: Resolver<T>,
}

pub struct Optional<T> {
    resolver: Resolver<T>,
    once: Option<OncePolicy>,
}

pub struct Deferred<T> {
    resolver: Resolver<T>,
    group: Cow<'static, str>,
    rescue: bool,
    once: Option<OncePolicy>,
}

pub struct Once<T> {
    resolver: Resolver<T>,
    policy: OncePolicy,
}
```

Constructors should accept callbacks returning either `T` or `Result<T, E>` where `E` can become an `InertiaError`.

## Lazy prop

```rust
users: Lazy::new({
    let users = app.users.clone();

    move || async move {
        users.all().await
    }
})
```

It runs on:

* Initial visits.
* Full reloads.
* Matching partial reloads that include it.

It does not run when a matching partial reload excludes it.

## Optional prop

```rust
audit_log: Optional::new({
    let audit = app.audit.clone();

    move || async move {
        audit.for_account(account_id).await
    }
})
```

It runs only when explicitly requested:

```ts
router.reload({ only: ['auditLog'] })
```

## Deferred prop

```rust
stats: Deferred::new({
    let analytics = app.analytics.clone();

    move || async move {
        analytics.dashboard().await
    }
})
```

Grouped:

```rust
stats: Deferred::new(load_stats)
    .group("analytics")
```

Rescued:

```rust
stats: Deferred::new(load_stats)
    .group("analytics")
    .rescue()
```

A rescued failure should:

* Report the original error through the error reporter.
* Omit the failed prop.
* Add the key to `rescuedProps`.
* Allow the remaining response to succeed.

Deferred once prop:

```rust
stats: Deferred::new(load_stats)
    .group("analytics")
    .once()
    .ttl(Duration::from_secs(300))
```

## Once prop

The default once key should be the field’s serialized name:

```rust
plans: Once::new({
    let billing = app.billing.clone();

    move || async move {
        billing.plans().await
    }
})
```

Custom shared key:

```rust
plans: Once::new(load_plans)
    .key("billing-plans:v3")
```

Expiration:

```rust
exchange_rates: Once::new(load_rates)
    .ttl(Duration::from_secs(60 * 60))
```

Conditional refresh:

```rust
permissions: Once::new(load_permissions)
    .fresh_if(user.permissions_changed)
```

The official once-prop behavior includes remembered client values, explicit refreshes, expiration, custom keys, and composition with deferred and optional props. ([Inertia.js][6]) ([Inertia.js][6])

## Infinite scroll

```rust
events: Scroll::new(page.items)
    .page_name("events_page")
    .current(page.current)
    .previous(page.previous)
    .next(page.next)
```

Simplified type:

```rust
pub struct Scroll<T> {
    data: T,
    page_name: Cow<'static, str>,
    current: u64,
    previous: Option<u64>,
    next: Option<u64>,
}
```

`Scroll<T>` should automatically:

* Serialize its data.
* Add the correct `scrollProps` entry.
* Mark `<field>.data` for append merging.
* Switch to prepend metadata when the request carries prepend intent.
* Remove stale scroll metadata after reset.

## Advanced merge value

For advanced nested merge operations:

```rust
feed: Merge::new(feed)
    .append("posts")
    .prepend("notifications")
    .match_on("posts.id")
    .match_on("notifications.id")
```

This avoids trying to encode an arbitrarily complex merge graph in a proc-macro attribute.

## Concurrency rule

All selected async props should resolve concurrently.

Given:

```rust
struct DashboardPage {
    projects: Deferred<Vec<ProjectDto>>,
    usage: Deferred<UsageDto>,
    activity: Deferred<Vec<ActivityDto>>,
}
```

A request for all three should run all three resolvers concurrently.

A request for only `usage` must:

* Never invoke the `projects` resolver.
* Never invoke the `activity` resolver.
* Never construct their futures.
* Resolve only `usage`.

---

# 11. The `page!` regular macro

Typed page structs should be the main application API. `page!` is a compact escape hatch for small or one-off pages.

```rust
async fn home(
    State(app): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = app.current_user().await?;

    Ok(page!("Home", {
        user,
        message: "Welcome back",
    }))
}
```

With async props:

```rust
async fn dashboard(
    State(app): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    Ok(page!("Dashboard", {
        projects: Lazy::new({
            let app = app.clone();

            move || async move {
                app.projects.recent().await
            }
        }),

        stats: Deferred::new({
            let app = app.clone();

            move || async move {
                app.analytics.stats().await
            }
        }),
    }))
}
```

Metadata can use a small builder:

```rust
page!("Feed/Index", {
    posts,
    notifications,
})
.merge("posts")
.prepend("notifications")
.match_on("posts.id")
.match_on("notifications.id")
```

Nested dynamic props:

```rust
page!("Settings/Profile", {
    auth: {
        user,
        permissions,
    },
    settings: {
        locale,
        timezone,
    },
})
```

The macro should detect duplicate field names at compile time.

It should not grow into an alternate page language. Once a page needs several attributes or protocol modes, it should become an `InertiaPage` struct.

---

# 12. Typed shared props

Shared data should be resolved outside handlers and automatically merged into every page. This matches Inertia’s model where shared data is registered outside controllers and included with page props. ([Inertia.js][7])

## Shared output

```rust
#[derive(InertiaProps)]
#[inertia(rename_all = "camelCase")]
struct AppSharedProps {
    app_name: &'static str,
    auth: Option<AuthShared>,

    countries: Once<Vec<CountryDto>>,
}

#[derive(Serialize)]
struct AuthShared {
    user: UserSummary,
    permissions: Vec<String>,
}
```

## Provider trait

```rust
pub trait SharedProps:
    Clone + Send + Sync + 'static
{
    type Props: IntoInertiaProps;

    async fn share(
        &self,
        context: SharedContext,
    ) -> InertiaResult<Self::Props>;
}
```

Request context:

```rust
#[derive(Clone)]
pub struct SharedContext {
    pub method: Method,
    pub uri: Uri,
    pub headers: HeaderMap,
    pub extensions: Arc<Extensions>,
    pub inertia: RequestContext,
}

impl SharedContext {
    pub fn extension<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static;
}
```

## Provider implementation

```rust
#[derive(Clone)]
struct AppShared {
    countries: CountryRepository,
}

impl AppShared {
    fn new(state: AppState) -> Self {
        Self {
            countries: state.countries,
        }
    }
}

impl SharedProps for AppShared {
    type Props = AppSharedProps;

    async fn share(
        &self,
        context: SharedContext,
    ) -> InertiaResult<Self::Props> {
        let auth = context
            .extension::<CurrentUser>()
            .map(|current| AuthShared {
                user: current.user.summary(),
                permissions: current.permissions.clone(),
            });

        Ok(AppSharedProps {
            app_name: "Tasks",
            auth,

            countries: Once::new({
                let countries = self.countries.clone();

                move || async move {
                    countries.all().await
                }
            })
            .key("countries:v1")
            .ttl(Duration::from_secs(24 * 60 * 60)),
        })
    }
}
```

Registration:

```rust
InertiaApp::vite()
    .share(AppShared::new(state))
```

Rules:

* Route props win when a route and shared provider use the same root key.
* Shared props are resolved once per response.
* Shared keys are added to `sharedProps` metadata.
* Shared providers may use normal, lazy, and once props.
* Shared data should use nested structs rather than dotted string keys.
* Provider output is independently testable.

---

# 13. Flash data

Inertia 3 flash values belong on `page.flash`, are scoped to the request, and are not persisted in browser history. ([Inertia.js][8])

## Store abstraction

```rust
pub trait FlashStore:
    Clone + Send + Sync + 'static
{
    async fn take(
        &self,
        request: &RequestSnapshot,
    ) -> InertiaResult<FlashMap>;

    async fn persist(
        &self,
        response: &mut Response,
        flash: FlashMap,
    ) -> InertiaResult<()>;

    async fn reflash(
        &self,
        response: &mut Response,
        flash: FlashMap,
    ) -> InertiaResult<()>;
}
```

Built-ins:

```rust
Flash::signed_cookie(secret)
Flash::memory()
```

`Flash::memory()` is intended for tests.

## Redirect flash

```rust
Ok(
    Redirect::to("/todos")
        .flash("toast", Toast::success("Todo created")),
)
```

Convenience:

```rust
Ok(
    Redirect::to("/todos")
        .success("Todo created"),
)
```

Multiple values:

```rust
Ok(
    Redirect::to("/projects")
        .flash("message", "Project created")
        .flash("newProjectId", project.id),
)
```

## Page flash

A local extension trait should permit flash on any Inertia page response:

```rust
Ok(
    ProjectsIndexPage { projects }
        .flash("highlight", project_id),
)
```

Trait shape:

```rust
pub trait InertiaResponseExt:
    Sized
{
    fn flash<K, V>(
        self,
        key: K,
        value: V,
    ) -> Flashed<Self>
    where
        K: Into<String>,
        V: Serialize;
}
```

---

# 14. Redirects and external locations

Handlers should return redirect types directly.

## Internal redirects

```rust
async fn store(...) -> Result<Redirect, AppError> {
    Ok(Redirect::to("/todos"))
}
```

Back:

```rust
Ok(Redirect::back())
```

Back with fallback:

```rust
Ok(
    Redirect::back()
        .or("/todos"),
)
```

The layer chooses:

* `302 Found` for read requests.
* `303 See Other` after `POST`, `PUT`, `PATCH`, and `DELETE`.

## External locations

```rust
async fn billing_portal(
    State(app): State<AppState>,
) -> Result<Location, AppError> {
    let url = app.billing.portal_url().await?;

    Ok(Location::external(url))
}
```

The layer returns:

* `409 Conflict` plus `X-Inertia-Location` for an Inertia visit.
* `X-Inertia-Redirect` when a fragment must be preserved.
* An ordinary redirect for a direct browser visit.

## Types

```rust
pub struct Redirect {
    target: RedirectTarget,
    flash: FlashMap,
}

pub enum RedirectTarget {
    Uri(Uri),
    Back {
        fallback: Option<Uri>,
    },
}

pub struct Location {
    url: String,
}
```

URLs must be validated when the response is constructed, not by calling `unwrap()` while inserting headers.

---

# 15. Forms and validation

Inertia validation should redirect back with flashed errors rather than returning a normal `422` JSON response. The client determines validation failure by inspecting `page.props.errors`. ([Inertia.js][9])

## Form declaration

```rust
#[derive(
    Debug,
    Deserialize,
    Serialize,
    garde::Validate,
    InertiaForm,
)]
#[inertia(error_bag = "createTodo")]
struct CreateTodo {
    #[garde(length(min = 1, max = 200))]
    title: String,

    #[garde(skip)]
    #[inertia(sensitive)]
    internal_token: Option<String>,
}
```

The `InertiaForm` derive should generate:

* Default error-bag metadata.
* Old-input serialization.
* Sensitive-field removal.
* Field-name mapping.
* Integration with `Validated<T>`.

## Core traits

```rust
pub trait InertiaForm:
    DeserializeOwned + Send + 'static
{
    const ERROR_BAG: Option<&'static str>;

    fn old_input(
        &self,
    ) -> InertiaResult<Value>;
}

pub trait ValidateInertia {
    fn validate_inertia(
        &self,
    ) -> Result<(), ValidationErrors>;
}
```

Feature integrations:

```rust
impl<T> ValidateInertia for T
where
    T: garde::Validate
{
    // Convert garde errors.
}
```

The same model can support `validator`.

## Handler

```rust
async fn store(
    State(app): State<AppState>,
    Validated(input): Validated<CreateTodo>,
) -> Result<Redirect, AppError> {
    let todo = app.todos.create(input).await?;

    Ok(
        Redirect::to(format!("/todos/{}", todo.id))
            .success("Todo created"),
    )
}
```

`Validated<T>` should:

1. Decode JSON, URL-encoded form data, or multipart data through the configured body decoder.
2. Validate the input.
3. Use `X-Inertia-Error-Bag` when present.
4. Otherwise use `T::ERROR_BAG`.
5. Flash validation errors.
6. Flash safe old input.
7. Return a `303` redirect back.
8. Never enter the handler when validation fails.

Error-bag precedence:

```text
X-Inertia-Error-Bag request header
    ↓
#[inertia(error_bag = "...")]
    ↓
default error bag
```

## Manual validation escape hatch

Applications with domain-level validation should be able to bypass framework integrations:

```rust
async fn store(
    form: InertiaFormRequest<CreateTodo>,
) -> Result<Redirect, FormError> {
    let input = form.into_inner();

    let errors = validate_todo(&input);

    if !errors.is_empty() {
        return Err(
            form.invalid(errors)
                .with_old_input(&input),
        );
    }

    Ok(Redirect::to("/todos"))
}
```

---

# 16. Error handling

## Error type

```rust
#[derive(Debug, thiserror::Error)]
pub enum InertiaError {
    #[error("failed to serialize prop `{prop}`")]
    PropSerialization {
        prop: String,
        source: serde_json::Error,
    },

    #[error("prop resolver `{prop}` failed")]
    PropResolver {
        prop: String,
        source: BoxError,
    },

    #[error("shared props failed")]
    SharedProps {
        source: BoxError,
    },

    #[error("root template failed")]
    RootTemplate {
        source: BoxError,
    },

    #[error("invalid redirect target")]
    InvalidRedirect {
        source: BoxError,
    },

    #[error("flash store failed")]
    Flash {
        source: BoxError,
    },
}
```

## Application error handler

```rust
pub trait InertiaErrorHandler:
    Clone + Send + Sync + 'static
{
    async fn handle(
        &self,
        error: InertiaError,
        context: ErrorContext,
    ) -> Response;
}
```

Configuration:

```rust
InertiaApp::vite()
    .on_error(AppInertiaErrorHandler)
```

Default behavior:

* Log with `tracing`.
* Return a generic `500`.
* Never expose sensitive resolver details in production.
* Never panic.

A rescued deferred prop should report its error without invoking the normal fatal response handler.

---

# 17. Testing API

Official Inertia testing support centers on component assertions, nested prop checks, partial reloads, deferred groups, flash values, and redirects. ([Inertia.js][10]) ([Inertia.js][10])

The Rust adapter should provide the same capabilities through an in-process Axum client.

## Test client

```rust
pub struct InertiaTest {
    app: Router,
    cookies: TestCookieJar,
}

impl InertiaTest {
    pub fn new(app: Router) -> Self;

    pub fn get(
        &self,
        uri: impl Into<Uri>,
    ) -> TestRequest;

    pub fn post(
        &self,
        uri: impl Into<Uri>,
    ) -> TestRequest;

    pub fn patch(
        &self,
        uri: impl Into<Uri>,
    ) -> TestRequest;

    pub fn delete(
        &self,
        uri: impl Into<Uri>,
    ) -> TestRequest;
}
```

Request API:

```rust
client
    .get("/todos")
    .inertia()
    .version("test")
    .send()
    .await;
```

Browser response:

```rust
client
    .get("/todos")
    .browser()
    .send()
    .await
    .assert_html_page::<TodosIndexPage>();
```

JSON body:

```rust
client
    .post("/todos")
    .json(&serde_json::json!({
        "title": "Ship the adapter",
    }))
    .send()
    .await;
```

## Page assertions

```rust
let page = client
    .get("/todos")
    .inertia()
    .send()
    .await
    .assert_page::<TodosIndexPage>();

page
    .has_len(TodosIndexPage::TODOS, 2)
    .where_eq("todos.0.title", "Ship it")
    .has(TodosIndexPage::FILTERS)
    .missing(TodosIndexPage::ARCHIVED)
    .is_deferred(
        "summary",
        TodosIndexPage::STATS,
    );
```

Nested scope:

```rust
page.scope("todos.0", |todo| {
    todo
        .where_eq("title", "Ship it")
        .where_eq("completed", false)
        .missing("privateNotes");
});
```

## Partial reloads

```rust
let archived = page
    .reload_only([
        TodosIndexPage::ARCHIVED,
    ])
    .await;

archived
    .has(TodosIndexPage::ARCHIVED)
    .missing(TodosIndexPage::TODOS)
    .missing(TodosIndexPage::STATS);
```

Except:

```rust
let reload = page
    .reload_except([
        TodosIndexPage::TODOS,
    ])
    .await;
```

## Deferred groups

```rust
let summary = page
    .load_deferred("summary")
    .await;

summary
    .has(TodosIndexPage::STATS)
    .missing(TodosIndexPage::ARCHIVED);
```

Multiple groups:

```rust
page
    .load_deferred([
        "summary",
        "analytics",
    ])
    .await;
```

## Redirect and flash assertions

```rust
let response = client
    .post("/todos")
    .json(&serde_json::json!({
        "title": "Write tests",
    }))
    .send()
    .await;

response
    .assert_see_other("/todos")
    .assert_flash(
        "toast.message",
        "Todo created",
    );
```

Follow redirect while preserving cookies:

```rust
let page = response
    .follow()
    .await
    .assert_page::<TodosIndexPage>();

page.assert_flash(
    "toast.message",
    "Todo created",
);
```

## Validation assertions

```rust
client
    .post("/todos")
    .error_bag("createTodo")
    .json(&serde_json::json!({
        "title": "",
    }))
    .send()
    .await
    .assert_see_other("/todos")
    .assert_error(
        "createTodo",
        "title",
    )
    .assert_old_input_missing(
        "internalToken",
    );
```

## Resolver-level unit testing

Endpoint tests are not enough for complex prop policies. The testing crate should also expose a page harness:

```rust
let page = TodosIndexPage {
    todos,
    filters,
    stats,
    archived,
};

let resolved = PageHarness::new(page)
    .initial()
    .resolve()
    .await?;

resolved
    .has(TodosIndexPage::TODOS)
    .missing(TodosIndexPage::STATS)
    .missing(TodosIndexPage::ARCHIVED);
```

Partial request:

```rust
let resolved = PageHarness::new(page)
    .only([TodosIndexPage::STATS])
    .resolve()
    .await?;
```

## Resolver non-execution test

```rust
#[tokio::test]
async fn omitted_optional_prop_is_not_called() {
    let calls = Arc::new(AtomicUsize::new(0));

    let page = ExamplePage {
        optional: Optional::new({
            let calls = calls.clone();

            move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, AppError>("loaded")
            }
        }),
    };

    PageHarness::new(page)
        .initial()
        .resolve()
        .await
        .unwrap();

    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
    );
}
```

## Macro tests

The workspace should use compile-fail tests for:

* Invalid page attributes.
* Duplicate prop names.
* Unsupported prop combinations.
* Invalid form attributes.
* Renamed crate imports.
* Private and generic page types.
* Lifetimes and borrowed DTOs where supported.

---

# 18. Recommended Cargo workspace

Because this repository is Axum-specific, the protocol implementation should remain inside the main crate. Creating a separately published `core` crate now would add release and dependency surface without another real consumer.

The workspace should contain three published crates:

```text
inertia-axum/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── CHANGELOG.md
├── LICENSE-APACHE
├── LICENSE-MIT
│
├── crates/
│   ├── inertia-axum/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── prelude.rs
│   │   │   ├── app.rs
│   │   │   ├── config.rs
│   │   │   ├── error.rs
│   │   │   ├── layer.rs
│   │   │   ├── root.rs
│   │   │   ├── shared.rs
│   │   │   ├── flash.rs
│   │   │   ├── redirect.rs
│   │   │   ├── form.rs
│   │   │   │
│   │   │   ├── assets/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── vite.rs
│   │   │   │   ├── manifest.rs
│   │   │   │   └── testing.rs
│   │   │   │
│   │   │   ├── page/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── pending.rs
│   │   │   │   ├── props.rs
│   │   │   │   ├── resolver.rs
│   │   │   │   ├── metadata.rs
│   │   │   │   ├── lazy.rs
│   │   │   │   ├── optional.rs
│   │   │   │   ├── deferred.rs
│   │   │   │   ├── once.rs
│   │   │   │   ├── merge.rs
│   │   │   │   └── scroll.rs
│   │   │   │
│   │   │   └── protocol/
│   │   │       ├── mod.rs
│   │   │       ├── headers.rs
│   │   │       ├── request.rs
│   │   │       ├── response.rs
│   │   │       ├── page.rs
│   │   │       ├── filtering.rs
│   │   │       └── version.rs
│   │   │
│   │   └── tests/
│   │       ├── protocol.rs
│   │       ├── layer.rs
│   │       ├── pages.rs
│   │       ├── flash.rs
│   │       ├── validation.rs
│   │       ├── vite.rs
│   │       └── ui/
│   │           ├── pass/
│   │           └── fail/
│   │
│   ├── inertia-axum-macros/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── page.rs
│   │       ├── props.rs
│   │       ├── form.rs
│   │       ├── attributes.rs
│   │       └── diagnostics.rs
│   │
│   └── inertia-axum-testing/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── client.rs
│           ├── request.rs
│           ├── response.rs
│           ├── page.rs
│           ├── assertions.rs
│           └── cookies.rs
│
├── examples/
│   ├── minimal/
│   │   ├── Cargo.toml
│   │   ├── src/main.rs
│   │   └── frontend/
│   │
│   ├── todo/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── app.rs
│   │   │   ├── state.rs
│   │   │   ├── error.rs
│   │   │   ├── shared.rs
│   │   │   └── todos/
│   │   │       ├── mod.rs
│   │   │       ├── model.rs
│   │   │       ├── repository.rs
│   │   │       ├── pages.rs
│   │   │       ├── forms.rs
│   │   │       └── handlers.rs
│   │   └── frontend/
│   │       └── src/
│   │           ├── main.ts
│   │           └── Pages/
│   │               └── Todos/
│   │                   ├── Index.tsx
│   │                   └── Show.tsx
│   │
│   └── observatory/
│       ├── Cargo.toml
│       ├── src/
│       └── frontend/
│
├── fixtures/
│   ├── vite/
│   │   ├── valid-manifest.json
│   │   ├── missing-entry.json
│   │   └── nested-imports.json
│   └── protocol/
│       ├── initial-page.json
│       ├── deferred-page.json
│       ├── once-page.json
│       └── scroll-page.json
│
├── docs/
│   ├── design.md
│   ├── protocol-v3.md
│   ├── macros.md
│   ├── testing.md
│   ├── vite.md
│   └── migration-from-0.9.md
│
├── xtask/
│   ├── Cargo.toml
│   └── src/main.rs
│
└── .github/
    └── workflows/
        ├── ci.yml
        ├── examples.yml
        └── release.yml
```

## Root `Cargo.toml`

```toml
[workspace]
resolver = "2"

members = [
    "crates/inertia-axum",
    "crates/inertia-axum-macros",
    "crates/inertia-axum-testing",

    "examples/minimal",
    "examples/todo",
    "examples/observatory",

    "xtask",
]

default-members = [
    "crates/inertia-axum",
    "crates/inertia-axum-macros",
    "crates/inertia-axum-testing",
]

[workspace.package]
version = "1.0.0-alpha.1"
edition = "2024"
rust-version = "1.85"
license = "MIT OR Apache-2.0"
repository = "https://github.com/mjhoy/inertia-axum"

[workspace.dependencies]
axum = "0.8"
bytes = "1"
cookie = "0.18"
futures-util = "0.3"
http = "1"
http-body-util = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
tower = { version = "0.5", features = ["util"] }
tower-http = { version = "0.6", features = ["fs"] }
tracing = "0.1"

proc-macro2 = "1"
quote = "1"
syn = { version = "2", features = ["full"] }
proc-macro-crate = "3"

trybuild = "1"
```

## Package responsibilities

`inertia-axum` owns:

* Protocol implementation.
* Tower layer.
* Page resolution.
* Vite.
* Shared props.
* Flash.
* Redirects.
* Forms.
* Public facade.

`inertia-axum-macros` owns only token parsing and code generation.

`inertia-axum-testing` owns:

* In-process client.
* Request builders.
* Cookie persistence.
* Page assertions.
* Partial and deferred follow-up requests.

Macro output should reference:

```rust
::axum_inertia::__private
```

and use `proc_macro_crate` so renamed dependencies continue to work:

```toml
inertia = { package = "inertia-axum", version = "1" }
```

---

# 19. Complete Todo application DX

## Page types

```rust
use axum_inertia::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoFilters {
    pub search: Option<String>,
    pub completed: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoDto {
    pub id: uuid::Uuid,
    pub title: String,
    pub completed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoStats {
    pub total: u64,
    pub completed: u64,
    pub remaining: u64,
}

#[derive(InertiaPage)]
#[inertia(
    component = "Todos/Index",
    rename_all = "camelCase",
)]
pub struct TodosIndexPage {
    pub todos: Vec<TodoDto>,
    pub filters: TodoFilters,
    pub stats: Deferred<TodoStats>,
    pub archived: Optional<Vec<TodoDto>>,
}

#[derive(InertiaPage)]
#[inertia(
    component = "Todos/Show",
    rename_all = "camelCase",
)]
pub struct TodoShowPage {
    pub todo: TodoDto,
}
```

## Forms

```rust
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Deserialize,
    Serialize,
    garde::Validate,
    InertiaForm,
)]
#[inertia(error_bag = "createTodo")]
pub struct CreateTodo {
    #[garde(length(min = 1, max = 200))]
    pub title: String,
}

#[derive(
    Debug,
    Deserialize,
    Serialize,
    garde::Validate,
    InertiaForm,
)]
#[inertia(error_bag = "updateTodo")]
pub struct UpdateTodo {
    #[garde(length(min = 1, max = 200))]
    pub title: String,

    pub completed: bool,
}
```

## Index handler

```rust
use axum::{
    extract::{Query, State},
};

pub async fn index(
    State(app): State<AppState>,
    Query(filters): Query<TodoFilters>,
) -> Result<TodosIndexPage, AppError> {
    let todos = app.todos.search(&filters).await?;

    let stats = Deferred::new({
        let repository = app.todos.clone();

        move || async move {
            repository.stats().await
        }
    })
    .group("summary");

    let archived = Optional::new({
        let repository = app.todos.clone();

        move || async move {
            repository.archived().await
        }
    });

    Ok(TodosIndexPage {
        todos,
        filters,
        stats,
        archived,
    })
}
```

## Show handler

```rust
pub async fn show(
    State(app): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<TodoShowPage, AppError> {
    Ok(TodoShowPage {
        todo: app.todos.find(id).await?,
    })
}
```

## Store handler

```rust
pub async fn store(
    State(app): State<AppState>,
    Validated(input): Validated<CreateTodo>,
) -> Result<Redirect, AppError> {
    let todo = app.todos.create(input).await?;

    Ok(
        Redirect::to(format!("/todos/{}", todo.id))
            .success("Todo created"),
    )
}
```

## Update handler

```rust
pub async fn update(
    State(app): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Validated(input): Validated<UpdateTodo>,
) -> Result<Redirect, AppError> {
    app.todos.update(id, input).await?;

    Ok(
        Redirect::back()
            .or(format!("/todos/{id}"))
            .success("Todo updated"),
    )
}
```

## Delete handler

```rust
pub async fn destroy(
    State(app): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Redirect, AppError> {
    app.todos.delete(id).await?;

    Ok(
        Redirect::to("/todos")
            .success("Todo deleted"),
    )
}
```

## Router

```rust
use axum::{
    routing::{get, patch},
    Router,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/todos",
            get(index).post(store),
        )
        .route(
            "/todos/{id}",
            get(show)
                .patch(update)
                .delete(destroy),
        )
}
```

## Shared props

```rust
#[derive(InertiaProps)]
#[inertia(rename_all = "camelCase")]
struct TodoSharedProps {
    app_name: &'static str,
    auth: Option<AuthShared>,
}

#[derive(Serialize)]
struct AuthShared {
    user: UserSummary,
}

#[derive(Clone)]
struct TodoShared;

impl SharedProps for TodoShared {
    type Props = TodoSharedProps;

    async fn share(
        &self,
        context: SharedContext,
    ) -> InertiaResult<Self::Props> {
        let auth = context
            .extension::<CurrentUser>()
            .map(|current| AuthShared {
                user: current.summary(),
            });

        Ok(TodoSharedProps {
            app_name: "Tasks",
            auth,
        })
    }
}
```

## Application factory

Using an application factory makes production and test setup use exactly the same routes:

```rust
pub fn app<A>(
    state: AppState,
    inertia: A,
) -> Result<Router, InitError>
where
    A: IntoInertiaApp,
{
    Router::new()
        .merge(todos::router())
        .with_state(state)
        .inertia(inertia)
}
```

Production:

```rust
let app = app(
    state,
    InertiaApp::vite()
        .title("Tasks")
        .share(TodoShared)
        .flash(Flash::signed_cookie(app_key)),
)?;
```

Testing:

```rust
let app = app(
    state,
    InertiaApp::testing()
        .share(TodoShared)
        .flash(Flash::memory()),
)?;
```

## Todo endpoint tests

```rust
#[tokio::test]
async fn index_returns_todos_and_defers_stats() {
    let state = AppState::test()
        .with_todos([
            todo("Write documentation"),
            todo("Ship release"),
        ]);

    let mut client = InertiaTest::new(
        test_app(state).unwrap(),
    );

    let page = client
        .get("/todos")
        .inertia()
        .send()
        .await
        .assert_page::<TodosIndexPage>();

    page
        .has_len(TodosIndexPage::TODOS, 2)
        .where_eq(
            "todos.0.title",
            "Write documentation",
        )
        .has(TodosIndexPage::FILTERS)
        .missing(TodosIndexPage::STATS)
        .missing(TodosIndexPage::ARCHIVED)
        .is_deferred(
            "summary",
            TodosIndexPage::STATS,
        );
}
```

Deferred request:

```rust
#[tokio::test]
async fn stats_can_be_loaded_separately() {
    let mut client = todo_client().await;

    let page = client
        .get("/todos")
        .inertia()
        .send()
        .await
        .assert_page::<TodosIndexPage>();

    let summary = page
        .load_deferred("summary")
        .await;

    summary
        .has(TodosIndexPage::STATS)
        .where_eq("stats.total", 3)
        .missing(TodosIndexPage::TODOS)
        .missing(TodosIndexPage::ARCHIVED);
}
```

Optional request:

```rust
#[tokio::test]
async fn archived_is_only_loaded_explicitly() {
    let mut client = todo_client().await;

    let page = client
        .get("/todos")
        .inertia()
        .send()
        .await
        .assert_page::<TodosIndexPage>();

    let archived = page
        .reload_only([
            TodosIndexPage::ARCHIVED,
        ])
        .await;

    archived
        .has(TodosIndexPage::ARCHIVED)
        .missing(TodosIndexPage::TODOS)
        .missing(TodosIndexPage::STATS);
}
```

Create and flash:

```rust
#[tokio::test]
async fn a_todo_can_be_created() {
    let mut client = todo_client().await;

    let response = client
        .post("/todos")
        .json(&serde_json::json!({
            "title": "Test the adapter",
        }))
        .send()
        .await;

    response
        .assert_see_other_matching(
            |location| {
                location.starts_with("/todos/")
            },
        )
        .assert_flash(
            "success",
            "Todo created",
        );
}
```

Validation:

```rust
#[tokio::test]
async fn title_is_required() {
    let mut client = todo_client().await;

    client
        .post("/todos")
        .error_bag("createTodo")
        .json(&serde_json::json!({
            "title": "",
        }))
        .send()
        .await
        .assert_see_other("/todos")
        .assert_error(
            "createTodo",
            "title",
        );
}
```

---

# 20. Unique example: observatory anomaly board

This application tracks unexpected readings from remote telescopes. It demonstrates deferred scientific computations, optional raw data, infinite scroll, once props, merge metadata, flash responses, and external locations.

## Page

```rust
#[derive(InertiaPage)]
#[inertia(
    component = "Anomalies/Show",
    rename_all = "camelCase",
    encrypt_history,
)]
struct AnomalyShowPage {
    anomaly: AnomalyDto,

    timeline: Scroll<AnomalyEventDto>,

    telemetry: Deferred<TelemetrySummary>,

    affected_instruments:
        Deferred<Vec<InstrumentSummary>>,

    raw_frames:
        Optional<Vec<RawFrameDto>>,

    calibration_profiles:
        Once<Vec<CalibrationProfileDto>>,

    #[inertia(
        deep_merge,
        match_on = "id",
    )]
    collaborators: Vec<CollaboratorDto>,
}
```

## Handler

```rust
async fn show_anomaly(
    State(app): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Query(query): Query<TimelineQuery>,
) -> Result<AnomalyShowPage, AppError> {
    let anomaly =
        app.anomalies.find(id).await?;

    let timeline_page = app
        .anomalies
        .timeline(id, query.page)
        .await?;

    let timeline = Scroll::new(
        timeline_page.items,
    )
    .page_name("timeline_page")
    .current(timeline_page.current)
    .previous(timeline_page.previous)
    .next(timeline_page.next);

    let telemetry = Deferred::new({
        let telemetry = app.telemetry.clone();

        move || async move {
            telemetry.summary_for(id).await
        }
    })
    .group("science")
    .rescue();

    let affected_instruments = Deferred::new({
        let instruments = app.instruments.clone();

        move || async move {
            instruments.affected_by(id).await
        }
    })
    .group("science");

    let raw_frames = Optional::new({
        let telemetry = app.telemetry.clone();

        move || async move {
            telemetry.raw_frames(id).await
        }
    });

    let calibration_profiles = Once::new({
        let calibration = app.calibration.clone();

        move || async move {
            calibration.profiles().await
        }
    })
    .key("calibration-profiles:v3")
    .ttl(Duration::from_secs(12 * 60 * 60));

    let collaborators = app
        .anomalies
        .collaborators(id)
        .await?;

    Ok(AnomalyShowPage {
        anomaly,
        timeline,
        telemetry,
        affected_instruments,
        raw_frames,
        calibration_profiles,
        collaborators,
    })
}
```

## Create form

```rust
#[derive(
    Debug,
    Deserialize,
    Serialize,
    garde::Validate,
    InertiaForm,
)]
#[inertia(error_bag = "createAnomaly")]
struct CreateAnomaly {
    #[garde(length(min = 3, max = 120))]
    title: String,

    instrument_id: uuid::Uuid,

    severity: Severity,

    #[garde(length(max = 4_000))]
    summary: String,

    #[inertia(sensitive)]
    internal_operator_token: Option<String>,
}
```

## Create handler

```rust
async fn create_anomaly(
    State(app): State<AppState>,
    Extension(operator): Extension<CurrentOperator>,
    Validated(input): Validated<CreateAnomaly>,
) -> Result<Redirect, AppError> {
    let anomaly = app
        .anomalies
        .create(operator.id, input)
        .await?;

    Ok(
        Redirect::to(format!(
            "/anomalies/{}",
            anomaly.id,
        ))
        .flash(
            "toast",
            Toast::success(
                "Anomaly command room created",
            ),
        ),
    )
}
```

## Acknowledge handler

```rust
async fn acknowledge_anomaly(
    State(app): State<AppState>,
    Extension(operator): Extension<CurrentOperator>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Redirect, AppError> {
    app.anomalies
        .acknowledge(id, operator.id)
        .await?;

    Ok(
        Redirect::back()
            .or(format!("/anomalies/{id}"))
            .success("Anomaly acknowledged"),
    )
}
```

## External telescope console

```rust
async fn telescope_console(
    State(app): State<AppState>,
    Path(instrument_id): Path<uuid::Uuid>,
) -> Result<Location, AppError> {
    let console_url = app
        .instruments
        .console_url(instrument_id)
        .await?;

    Ok(Location::external(console_url))
}
```

## Router

```rust
fn anomaly_router() -> Router<AppState> {
    Router::new()
        .route(
            "/anomalies",
            post(create_anomaly),
        )
        .route(
            "/anomalies/{id}",
            get(show_anomaly),
        )
        .route(
            "/anomalies/{id}/acknowledge",
            post(acknowledge_anomaly),
        )
        .route(
            "/instruments/{instrument_id}/console",
            get(telescope_console),
        )
}
```

## Initial-page test

```rust
#[tokio::test]
async fn initial_anomaly_page_is_fast() {
    let mut client = observatory_client().await;

    let page = client
        .get("/anomalies/anomaly-1")
        .inertia()
        .send()
        .await
        .assert_page::<AnomalyShowPage>();

    page
        .has(AnomalyShowPage::ANOMALY)
        .has(AnomalyShowPage::TIMELINE)
        .has(AnomalyShowPage::CALIBRATION_PROFILES)
        .has(AnomalyShowPage::COLLABORATORS)
        .missing(AnomalyShowPage::TELEMETRY)
        .missing(
            AnomalyShowPage::AFFECTED_INSTRUMENTS,
        )
        .missing(AnomalyShowPage::RAW_FRAMES)
        .is_deferred(
            "science",
            AnomalyShowPage::TELEMETRY,
        )
        .is_deferred(
            "science",
            AnomalyShowPage::AFFECTED_INSTRUMENTS,
        );
}
```

## Deferred group test

```rust
#[tokio::test]
async fn science_data_loads_in_one_group() {
    let mut client = observatory_client().await;

    let page = client
        .get("/anomalies/anomaly-1")
        .inertia()
        .send()
        .await
        .assert_page::<AnomalyShowPage>();

    let science = page
        .load_deferred("science")
        .await;

    science
        .has(AnomalyShowPage::TELEMETRY)
        .has(
            AnomalyShowPage::AFFECTED_INSTRUMENTS,
        )
        .missing(AnomalyShowPage::RAW_FRAMES)
        .missing(AnomalyShowPage::TIMELINE);
}
```

## Optional raw-data test

```rust
#[tokio::test]
async fn raw_frames_require_explicit_request() {
    let mut client = observatory_client().await;

    let page = client
        .get("/anomalies/anomaly-1")
        .inertia()
        .send()
        .await
        .assert_page::<AnomalyShowPage>();

    page
        .reload_only([
            AnomalyShowPage::RAW_FRAMES,
        ])
        .await
        .has(AnomalyShowPage::RAW_FRAMES)
        .missing(AnomalyShowPage::TELEMETRY)
        .missing(AnomalyShowPage::TIMELINE);
}
```

## Once-prop test

```rust
#[tokio::test]
async fn calibration_profiles_are_not_reloaded() {
    let calls = Arc::new(AtomicUsize::new(0));

    let app = observatory_test_app_with_counter(
        calls.clone(),
    );

    let mut client = InertiaTest::new(app);

    client
        .get("/anomalies/anomaly-1")
        .inertia()
        .send()
        .await
        .assert_page::<AnomalyShowPage>()
        .has(
            AnomalyShowPage::CALIBRATION_PROFILES,
        );

    client
        .get("/anomalies/anomaly-2")
        .inertia()
        .remember_once([
            "calibration-profiles:v3",
        ])
        .send()
        .await
        .assert_page::<AnomalyShowPage>()
        .missing(
            AnomalyShowPage::CALIBRATION_PROFILES,
        );

    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
    );
}
```

---

# 21. Deliberate omissions

The first implementation should not include:

* A custom routing DSL.
* `#[get]`, `#[post]`, or controller proc macros.
* A required CLI.
* Frontend directory scanning during every macro expansion.
* A separately published protocol-core crate.
* ORM-specific integrations.
* A validation DSL invented by this crate.
* Required TypeScript generation.
* Framework-specific concepts unrelated to Inertia.

The page component string appearing once in the page declaration is an acceptable trade-off. Introducing a component scanner and generated Rust modules to remove that final string would add significantly more build complexity than DX value.

TypeScript generation can be added later as an independent optional package without changing the page or prop model.

---

# 22. Definition of done

The redesign is complete only when these examples are possible and these rules hold:

1. A standard application installs Inertia with one `.inertia(...)` call.
2. `InertiaApp` is never required inside `AppState`.
3. No page handler extracts an Inertia request object.
4. A derived page struct can be returned directly from an Axum handler.
5. The component name appears only in the page declaration.
6. Partial selection happens before resolver invocation.
7. Omitted async resolvers are never called or polled.
8. Selected async resolvers execute concurrently.
9. Shared props are typed and request-aware.
10. Flash values are available through `page.flash`.
11. Validation failures redirect back and populate `page.props.errors`.
12. Version mismatches short-circuit before handler execution.
13. Write redirects use `303 See Other`.
14. Initial page JSON is safe for script embedding.
15. No serialization or header error can panic.
16. Every public macro has pass and compile-fail tests.
17. Endpoint tests use in-process Axum services without sockets.
18. Tests can perform partial, deferred, once, flash, redirect, and validation assertions.
19. The Todo and observatory examples are workspace members and run in CI.
20. Protocol behavior is covered by fixtures and tests representing the complete Inertia 3 page object and request headers.

[1]: https://docs.rs/inertia-axum/latest/axum_inertia/ "https://docs.rs/inertia-axum/latest/axum_inertia/"
[2]: https://docs.rs/inertia-axum/latest/src/axum_inertia/lib.rs.html "https://docs.rs/inertia-axum/latest/src/axum_inertia/lib.rs.html"
[3]: https://github.com/mjhoy/inertia-axum/blob/main/src/request.rs "https://github.com/mjhoy/inertia-axum/blob/main/src/request.rs"
[4]: https://inertiajs.com/the-protocol "https://inertiajs.com/the-protocol"
[5]: https://docs.rs/inertia-axum/latest/axum_inertia/vite/index.html "https://docs.rs/inertia-axum/latest/axum_inertia/vite/index.html"
[6]: https://inertiajs.com/docs/v3/data-props/once-props "https://inertiajs.com/docs/v3/data-props/once-props"
[7]: https://inertiajs.com/shared-data "https://inertiajs.com/shared-data"
[8]: https://inertiajs.com/docs/v3/data-props/flash-data "https://inertiajs.com/docs/v3/data-props/flash-data"
[9]: https://inertiajs.com/validation "https://inertiajs.com/validation"
[10]: https://inertiajs.com/testing "https://inertiajs.com/testing"
