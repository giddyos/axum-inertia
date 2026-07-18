# Actix Web embedded frontend

Debug builds load the Vite development server. Release builds embed the generated
Vite manifest and assets into the Rust binary:

```console
cargo build --release -p actix-embedded
```
