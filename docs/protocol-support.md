# Protocol Support Matrix

This matrix covers the Axum adapter and the framework-neutral Inertia protocol
core. The "Verified by" column lists representative tests.

Status vocabulary:

- `Supported`: implemented and covered by representative tests.
- `Partial`: implemented with a documented limitation.
- `Not supported`: not implemented.

| Feature | Status | Verified by |
| --- | --- | --- |
| Initial HTML response | Supported | `protocol_v3::basic_responses::initial_visit_returns_html_page` |
| JSON Inertia response | Supported | `protocol_v3::basic_responses::inertia_visit_returns_json_page` |
| Asset version conflict | Supported | `protocol_v3::versioning::stale_get_returns_conflict_before_handler` |
| Dynamic asset version | Supported | `protocol_v3::versioning::dynamic_version_is_resolved_per_request_and_not_found_passthrough` |
| Query-string and local page URLs | Supported | `protocol_v3::basic_responses::absolute_form_request_uses_local_page_url` |
| Request header parsing | Supported | `protocol_v3::request_headers::request_context_parses_all_supported_headers` |
| Partial reloads and component mismatch | Supported | `protocol_v3::partial_reloads::*` |
| Merge and deep-merge metadata | Supported | `protocol_v3::page_objects::page_serializes_all_supported_v3_metadata` |
| Deferred props | Supported | `unified_props::optional_deferred_always_and_partial_precedence_match_protocol` |
| Lazy and optional props | Supported | `unified_props::unselected_resolvers_do_not_construct_their_futures` |
| Once props | Supported | `protocol_v3::lazy_deferred_once::once_props_support_exclusion_explicit_reload_and_expiration` |
| Shared props | Supported | `protocol_v3::shared_props::fixed_and_request_aware_shared_props_merge_and_dedupe_roots` |
| History flags | Supported | `protocol_v3::page_objects::page_serializes_all_supported_v3_metadata` |
| Scroll and infinite-scroll metadata | Supported | `protocol_v3::merge_and_scroll::*` |
| Reset metadata | Supported | `protocol_v3::merge_and_scroll::reset_removes_only_matching_metadata` |
| Errors prop and error-bag headers | Supported | `forms_validation::invalid_json_redirects_before_handler_and_request_bag_wins` |
| External location redirects | Supported | `protocol_v3::redirects::external_locations_use_protocol_conflicts_and_fragments_redirect_header` |
| Write-method redirects | Supported | `protocol_v3::redirects::direct_location_and_application_redirects_are_method_aware` |
| Not-found passthrough | Supported | `protocol_v3::versioning::dynamic_version_is_resolved_per_request_and_not_found_passthrough` |
| Rescued deferred props | Supported | `unified_props::rescued_failures_are_omitted_reported_and_deterministic` |
| Numeric asset versions | Supported | `vite::numeric_asset_versions_retain_json_scalar_and_normalize_headers` |
| Flash data reflashing | Supported | `transient_flash::stale_version_conflict_reflashes_without_running_handler` |
| Precognition | Not supported | Not implemented |
| SSR bridge | Not supported | Not implemented |
| Async prop resolvers | Supported | `unified_props::selected_async_resolvers_run_concurrently` |

Async prop selection happens before resolver invocation, so unselected lazy,
optional, deferred, or once futures are never constructed or polled. Selected
resolvers execute concurrently, while rescued failures are reported,
deterministically omitted, and represented in `rescuedProps`.

Validation failures use redirect-back transient state rather than `422` JSON.
Errors and optional redacted old input appear on the next page; flash values
use the separate `page.flash` namespace and are consumed once.
