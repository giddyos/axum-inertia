use inertia_embed::embed_frontend;

fn main() {
    let _ = embed_frontend! {
        entry: "src/main.ts",
    };
}
