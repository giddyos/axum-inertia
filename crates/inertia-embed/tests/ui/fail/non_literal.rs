use inertia_embed::embed_frontend;

const ROOT: &str = "../../../../crates/inertia-embed/tests/fixtures/valid/dist";

fn main() {
    let _ = embed_frontend! {
        root: ROOT,
        entry: "src/main.ts",
    };
}
