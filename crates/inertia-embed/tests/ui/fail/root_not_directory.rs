use inertia_embed::embed_frontend;

fn main() {
    let _ = embed_frontend! {
        root: "../../../../crates/inertia-embed/Cargo.toml",
        entry: "src/main.ts",
    };
}
