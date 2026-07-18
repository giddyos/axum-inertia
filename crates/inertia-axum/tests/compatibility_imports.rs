#![allow(missing_docs, unused_imports)]

use inertia_axum::{
    AssetContext, AssetProvider, AssetVersion, Component, ConfigError, Form, Inertia, InertiaApp,
    InertiaAppBuilder, InertiaForm, InertiaPage, InertiaProps, PendingPage, Redirect,
    RouterInertiaExt, Share, advanced, compat, prelude,
};

#[test]
fn practical_root_imports_and_compatibility_modules_remain_available() {
    let _ = std::any::type_name::<AssetContext<'static>>();
    let _ = std::any::type_name::<AssetVersion>();
    let _ = std::any::type_name::<Component>();
    let _ = std::any::type_name::<ConfigError>();
    let _ = std::any::type_name::<Inertia>();
    let _ = std::any::type_name::<InertiaApp>();
    let _ = std::any::type_name::<InertiaAppBuilder>();
    let _ = std::any::type_name::<PendingPage>();
    let _ = std::any::type_name::<Redirect>();
}
