use inertia_embed::embed_frontend;

fn main() {
    let _ = embed_frontend! {
        root: "../../../../crates/inertia-embed/tests/fixtures/valid/dist",
        manifest: "../../../../crates/inertia-embed/tests/fixtures/valid/dist/.vite/duplicate.json",
        entry: "src/main.ts",
    };
}
