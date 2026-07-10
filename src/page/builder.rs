use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

use crate::page::{OnceProp, Page, PageDraft, PageMetadata, ScrollProps};
use crate::props::IntoPageProps;
use crate::redirect::{Location, Redirect};
use crate::request::RequestContext;

/// Builder for the advanced `Inertia::page(...).props(...)` API.
pub struct InertiaPageBuilder {
    component: String,
    url: Option<String>,
    metadata: PageMetadata,
    local_shared: Vec<(String, Value)>,
}

impl InertiaPageBuilder {
    /// Marks the page's history state for encryption.
    pub fn encrypt_history(mut self) -> Self {
        self.metadata = self.metadata.encrypt_history();
        self
    }

    /// Marks the response as clearing encrypted history state.
    pub fn clear_history(mut self) -> Self {
        self.metadata = self.metadata.clear_history();
        self
    }

    /// Preserves the original URL fragment across a redirect.
    pub fn preserve_fragment(mut self) -> Self {
        self.metadata = self.metadata.preserve_fragment();
        self
    }

    /// Marks a prop key to always be included during partial reloads.
    pub fn always<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.always(prop);
        self
    }

    /// Marks a prop key for append-style merging.
    pub fn merge<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.merge(prop);
        self
    }

    /// Marks a prop key for prepend-style merging.
    pub fn prepend<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.prepend(prop);
        self
    }

    /// Marks a prop key for deep merging.
    pub fn deep_merge<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.deep_merge(prop);
        self
    }

    /// Adds a matching key used by merge metadata.
    pub fn match_on<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.match_on(prop);
        self
    }

    /// Adds infinite-scroll metadata for a prop.
    pub fn scroll<P: Into<String>>(mut self, prop: P, scroll: ScrollProps) -> Self {
        self.metadata = self.metadata.scroll(prop, scroll);
        self
    }

    /// Marks a prop as deferred in the default group.
    ///
    /// This declares page-object metadata and omits the prop value until it is
    /// explicitly requested by a partial reload. It does not install a lazy or
    /// async resolver.
    pub fn defer<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.defer(prop);
        self
    }

    /// Marks a prop as deferred in `group`.
    ///
    /// This declares page-object metadata and omits the prop value until it is
    /// explicitly requested by a partial reload. It does not install a lazy or
    /// async resolver.
    pub fn defer_group<G: Into<String>, P: Into<String>>(mut self, group: G, prop: P) -> Self {
        self.metadata = self.metadata.defer_group(group, prop);
        self
    }

    /// Marks a deferred prop as rescued.
    ///
    /// This only serializes the `rescuedProps` metadata. It does not catch
    /// errors while resolving prop values.
    pub fn rescue<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.rescue(prop);
        self
    }

    /// Marks a top-level prop as shared.
    ///
    /// This only serializes the `sharedProps` metadata. It does not register or
    /// merge global shared application state.
    pub fn share<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.share(prop);
        self
    }

    /// Marks a prop as a once prop using the prop name as the once key.
    pub fn once<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.once(prop);
        self
    }

    /// Marks a prop as a once prop with an explicit once key.
    pub fn once_with_key<K: Into<String>>(mut self, key: K, once: OnceProp) -> Self {
        self.metadata = self.metadata.once_with_key(key, once);
        self
    }

    /// Sets the page props and returns an [`Inertia`] response.
    pub fn props<T>(self, props: T) -> Inertia<T> {
        Inertia {
            component: self.component,
            props,
            url: self.url,
            metadata: self.metadata,
            local_shared: self.local_shared,
        }
    }

    /// Adds a pre-serialized route-local shared value.
    pub fn shared_value<K>(mut self, key: K, value: Value) -> Self
    where
        K: Into<String>,
    {
        self.local_shared.push((key.into(), value));
        self
    }

    /// Serializes and adds a route-local shared value.
    pub fn serialize_shared<K, V>(mut self, key: K, value: V) -> Result<Self, serde_json::Error>
    where
        K: Into<String>,
        V: Serialize,
    {
        self.local_shared
            .push((key.into(), serde_json::to_value(value)?));
        Ok(self)
    }

    /// Overrides the page object's `url` field.
    pub fn with_url<U: Into<String>>(mut self, url: U) -> Self {
        self.url = Some(url.into());
        self
    }
}

impl Inertia<()> {
    /// Starts the advanced page builder API.
    pub fn page<C: Into<String>>(component: C) -> InertiaPageBuilder {
        InertiaPageBuilder {
            component: component.into(),
            url: None,
            metadata: PageMetadata::new(),
            local_shared: Vec::new(),
        }
    }

    /// Creates an external redirect response.
    ///
    /// Framework integrations should convert this into a `409 Conflict`
    /// response with the destination URL in the `X-Inertia-Location` header,
    /// or `X-Inertia-Redirect` when the destination contains a fragment.
    pub fn location<U: Into<String>>(url: U) -> Location {
        Location::new(url)
    }

    /// Creates a method-aware redirect response.
    ///
    /// Framework integrations should use `303 See Other` for write-method
    /// requests so the follow-up request is a `GET`.
    pub fn redirect<U: Into<String>>(url: U) -> Redirect {
        Redirect::new(url)
    }
}

/// A framework-neutral Inertia page response.
///
/// Framework integrations convert this value into either an HTML first-load
/// response or a JSON Inertia response, depending on the incoming request
/// headers.
pub struct Inertia<T> {
    component: String,
    props: T,
    url: Option<String>,
    metadata: PageMetadata,
    local_shared: Vec<(String, Value)>,
}

impl<T> Inertia<T> {
    /// Constructs a response for `component` with serializable `props`.
    ///
    /// Framework integrations default the page object's `url` field to the
    /// current request URI unless [`with_url`](Self::with_url) is used.
    pub fn response<C: Into<String>>(component: C, props: T) -> Self {
        Self {
            component: component.into(),
            props,
            url: None,
            metadata: PageMetadata::new(),
            local_shared: Vec::new(),
        }
    }

    /// Overrides the page object's `url` field.
    pub fn with_url<U: Into<String>>(mut self, url: U) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Marks the page's history state for encryption.
    pub fn encrypt_history(mut self) -> Self {
        self.metadata = self.metadata.encrypt_history();
        self
    }

    /// Marks the response as clearing encrypted history state.
    pub fn clear_history(mut self) -> Self {
        self.metadata = self.metadata.clear_history();
        self
    }

    /// Preserves the original URL fragment across a redirect.
    pub fn preserve_fragment(mut self) -> Self {
        self.metadata = self.metadata.preserve_fragment();
        self
    }

    /// Marks a prop key to always be included during partial reloads.
    pub fn always<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.always(prop);
        self
    }

    /// Marks a prop key for append-style merging.
    pub fn merge<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.merge(prop);
        self
    }

    /// Marks a prop key for prepend-style merging.
    pub fn prepend<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.prepend(prop);
        self
    }

    /// Marks a prop key for deep merging.
    pub fn deep_merge<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.deep_merge(prop);
        self
    }

    /// Adds a matching key used by merge metadata.
    pub fn match_on<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.match_on(prop);
        self
    }

    /// Adds infinite-scroll metadata for a prop.
    pub fn scroll<P: Into<String>>(mut self, prop: P, scroll: ScrollProps) -> Self {
        self.metadata = self.metadata.scroll(prop, scroll);
        self
    }

    /// Marks a prop as deferred in the default group.
    ///
    /// This declares page-object metadata and omits the prop value until it is
    /// explicitly requested by a partial reload. It does not install a lazy or
    /// async resolver.
    pub fn defer<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.defer(prop);
        self
    }

    /// Marks a prop as deferred in `group`.
    ///
    /// This declares page-object metadata and omits the prop value until it is
    /// explicitly requested by a partial reload. It does not install a lazy or
    /// async resolver.
    pub fn defer_group<G: Into<String>, P: Into<String>>(mut self, group: G, prop: P) -> Self {
        self.metadata = self.metadata.defer_group(group, prop);
        self
    }

    /// Marks a deferred prop as rescued.
    ///
    /// This only serializes the `rescuedProps` metadata. It does not catch
    /// errors while resolving prop values.
    pub fn rescue<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.rescue(prop);
        self
    }

    /// Marks a top-level prop as shared.
    ///
    /// This only serializes the `sharedProps` metadata. It does not register or
    /// merge global shared application state.
    pub fn share<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.share(prop);
        self
    }

    /// Marks a prop as a once prop using the prop name as the once key.
    pub fn once<P: Into<String>>(mut self, prop: P) -> Self {
        self.metadata = self.metadata.once(prop);
        self
    }

    /// Marks a prop as a once prop with an explicit once key.
    pub fn once_with_key<K: Into<String>>(mut self, key: K, once: OnceProp) -> Self {
        self.metadata = self.metadata.once_with_key(key, once);
        self
    }

    /// Returns the Inertia component name.
    pub fn component(&self) -> &str {
        &self.component
    }

    /// Returns a reference to the component props.
    pub fn props(&self) -> &T {
        &self.props
    }

    /// Returns the explicit page URL override, if one was set.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Returns the configured page metadata.
    pub fn metadata(&self) -> &PageMetadata {
        &self.metadata
    }

    /// Adds a pre-serialized route-local shared value.
    pub fn shared_value<K>(mut self, key: K, value: Value) -> Self
    where
        K: Into<String>,
    {
        self.local_shared.push((key.into(), value));
        self
    }

    /// Serializes and adds a route-local shared value.
    pub fn serialize_shared<K, V>(mut self, key: K, value: V) -> Result<Self, serde_json::Error>
    where
        K: Into<String>,
        V: Serialize,
    {
        self.local_shared
            .push((key.into(), serde_json::to_value(value)?));
        Ok(self)
    }
}

impl<T: IntoPageProps> Inertia<T> {
    pub(crate) fn into_page_draft(
        self,
        default_url: &str,
        version: Option<Arc<str>>,
        request: &RequestContext,
        partial_reload_enabled: bool,
    ) -> Result<PageDraft, serde_json::Error> {
        let component = self.component;
        let url = self.url.unwrap_or_else(|| default_url.to_owned());
        let (props, metadata, route_props) = self.props.into_page_props(
            &component,
            request,
            partial_reload_enabled,
            self.metadata,
        )?;
        let mut draft = PageDraft::new(
            Page::from_parts_arc(component, props, url, version, metadata),
            route_props,
        );
        for (key, value) in self.local_shared {
            draft.insert_shared(&key, value);
        }
        Ok(draft)
    }

    /// Builds a concrete Inertia page object.
    ///
    /// Framework integrations pass the resolved request URL, asset version,
    /// and parsed request context so props can be filtered for partial reloads,
    /// deferred props, and once props.
    pub fn into_page(
        self,
        url: impl Into<String>,
        version: Option<String>,
        request: &RequestContext,
    ) -> Result<Page<Value>, serde_json::Error> {
        let url = url.into();
        self.into_page_draft(&url, version.map(Arc::from), request, true)
            .map(PageDraft::finish)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use serde_json::json;
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::rc::Rc;

    fn request_context_from(headers: &[(&str, &str)]) -> RequestContext {
        let headers = headers.iter().copied().collect::<HashMap<_, _>>();

        RequestContext::from_header_fn(|name| headers.get(name).copied())
    }

    #[test]
    fn request_context_parses_inertia_headers() {
        let context = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_VERSION, "abc"),
            (X_INERTIA_PARTIAL_COMPONENT, "Users/Index"),
            (X_INERTIA_PARTIAL_DATA, "users, stats"),
            (X_INERTIA_RESET, "users"),
            (X_INERTIA_ERROR_BAG, "createUser"),
            (X_INERTIA_INFINITE_SCROLL_MERGE_INTENT, "append"),
            (X_INERTIA_EXCEPT_ONCE_PROPS, "plans,features"),
            (PURPOSE, "prefetch"),
            (CACHE_CONTROL, "max-age=0, no-cache"),
        ]);

        assert!(context.is_inertia());
        assert_eq!(context.version(), Some("abc"));
        assert_eq!(context.partial_component(), Some("Users/Index"));
        assert_eq!(context.partial_data(), ["users", "stats"]);
        assert_eq!(context.reset(), ["users"]);
        assert_eq!(context.error_bag(), Some("createUser"));
        assert_eq!(context.infinite_scroll_merge_intent(), Some("append"));
        assert_eq!(context.except_once_props(), ["plans", "features"]);
        assert!(context.is_prefetch());
        assert!(context.is_reload());
    }

    #[test]
    fn page_serializes_v3_metadata() {
        let page = Page::from_parts(
            "Feed/Index",
            json!({ "errors": {}, "posts": [{ "id": 1 }] }),
            "/feed",
            Some("version-1".into()),
            PageMetadata::new()
                .encrypt_history()
                .clear_history()
                .preserve_fragment()
                .merge("posts")
                .prepend("notifications")
                .deep_merge("conversations")
                .match_on("posts.id")
                .scroll("posts", ScrollProps::new("page", 1).next_page(2))
                .defer("analytics")
                .rescue("analytics")
                .share("auth")
                .once("plans"),
        );

        let value = serde_json::to_value(page).unwrap();

        assert_eq!(value["component"], "Feed/Index");
        assert_eq!(value["version"], "version-1");
        assert_eq!(value["encryptHistory"], true);
        assert_eq!(value["clearHistory"], true);
        assert_eq!(value["preserveFragment"], true);
        assert_eq!(value["mergeProps"], json!(["posts", "posts.data"]));
        assert_eq!(value["prependProps"], json!(["notifications"]));
        assert_eq!(value["deepMergeProps"], json!(["conversations"]));
        assert_eq!(value["matchPropsOn"], json!(["posts.id"]));
        assert_eq!(value["scrollProps"]["posts"]["pageName"], "page");
        assert_eq!(value["scrollProps"]["posts"]["nextPage"], 2);
        assert_eq!(value["deferredProps"], json!({ "default": ["analytics"] }));
        assert_eq!(value["rescuedProps"], json!(["analytics"]));
        assert_eq!(value["sharedProps"], json!(["auth"]));
        assert_eq!(
            value["onceProps"]["plans"],
            json!({ "prop": "plans", "expiresAt": null })
        );
    }

    #[test]
    fn route_local_shared_values_merge_before_global_values() {
        let page = Inertia::response("Dashboard", serde_json::json!({}))
            .shared_value("auth.user", serde_json::json!({"name": "Route"}))
            .into_page("/dashboard", None, &RequestContext::default())
            .unwrap()
            .with_shared_props([("auth.user", serde_json::json!({"name": "Global"}))]);

        assert_eq!(page.props()["auth"]["user"]["name"], "Route");
    }

    #[test]
    fn partial_data_filters_matching_component_props() {
        let context = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Events"),
            (X_INERTIA_PARTIAL_DATA, "events"),
        ]);
        let mut props = json!({
            "auth": { "name": "Ada" },
            "events": [1, 2],
            "categories": ["meetups"]
        });

        context.filter_props("Events", &mut props, &PageMetadata::new().always("auth"));

        assert_eq!(
            props,
            json!({
                "errors": {},
                "auth": { "name": "Ada" },
                "events": [1, 2]
            })
        );
    }

    #[test]
    fn partial_except_takes_precedence_over_partial_data() {
        let context = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Events"),
            (X_INERTIA_PARTIAL_DATA, "events"),
            (X_INERTIA_PARTIAL_EXCEPT, "categories"),
        ]);
        let mut props = json!({
            "auth": { "name": "Ada" },
            "events": [1, 2],
            "categories": ["meetups"]
        });

        context.filter_props("Events", &mut props, &PageMetadata::new());

        assert_eq!(
            props,
            json!({
                "errors": {},
                "auth": { "name": "Ada" },
                "events": [1, 2]
            })
        );
    }

    #[test]
    fn partial_except_without_partial_data_excludes_listed_props() {
        let context = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Events"),
            (X_INERTIA_PARTIAL_EXCEPT, "categories"),
        ]);
        let mut props = json!({
            "events": [1, 2],
            "categories": ["meetups"],
            "filters": { "open": true }
        });

        context.filter_props("Events", &mut props, &PageMetadata::new());

        assert_eq!(
            props,
            json!({
                "errors": {},
                "events": [1, 2],
                "filters": { "open": true }
            })
        );
    }

    #[test]
    fn partial_headers_are_ignored_for_different_components() {
        let context = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Events"),
            (X_INERTIA_PARTIAL_DATA, "events"),
        ]);
        let mut props = json!({
            "auth": { "name": "Ada" },
            "events": [1, 2]
        });

        context.filter_props("Dashboard", &mut props, &PageMetadata::new());

        assert_eq!(
            props,
            json!({
                "errors": {},
                "auth": { "name": "Ada" },
                "events": [1, 2]
            })
        );
    }

    #[test]
    fn deferred_and_once_props_are_omitted_until_explicitly_requested() {
        let context =
            request_context_from(&[(X_INERTIA, "true"), (X_INERTIA_EXCEPT_ONCE_PROPS, "plans")]);
        let mut props = json!({
            "analytics": { "views": 10 },
            "plans": ["basic"],
            "user": { "name": "Ada" }
        });
        let metadata = PageMetadata::new().defer("analytics").once("plans");

        context.filter_props("Dashboard", &mut props, &metadata);

        assert_eq!(
            props,
            json!({
                "errors": {},
                "user": { "name": "Ada" }
            })
        );

        let context = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Dashboard"),
            (X_INERTIA_PARTIAL_DATA, "analytics,plans"),
            (X_INERTIA_EXCEPT_ONCE_PROPS, "plans"),
        ]);
        let mut props = json!({
            "analytics": { "views": 10 },
            "plans": ["basic"],
            "user": { "name": "Ada" }
        });

        context.filter_props("Dashboard", &mut props, &metadata);

        assert_eq!(
            props,
            json!({
                "analytics": { "views": 10 },
                "errors": {},
                "plans": ["basic"]
            })
        );
    }

    #[test]
    fn request_reset_filters_merge_and_scroll_metadata() {
        let request = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Feed"),
            (X_INERTIA_PARTIAL_DATA, "posts"),
            (X_INERTIA_RESET, "posts"),
        ]);
        let response = Inertia::page("Feed")
            .scroll("posts", ScrollProps::new("page", 1).next_page(2))
            .match_on("posts.data.id")
            .props(json!({ "posts": { "data": [1, 2] } }))
            .into_page("/feed", Some("version-1".into()), &request)
            .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(value["props"]["posts"]["data"], json!([1, 2]));
        assert!(value.get("mergeProps").is_none());
        assert!(value.get("matchPropsOn").is_none());
        assert!(value.get("scrollProps").is_none());
    }

    #[test]
    fn reset_and_scroll_intent_are_ignored_when_partial_component_differs() {
        let request = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Other"),
            (X_INERTIA_PARTIAL_DATA, "posts"),
            (X_INERTIA_RESET, "posts"),
            (X_INERTIA_INFINITE_SCROLL_MERGE_INTENT, "prepend"),
        ]);
        let response = Inertia::page("Feed")
            .scroll("posts", ScrollProps::new("page", 1).next_page(2))
            .props(json!({ "posts": { "data": [1, 2] } }))
            .into_page("/feed", Some("version-1".into()), &request)
            .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(value["props"]["posts"]["data"], json!([1, 2]));
        assert_eq!(value["mergeProps"], json!(["posts.data"]));
        assert_eq!(value["scrollProps"]["posts"]["nextPage"], 2);
        assert!(value.get("prependProps").is_none());
    }

    #[test]
    fn infinite_scroll_merge_intent_can_prepend_scroll_props() {
        let request = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Feed"),
            (X_INERTIA_PARTIAL_DATA, "posts"),
            (X_INERTIA_INFINITE_SCROLL_MERGE_INTENT, "prepend"),
        ]);
        let response = Inertia::page("Feed")
            .scroll("posts", ScrollProps::new("page", 1).next_page(2))
            .props(json!({ "posts": { "data": [1, 2] } }))
            .into_page("/feed", Some("version-1".into()), &request)
            .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(value["prependProps"], json!(["posts.data"]));
        assert!(value.get("mergeProps").is_none());
        assert_eq!(value["scrollProps"]["posts"]["nextPage"], 2);
    }

    #[test]
    fn once_with_custom_key_omits_loaded_prop_until_requested() {
        let request = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_EXCEPT_ONCE_PROPS, "billing"),
        ]);
        let response = Inertia::response(
            "Billing",
            json!({
                "current_plan": "basic",
                "plans": ["basic", "pro"]
            }),
        )
        .once_with_key("billing", OnceProp::new("plans").expires_at(123))
        .into_page("/billing", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert!(value["props"].get("plans").is_none());
        assert_eq!(
            value["onceProps"]["billing"],
            json!({ "prop": "plans", "expiresAt": 123 })
        );
    }

    #[test]
    fn lazy_props_are_only_resolved_when_included() {
        let request = request_context_from(&[]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Dashboard",
            InertiaProps::new()
                .value("user", json!({ "name": "Ada" }))
                .lazy("stats", {
                    let calls = Rc::clone(&calls);
                    move || {
                        calls.set(calls.get() + 1);
                        json!({ "views": 10 })
                    }
                }),
        )
        .into_page("/dashboard", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 1);
        assert_eq!(value["props"]["stats"]["views"], 10);

        let request = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Dashboard"),
            (X_INERTIA_PARTIAL_DATA, "user"),
        ]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Dashboard",
            InertiaProps::new()
                .value("user", json!({ "name": "Ada" }))
                .lazy("stats", {
                    let calls = Rc::clone(&calls);
                    move || {
                        calls.set(calls.get() + 1);
                        json!({ "views": 10 })
                    }
                }),
        )
        .into_page("/dashboard", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 0);
        assert_eq!(value["props"]["user"]["name"], "Ada");
        assert!(value["props"].get("stats").is_none());
    }

    #[test]
    fn lazy_props_can_borrow_values_for_immediate_rendering() {
        let request = request_context_from(&[]);
        let name = String::from("Ada");
        let response = Inertia::response(
            "Profile",
            ScopedInertiaProps::new()
                .value("name", &name)
                .lazy("upperName", || name.to_uppercase()),
        )
        .into_page("/profile", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(value["props"]["name"], "Ada");
        assert_eq!(value["props"]["upperName"], "ADA");
    }

    #[test]
    fn optional_props_resolve_only_when_explicitly_requested() {
        let request = request_context_from(&[]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Dashboard",
            InertiaProps::new().optional("audit", {
                let calls = Rc::clone(&calls);
                move || {
                    calls.set(calls.get() + 1);
                    json!(["created"])
                }
            }),
        )
        .into_page("/dashboard", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 0);
        assert!(value["props"].get("audit").is_none());

        let request = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Dashboard"),
            (X_INERTIA_PARTIAL_DATA, "audit"),
        ]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Dashboard",
            InertiaProps::new().optional("audit", {
                let calls = Rc::clone(&calls);
                move || {
                    calls.set(calls.get() + 1);
                    json!(["created"])
                }
            }),
        )
        .into_page("/dashboard", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 1);
        assert_eq!(value["props"]["audit"], json!(["created"]));
    }

    #[test]
    fn optional_props_respect_partial_except_precedence() {
        let request = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Dashboard"),
            (X_INERTIA_PARTIAL_DATA, "audit"),
            (X_INERTIA_PARTIAL_EXCEPT, "audit"),
        ]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Dashboard",
            InertiaProps::new().optional("audit", {
                let calls = Rc::clone(&calls);
                move || {
                    calls.set(calls.get() + 1);
                    json!(["created"])
                }
            }),
        )
        .into_page("/dashboard", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 0);
        assert!(value["props"].get("audit").is_none());
    }

    #[test]
    fn lazy_errors_are_preserved_during_partial_reloads() {
        let request = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Form"),
            (X_INERTIA_PARTIAL_DATA, "user"),
        ]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Form",
            InertiaProps::new()
                .value("user", json!({ "name": "Ada" }))
                .lazy("errors", {
                    let calls = Rc::clone(&calls);
                    move || {
                        calls.set(calls.get() + 1);
                        json!({ "name": "Required" })
                    }
                })
                .lazy("stats", || 10),
        )
        .into_page("/form", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 1);
        assert_eq!(value["props"]["user"]["name"], "Ada");
        assert_eq!(value["props"]["errors"]["name"], "Required");
        assert!(value["props"].get("stats").is_none());
    }

    #[test]
    fn deferred_props_emit_metadata_and_resolve_only_when_requested() {
        let request = request_context_from(&[]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Dashboard",
            InertiaProps::new().defer_group("metrics", "analytics", {
                let calls = Rc::clone(&calls);
                move || {
                    calls.set(calls.get() + 1);
                    json!({ "views": 10 })
                }
            }),
        )
        .into_page("/dashboard", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 0);
        assert!(value["props"].get("analytics").is_none());
        assert_eq!(value["deferredProps"], json!({ "metrics": ["analytics"] }));

        let request = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Dashboard"),
            (X_INERTIA_PARTIAL_DATA, "analytics"),
        ]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Dashboard",
            InertiaProps::new().defer_group("metrics", "analytics", {
                let calls = Rc::clone(&calls);
                move || {
                    calls.set(calls.get() + 1);
                    json!({ "views": 10 })
                }
            }),
        )
        .into_page("/dashboard", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 1);
        assert_eq!(value["props"]["analytics"]["views"], 10);
        assert!(value.get("deferredProps").is_none());
    }

    #[test]
    fn deferred_once_props_already_loaded_by_client_are_not_advertised() {
        let request =
            request_context_from(&[(X_INERTIA, "true"), (X_INERTIA_EXCEPT_ONCE_PROPS, "stats")]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Dashboard",
            InertiaProps::new().defer_once("stats", {
                let calls = Rc::clone(&calls);
                move || {
                    calls.set(calls.get() + 1);
                    10
                }
            }),
        )
        .into_page("/dashboard", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 0);
        assert!(value["props"].get("stats").is_none());
        assert!(value.get("deferredProps").is_none());
        assert_eq!(
            value["onceProps"]["stats"],
            json!({ "prop": "stats", "expiresAt": null })
        );
    }

    #[test]
    fn always_lazy_props_survive_partial_reload_filtering() {
        let request = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Dashboard"),
            (X_INERTIA_PARTIAL_DATA, "users"),
        ]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Dashboard",
            InertiaProps::new()
                .value("users", json!(["Ada"]))
                .always("auth", {
                    let calls = Rc::clone(&calls);
                    move || {
                        calls.set(calls.get() + 1);
                        json!({ "user": { "name": "Ada" } })
                    }
                }),
        )
        .into_page("/dashboard", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 1);
        assert_eq!(value["props"]["users"], json!(["Ada"]));
        assert_eq!(value["props"]["auth"]["user"]["name"], "Ada");
    }

    #[test]
    fn once_lazy_props_are_not_resolved_when_client_already_has_them() {
        let request =
            request_context_from(&[(X_INERTIA, "true"), (X_INERTIA_EXCEPT_ONCE_PROPS, "plans")]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Billing",
            InertiaProps::new().once("plans", {
                let calls = Rc::clone(&calls);
                move || {
                    calls.set(calls.get() + 1);
                    json!(["basic"])
                }
            }),
        )
        .into_page("/billing", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 0);
        assert!(value["props"].get("plans").is_none());
        assert_eq!(
            value["onceProps"]["plans"],
            json!({ "prop": "plans", "expiresAt": null })
        );

        let request = request_context_from(&[
            (X_INERTIA, "true"),
            (X_INERTIA_PARTIAL_COMPONENT, "Billing"),
            (X_INERTIA_PARTIAL_DATA, "plans"),
            (X_INERTIA_EXCEPT_ONCE_PROPS, "plans"),
        ]);
        let calls = Rc::new(Cell::new(0));
        let response = Inertia::response(
            "Billing",
            InertiaProps::new().once("plans", {
                let calls = Rc::clone(&calls);
                move || {
                    calls.set(calls.get() + 1);
                    json!(["basic"])
                }
            }),
        )
        .into_page("/billing", Some("version-1".into()), &request)
        .unwrap();
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(calls.get(), 1);
        assert_eq!(value["props"]["plans"], json!(["basic"]));
    }

    #[test]
    fn lazy_route_prop_roots_block_shared_props_even_when_omitted() {
        let request = request_context_from(&[]);
        let response = Inertia::response(
            "Dashboard",
            InertiaProps::new().optional("auth", || json!({ "user": { "name": "Route" } })),
        )
        .into_page("/dashboard", Some("version-1".into()), &request)
        .unwrap()
        .with_shared_props(vec![
            (
                "auth.user",
                json!({
                    "name": "Shared"
                }),
            ),
            ("appName", json!("Demo")),
        ]);
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(value["props"]["auth"]["user"]["name"], "Shared");
        assert_eq!(value["props"]["appName"], "Demo");
        assert_eq!(value["sharedProps"], json!(["auth", "appName"]));
    }

    #[test]
    fn empty_shared_props_are_a_noop() {
        let page = Page::new("Empty", Value::Null, "/empty")
            .with_shared_props(Vec::<(&str, Value)>::new());
        let value = serde_json::to_value(page).unwrap();

        assert_eq!(value["props"], Value::Null);
        assert!(value.get("sharedProps").is_none());
    }

    #[test]
    fn page_equality_ignores_internal_route_prop_tracking() {
        let request = request_context_from(&[]);
        let response = Inertia::response(
            "Users",
            json!({
                "auth": {
                    "user": {
                        "name": "Ada"
                    }
                }
            }),
        )
        .into_page("/users", Some("version-1".into()), &request)
        .unwrap();
        let manual = Page::from_parts(
            "Users",
            json!({
                "errors": {},
                "auth": {
                    "user": {
                        "name": "Ada"
                    }
                }
            }),
            "/users",
            Some("version-1".into()),
            PageMetadata::new(),
        );

        assert_eq!(response, manual);
    }
}
