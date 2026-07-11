# Migrating from 0.5 to 1.0.0-alpha.1

The 1.0 alpha replaces per-handler rendering orchestration with direct page
responses. Protocol behavior remains compatible, while typed pages, async prop
policies, Vite setup, validation, and flash move into one application layer.

## Application setup

Before, applications assembled a root callback, manifest, static service, and
version middleware separately. Replace that setup with one conventional Vite
builder:

```rust,ignore
// Before: application-specific root/manifest/static/version setup.
let app = Router::new()
    .route("/todos", get(index))
    .layer(version_layer)
    .nest_service("/build", assets)
    .layer(Extension(shared));

// After:
let app = Router::new()
    .route("/todos", get(index))
    .inertia(InertiaApp::vite("frontend").build()?);
```

The defaults are `frontend/src/main.ts`, `frontend/dist/.vite/manifest.json`,
and `/build`. Override them on the builder only when the frontend differs.

## Page handlers

Remove the Inertia extractor and manual response construction from ordinary
handlers:

```rust,ignore
// Before:
async fn index(request: InertiaRequest) -> Result<Response, InertiaError> {
    request.render(Inertia::response("Todos/Index", props), shell)
}

// After, dynamic:
async fn index() -> DynamicPage {
    page!("Todos/Index", { todos, filters })
}
```

For typed pages, derive `InertiaPage`; fields produce typed `PropKey<T>`
constants and `Prop<T>` fields retain their loading policies:

```rust,ignore
#[derive(InertiaPage)]
#[inertia(component = "Todos/Index", rename_all = "camelCase")]
struct TodosIndexPage {
    todos: Vec<TodoDto>,
    stats: Prop<TodoStats>,
}

async fn index() -> TodosIndexPage {
    TodosIndexPage {
        todos: load_todos().await,
        stats: defer(load_stats).group("summary"),
    }
}
```

`Inertia<T>`, `InertiaPageBuilder`, `InertiaProps`, `InertiaRequest`,
`VersionLayer`, and compatibility shared providers remain available during the
alpha. New common-path code should use direct responses and `InertiaApp`.

## Shared data

Replace multiple string-keyed registrations with one typed `Share` provider.
Route props still win over route-local shared values, which win over global
shared values. Authentication middleware must wrap the Inertia layer so its
request extension is visible to the provider.

## Validation and flash

Replace manual `422` JSON handling with an `InertiaForm` and `Validated<T>`.
Validation failures redirect back with `303 See Other`; their errors appear on
the next page. Configure transient storage once:

```rust,ignore
let inertia = InertiaApp::vite("frontend")
    .transient(CookieTransient::encrypted(app_key))
    .build()?;

#[derive(Deserialize, InertiaForm)]
#[inertia(validate_with = "validate_todo", error_bag = "createTodo")]
struct CreateTodo { title: String }

async fn store(Validated(input): Validated<CreateTodo>) -> Redirect {
    save(input).await;
    Redirect::to("/todos").flash("toast", "Todo created")
}
```

Old input remains opt-in. If enabled, list secrets with `redact = "token"` or
mark fields `#[inertia(sensitive)]`.

## Tests

Use `inertia-axum-test` and generated keys for initial HTML, JSON visits,
partials, redirects, validation, flash, merge metadata, rescue, and versions:

```rust,ignore
let page = TestApp::new(router)
    .inertia_get("/todos")
    .only(TodosIndexPage::STATS)
    .send().await
    .assert_page::<TodosIndexPage>();

let stats: TodoStats = page.prop(TodosIndexPage::STATS);
```
