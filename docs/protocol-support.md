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
| Deferred props | Partial | `protocol_v3::lazy_deferred_once::lazy_optional_and_deferred_props_follow_selection_rules` |
| Lazy and optional props | Partial | `protocol_v3::lazy_deferred_once::lazy_optional_and_deferred_props_follow_selection_rules` |
| Once props | Supported | `protocol_v3::lazy_deferred_once::once_props_support_exclusion_explicit_reload_and_expiration` |
| Shared props | Supported | `protocol_v3::shared_props::fixed_and_request_aware_shared_props_merge_and_dedupe_roots` |
| History flags | Supported | `protocol_v3::page_objects::page_serializes_all_supported_v3_metadata` |
| Scroll and infinite-scroll metadata | Supported | `protocol_v3::merge_and_scroll::*` |
| Reset metadata | Supported | `protocol_v3::merge_and_scroll::reset_removes_only_matching_metadata` |
| Errors prop and error-bag headers | Partial | `protocol_v3::errors::*` |
| External location redirects | Supported | `protocol_v3::redirects::external_locations_use_protocol_conflicts_and_fragments_redirect_header` |
| Write-method redirects | Supported | `protocol_v3::redirects::direct_location_and_application_redirects_are_method_aware` |
| Not-found passthrough | Supported | `protocol_v3::versioning::dynamic_version_is_resolved_per_request_and_not_found_passthrough` |
| Rescued deferred props | Partial | `protocol_v3::page_objects::metadata_for_filtered_props_is_removed_and_rescued_metadata_serializes` |
| Numeric asset versions | Not supported | Versions are strings only |
| Flash data reflashing | Not supported | Not implemented by this crate |
| Precognition | Not supported | Not implemented |
| SSR bridge | Not supported | Not implemented |
| Async prop resolvers | Not supported | Synchronous resolvers only |

Deferred, lazy, and optional prop support is marked `Partial` because the
current prop container supports synchronous resolvers only. Async prop
resolvers remain planned.

Errors support is marked `Partial` because the protocol header is parsed and
the `errors` prop shape is preserved, but the Axum integration does not provide
a framework-level validation error bag or flash-message integration.

Rescued-prop support is metadata-only: resolver failures are not caught and
converted into rescued values by the crate.
