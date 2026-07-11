use crate::Frontend;
use std::{fs, path::Path};

pub fn run(root: &Path, framework: Frontend) -> Result<(), String> {
    let frontend = root.join("frontend");
    if frontend.exists() {
        return Err(format!("{} already exists", frontend.display()));
    }
    fs::create_dir_all(frontend.join("src/Pages")).map_err(|error| error.to_string())?;
    let (dependency, plugin, extension, main, home) = match framework {
        Frontend::Svelte => (
            "@sveltejs/vite-plugin-svelte",
            "svelte",
            "svelte",
            SVELTE_MAIN,
            SVELTE_HOME,
        ),
        Frontend::React => (
            "@vitejs/plugin-react",
            "react",
            "tsx",
            REACT_MAIN,
            REACT_HOME,
        ),
        Frontend::Vue => ("@vitejs/plugin-vue", "vue", "vue", VUE_MAIN, VUE_HOME),
    };
    let package = format!(
        r#"{{
  "private": true,
  "type": "module",
  "scripts": {{ "dev": "vite", "build": "vite build" }},
  "dependencies": {{ "@inertiajs/core": "^3.0.0", "@inertiajs/{plugin}": "^3.0.0", "{plugin}": "latest" }},
  "devDependencies": {{ "vite": "latest", "{dependency}": "latest", "typescript": "latest" }}
}}
"#
    );
    write(&frontend.join("package.json"), &package)?;
    write(&frontend.join("vite.config.ts"), &vite_config(framework))?;
    write(&frontend.join("src/main.ts"), main)?;
    write(&frontend.join(format!("src/Pages/Home.{extension}")), home)?;
    println!("Created {} frontend in {}", plugin, frontend.display());
    println!("\nRust setup:\n\n.inertia(InertiaApp::vite(\"frontend\").build()?)");
    Ok(())
}

fn vite_config(framework: Frontend) -> String {
    let (import, plugin) = match framework {
        Frontend::Svelte => (
            "import { svelte } from '@sveltejs/vite-plugin-svelte';",
            "svelte()",
        ),
        Frontend::React => ("import react from '@vitejs/plugin-react';", "react()"),
        Frontend::Vue => ("import vue from '@vitejs/plugin-vue';", "vue()"),
    };
    format!(
        "import {{ defineConfig }} from 'vite';\n{import}\n\nexport default defineConfig({{ plugins: [{plugin}], build: {{ manifest: true, rollupOptions: {{ input: 'src/main.ts' }} }} }});\n"
    )
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| error.to_string())
}

const SVELTE_MAIN: &str = "import { createInertiaApp } from '@inertiajs/svelte';\nimport { mount } from 'svelte';\ncreateInertiaApp({ resolve: name => import(`./Pages/${name}.svelte`), setup: ({ el, App, props }) => mount(App, { target: el, props }) });\n";
const SVELTE_HOME: &str = "<script lang=\"ts\">let { greeting = 'Hello' } = $props();</script>\n<h1>{greeting} from inertia-axum</h1>\n";
const REACT_MAIN: &str = "import { createInertiaApp } from '@inertiajs/react';\nimport { createRoot } from 'react-dom/client';\ncreateInertiaApp({ resolve: name => import(`./Pages/${name}.tsx`), setup: ({ el, App, props }) => createRoot(el).render(<App {...props} />) });\n";
const REACT_HOME: &str = "export default function Home({ greeting = 'Hello' }) { return <h1>{greeting} from inertia-axum</h1>; }\n";
const VUE_MAIN: &str = "import { createApp, h } from 'vue';\nimport { createInertiaApp } from '@inertiajs/vue3';\ncreateInertiaApp({ resolve: name => import(`./Pages/${name}.vue`), setup: ({ el, App, props, plugin }) => createApp({ render: () => h(App, props) }).use(plugin).mount(el) });\n";
const VUE_HOME: &str = "<script setup lang=\"ts\">withDefaults(defineProps<{ greeting?: string }>(), { greeting: 'Hello' });</script>\n<template><h1>{{ greeting }} from inertia-axum</h1></template>\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_each_supported_framework_skeleton() {
        for (name, framework, extension) in [
            ("svelte", Frontend::Svelte, "svelte"),
            ("react", Frontend::React, "tsx"),
            ("vue", Frontend::Vue, "vue"),
        ] {
            let root = std::env::temp_dir()
                .join(format!("cargo-inertia-init-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            run(&root, framework).unwrap();
            assert!(root.join("frontend/package.json").is_file());
            assert!(root.join("frontend/vite.config.ts").is_file());
            assert!(root.join("frontend/src/main.ts").is_file());
            assert!(
                root.join(format!("frontend/src/Pages/Home.{extension}"))
                    .is_file()
            );
            assert!(
                run(&root, framework)
                    .unwrap_err()
                    .contains("already exists")
            );
            fs::remove_dir_all(root).unwrap();
        }
    }
}
