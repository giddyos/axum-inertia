//! Eager and synchronously resolved route props.

mod eager;
mod lazy;
mod resolver;

pub use lazy::{InertiaProps, ScopedInertiaProps};
pub use resolver::IntoPageProps;
