//! Compiled-wire contract tests for the downstream-facing derive.

#![cfg(feature = "typegen")]
#![allow(dead_code)]

use inertia_axum::{
    __private::typegen::{Config, TS},
    InertiaType,
};
use serde::Serialize;

#[derive(Serialize, InertiaType)]
#[serde(rename_all = "camelCase")]
struct Address {
    postal_code: String,
}

#[derive(Serialize, InertiaType)]
struct User<T> {
    id: u32,
    address: T,
}

#[derive(Serialize, InertiaType)]
#[serde(tag = "kind", content = "payload")]
enum Event {
    Created { user: User<Address> },
    Deleted,
}

#[test]
fn inertia_type_preserves_wire_names_and_dependencies() {
    let config = Config::default();
    assert!(Address::decl(&config).contains("postalCode: string"));
    assert!(User::<Address>::decl(&config).contains("address: T"));
    let event = Event::decl(&config);
    assert!(event.contains("Created"));
    assert!(event.contains("User<Address>"));
    let dependencies = Event::dependencies(&config);
    assert!(
        dependencies
            .iter()
            .any(|dependency| dependency.ts_name == "User")
    );
    assert!(
        dependencies
            .iter()
            .any(|dependency| dependency.ts_name == "Address")
    );
}
