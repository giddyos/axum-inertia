//! Throwaway validation fixture for the Phase 0 type-generation engine gate.

#![allow(dead_code)]

#[cfg(test)]
mod spike {
    use ia::__private::typegen::{Config, TS, TypeVisitor};
    use serde::Serialize;
    use std::{any::TypeId, collections::BTreeSet};

    #[derive(Serialize)]
    struct User<T> {
        id: u32,
        profile: T,
    }

    #[derive(Serialize)]
    struct Profile {
        display_name: String,
        quota: u64,
    }

    #[derive(Serialize)]
    struct UsersPage {
        users: Vec<User<Profile>>,
    }

    #[derive(ia::__private::typegen::TS)]
    #[ts(crate = "ia::__private::typegen", rename = "User")]
    struct UserProxy<T> {
        id: u32,
        profile: T,
    }

    #[derive(ia::__private::typegen::TS)]
    #[ts(crate = "ia::__private::typegen", rename = "Profile")]
    struct ProfileProxy {
        display_name: String,
        quota: u64,
    }

    #[derive(ia::__private::typegen::TS)]
    #[ts(crate = "ia::__private::typegen", rename = "UsersPageProps")]
    struct UsersPageProxy {
        users: Vec<User<Profile>>,
    }

    macro_rules! delegate_ts {
        ($source:ty => $proxy:ty) => {
            impl TS for $source {
                type WithoutGenerics = <$proxy as TS>::WithoutGenerics;
                type OptionInnerType = Self;
                const IS_OPTION: bool = <$proxy as TS>::IS_OPTION;
                const IS_ENUM: bool = <$proxy as TS>::IS_ENUM;
                fn docs() -> Option<String> {
                    <$proxy as TS>::docs()
                }
                fn ident(config: &Config) -> String {
                    <$proxy as TS>::ident(config)
                }
                fn name(config: &Config) -> String {
                    <$proxy as TS>::name(config)
                }
                fn inline(config: &Config) -> String {
                    <$proxy as TS>::inline(config)
                }
                fn inline_flattened(config: &Config) -> String {
                    <$proxy as TS>::inline_flattened(config)
                }
                fn visit_dependencies(visitor: &mut impl TypeVisitor)
                where
                    Self: 'static,
                {
                    <$proxy as TS>::visit_dependencies(visitor);
                }
                fn visit_generics(visitor: &mut impl TypeVisitor)
                where
                    Self: 'static,
                {
                    <$proxy as TS>::visit_generics(visitor);
                }
                fn decl(config: &Config) -> String {
                    <$proxy as TS>::decl(config)
                }
                fn decl_concrete(config: &Config) -> String {
                    <$proxy as TS>::decl_concrete(config)
                }
                fn output_path() -> Option<std::path::PathBuf> {
                    <$proxy as TS>::output_path()
                }
            }
        };
    }

    impl<T> TS for User<T>
    where
        T: TS + 'static,
    {
        type WithoutGenerics = <UserProxy<T> as TS>::WithoutGenerics;
        type OptionInnerType = Self;
        const IS_OPTION: bool = <UserProxy<T> as TS>::IS_OPTION;
        const IS_ENUM: bool = <UserProxy<T> as TS>::IS_ENUM;
        fn docs() -> Option<String> {
            <UserProxy<T> as TS>::docs()
        }
        fn ident(config: &Config) -> String {
            <UserProxy<T> as TS>::ident(config)
        }
        fn name(config: &Config) -> String {
            <UserProxy<T> as TS>::name(config)
        }
        fn inline(config: &Config) -> String {
            <UserProxy<T> as TS>::inline(config)
        }
        fn inline_flattened(config: &Config) -> String {
            <UserProxy<T> as TS>::inline_flattened(config)
        }
        fn visit_dependencies(visitor: &mut impl TypeVisitor)
        where
            Self: 'static,
        {
            <UserProxy<T> as TS>::visit_dependencies(visitor);
        }
        fn visit_generics(visitor: &mut impl TypeVisitor)
        where
            Self: 'static,
        {
            <UserProxy<T> as TS>::visit_generics(visitor);
        }
        fn decl(config: &Config) -> String {
            <UserProxy<T> as TS>::decl(config)
        }
        fn decl_concrete(config: &Config) -> String {
            <UserProxy<T> as TS>::decl_concrete(config)
        }
        fn output_path() -> Option<std::path::PathBuf> {
            <UserProxy<T> as TS>::output_path()
        }
    }

    delegate_ts!(Profile => ProfileProxy);
    delegate_ts!(UsersPage => UsersPageProxy);

    struct Collector<'a> {
        config: &'a Config,
        visited: BTreeSet<TypeId>,
        declarations: BTreeSet<String>,
    }

    impl Collector<'_> {
        fn collect<T: TS + 'static + ?Sized>(&mut self) {
            if !self.visited.insert(TypeId::of::<T>()) {
                return;
            }
            if T::output_path().is_some() {
                self.declarations.insert(T::decl(self.config));
            }
            T::visit_dependencies(self);
        }
    }

    impl TypeVisitor for Collector<'_> {
        fn visit<T: TS + 'static + ?Sized>(&mut self) {
            self.collect::<T>();
        }
    }

    pub(crate) fn prove_engine() {
        let sentinel = "__INERTIA_LARGE_INTEGER__";
        let config = Config::default().with_large_int(sentinel);
        let mut collector = Collector {
            config: &config,
            visited: BTreeSet::new(),
            declarations: BTreeSet::new(),
        };
        collector.collect::<UsersPage>();
        let output = collector
            .declarations
            .into_iter()
            .collect::<Vec<_>>()
            .join("\n");
        assert!(output.contains("UsersPageProps"));
        assert!(output.contains("User<T>"));
        assert!(output.contains("Profile"));
        assert!(output.contains(sentinel));
        assert!(output.replace(sentinel, "number").contains("quota: number"));
    }

    #[test]
    fn __inertia_typegen_library_target_spike() {
        prove_engine();
    }
}
