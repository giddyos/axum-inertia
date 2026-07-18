use inertia_embed::embed_frontend;

fn main() {
    let _ = embed_frontend! {
        root: "../../../../crates/inertia-embed/tests/fixtures/valid/dist",
        entry: "src/main.ts",
        max_asset_size: 1,
    };
}
