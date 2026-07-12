//! Binary-target half of the Phase 0 filtered-test fixture.

fn main() {}

#[cfg(test)]
mod tests {
    #[test]
    fn __inertia_typegen_binary_target_spike() {
        // A filtered test in this binary target proves Cargo can execute
        // generated exporters for applications without a library target.
        assert_eq!(2 + 2, 4);
    }
}
