use inertia_axum_macros::InertiaForm;

#[derive(InertiaForm)]
#[inertia(validator = "magic")]
struct Invalid { value: String }

fn main() {}
