use inertia_embed::{EmbeddedFrontend, embed_frontend};

static FRONTEND: EmbeddedFrontend = embed_frontend! {
    root: "../../../../crates/inertia-embed/tests/fixtures/valid/dist",
    entry: "src/main.ts",
};

fn main() {
    assert_eq!(FRONTEND.public_path, "/build");
}
