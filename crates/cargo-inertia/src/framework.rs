//! Supported frontend frameworks.

/// A frontend framework that can be scaffolded by `cargo inertia`.
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[cfg_attr(feature = "templates", derive(serde::Serialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Framework {
    /// Svelte.
    Svelte,
    /// React.
    React,
    /// Vue.
    Vue,
}
