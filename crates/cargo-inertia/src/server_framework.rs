//! Rust web frameworks supported by full-project scaffolding.

/// A server framework selected for a generated Inertia application.
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[cfg_attr(feature = "templates", derive(serde::Serialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerFramework {
    /// Axum.
    Axum,
    /// Actix Web.
    ActixWeb,
    /// Rocket.
    Rocket,
}

impl ServerFramework {
    /// Human-readable framework name.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Axum => "Axum",
            Self::ActixWeb => "Actix Web",
            Self::Rocket => "Rocket",
        }
    }

    /// Adapter package used by generated projects.
    pub const fn adapter_crate(self) -> &'static str {
        match self {
            Self::Axum => "inertia-axum",
            Self::ActixWeb => "inertia-actix",
            Self::Rocket => "inertia-rocket",
        }
    }
}
