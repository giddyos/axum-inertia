use inertia_embed::{EmbeddedFrontend, embed_frontend};

static FRONTEND: EmbeddedFrontend = embed_frontend! {
    root: "../../../../crates/inertia-embed/tests/fixtures/valid/dist",
    manifest: "../../../../crates/inertia-embed/tests/fixtures/valid/dist/.vite/manifest.json",
    entry: "src/main.ts",
    include_source_maps: true,
    include_hidden: true,
    cache: "auto",
    max_manifest_size: 16777216,
    max_files: 100000,
    max_asset_size: 536870912,
    max_total_size: 0,
};

fn main() {
    assert!(FRONTEND.find("assets/source.js.map").is_some());
}
