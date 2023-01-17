//! Contains functionality related to validators

pub mod prelude {
    //! Contains useful re-exports

    pub use iroha_data_model::{permission::validator::Verdict, prelude::*};
    pub use iroha_wasm_derive::validator_entrypoint as entrypoint;

    pub use super::traits::Token;
    #[cfg(feature = "debug")]
    pub use crate::DebugExpectExt as _;
    pub use crate::EvaluateOnHost as _;
}

pub mod macros {
    //! Contains useful macros

    /// Shortcut for `return Verdict::Pass`.
    #[macro_export]
    macro_rules! pass {
        () => {
            return ::iroha_wasm::data_model::permission::validator::Verdict::Pass;
        };
    }

    /// Macro to return [`Verdict::Pass`](crate::data_model::permission::validator::Verdict::Pass)
    /// if the expression is `true`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// pass_if!(asset_id.account_id == authority);
    /// ```
    #[macro_export]
    macro_rules! pass_if {
        ($e:expr) => {
            if $e {
                return $crate::data_model::permission::validator::Verdict::Pass;
            }
        };
    }

    /// Shortcut for `return Verdict::Deny(...)`.
    ///
    /// Supports [`format!`](alloc::format) syntax as well as any expression returning [`String`](alloc::string::String).
    ///
    /// # Example
    ///
    /// ```no_run
    /// deny!("Some reason");
    /// deny!("Reason: {}", reason);
    /// deny!("Reason: {reason}");
    /// deny!(get_reason());
    /// ```
    #[macro_export]
    macro_rules! deny {
        ($l:literal $(,)?) => {
            return $crate::data_model::permission::validator::Verdict::Deny(
                ::alloc::fmt::format(::core::format_args!($l))
            )
        };
        ($e:expr $(,)?) =>{
            return $crate::data_model::permission::validator::Verdict::Deny($e)
        };
        ($fmt:expr, $($arg:tt)*) => {
            return $crate::data_model::permission::validator::Verdict::Deny(
                ::alloc::format!($fmt, $($arg)*)
            )
        };
    }

    /// Macro to return [`Verdict::Deny`](crate::data_model::permission::validator::Verdict::Deny)
    /// if the expression is `true`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// deny_if!(asset_id.account_id != authority, "You have to be an asset owner");
    /// deny_if!(asset_id.account_id != authority, "You have to be an {} owner", asset_id);
    /// deny_if!(asset_id.account_id != authority, construct_reason(&asset_id));
    /// ```
    #[macro_export]
    macro_rules! deny_if {
        ($e:expr, $l:literal $(,)?) => {
            if $e {
                deny!($l);
            }
        };
        ($e:expr, $r:expr $(,)?) =>{
            if $e {
                deny!($r);
            }
        };
        ($e:expr, $fmt:expr, $($arg:tt)*) => {
            if $e {
                deny!($fmt, $($arg)*);
            }
        };
    }

    /// Macro to parse literal as a type. Panics if failed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use iroha_wasm::parse;
    /// use iroha_wasm::data_model::prelude::*;
    ///
    /// let account_id = parse!("alice@wonderland" as <Account as Identifiable>::Id);
    /// ```
    #[macro_export]
    macro_rules! parse {
        ($l:literal as _) => {
            compile_error!(
                "Don't use `_` as a type in this macro, \
                 otherwise panic message would be less informative"
            )
        };
        ($l:literal as $t:ty) => {
            $l.parse::<$t>().dbg_expect(concat!(
                "Failed to parse `",
                $l,
                "` as `",
                stringify!($t),
                "`"
            ))
        };
    }

    /// Macro to declare a permission token.
    ///
    /// TODO: Replace with **derive** macro
    #[macro_export]
    macro_rules! declare_token {
        (
            $(#[$outer_meta:meta])* // Structure attributes
            $ident:ident {          // Structure definition
                $(
                    $(#[$inner_meta:meta])* // Field attributes
                    $param_name:ident ($param_string:literal): $param_typ:ty
                 ),* $(,)? // allow trailing comma
            },
            $definition_id:literal // Token definition id
        ) => {

            // For tokens with no parameters
            #[allow(missing_copy_implementations)]
            $(#[$outer_meta])*
            ///
            /// A wrapper around [PermissionToken](::iroha_wasm::data_model::permission::Token).
            pub struct $ident
            where
                $(
                    $param_typ: ::core::convert::Into<::iroha_wasm::data_model::Value>
                        + ::iroha_wasm::data_model::permission::token::ValueTrait,
                )*
            {
                $(
                    $(#[$inner_meta])*
                    #[doc = concat!(
                        "\nCorresponding parameter name in generic `[PermissionToken]` is `\"",
                        $param_string,
                        "\"`.",
                    )]
                    pub $param_name : $param_typ
                 ),*
            }

            impl $ident {
                fn into_permission_token(&self) -> ::iroha_wasm::data_model::permission::Token {
                    ::iroha_wasm::data_model::permission::Token::new(::iroha_wasm::parse!(
                        $definition_id as <
                            ::iroha_wasm::data_model::permission::token::Definition
                            as
                            ::iroha_wasm::data_model::prelude::Identifiable
                        >::Id
                    ))
                    .with_params([
                        $((
                            ::iroha_wasm::parse!($param_string
                                as ::iroha_wasm::data_model::prelude::Name),
                            self.$param_name.clone().into()
                        )),*
                    ])
                }
            }

            impl ::iroha_wasm::validator::traits::Token for $ident {
                fn is_owned_by(
                    &self,
                    account_id: &<
                        ::iroha_wasm::data_model::prelude::Account
                        as
                        ::iroha_wasm::data_model::prelude::Identifiable
                    >::Id
                ) -> bool {
                    use ::iroha_wasm::Execute as _;

                    ::iroha_wasm::data_model::prelude::QueryBox::DoesAccountHavePermissionToken(
                        ::iroha_wasm::data_model::prelude::DoesAccountHavePermissionToken {
                            account_id: account_id.clone().into(),
                            permission_token: self.into_permission_token(),
                        }
                    )
                    .execute()
                    .try_into()
                    .dbg_expect("Failed to convert `DoesAccountHavePermission` query result into `bool`")
                }
            }
        };
    #[cfg(test)]
    mod tests {
        //! Tests in this modules can't be doc-tests because of `compile_error!` on native target
        //! and `webassembly-test-runner` on wasm target.

        use webassembly_test::webassembly_test;

        use crate::{
            alloc::borrow::ToOwned as _, data_model::permission::validator::Verdict, deny,
        };

        #[webassembly_test]
        fn test_deny() {
            let a = || deny!("Some reason");
            assert_eq!(a(), Verdict::Deny("Some reason".to_owned()));

            let get_reason = || "Reason from expression".to_owned();
            let b = || deny!(get_reason());
            assert_eq!(b(), Verdict::Deny("Reason from expression".to_owned()));

            let mes = "Format message";
            let c = || deny!("Reason: {}", mes);
            assert_eq!(c(), Verdict::Deny("Reason: Format message".to_owned()));

            let mes = "Advanced format message";
            let d = || deny!("Reason: {mes}");
            assert_eq!(
                d(),
                Verdict::Deny("Reason: Advanced format message".to_owned())
            );
        }

        #[webassembly_test]
        fn test_deny_if() {
            let a = || {
                deny_if!(true, "Some reason");
                unreachable!()
            };
            assert_eq!(a(), Verdict::Deny("Some reason".to_owned()));

            let get_reason = || "Reason from expression".to_owned();
            let b = || {
                deny_if!(true, get_reason());
                unreachable!()
            };
            assert_eq!(b(), Verdict::Deny("Reason from expression".to_owned()));

            let mes = "Format message";
            let c = || {
                deny_if!(true, "Reason: {}", mes);
                unreachable!()
            };
            assert_eq!(c(), Verdict::Deny("Reason: Format message".to_owned()));

            let mes = "Advanced format message";
            let d = || {
                deny_if!(true, "Reason: {mes}");
                unreachable!()
            };
            assert_eq!(
                d(),
                Verdict::Deny("Reason: Advanced format message".to_owned())
            );
        }
    }
}

pub mod traits {
    //! Contains traits related to validators

    /// Trait for tokens declared with [`declare_token!`] macro inside validator's code.
    /// Provides a way to check if token is owned by the account.
    /// Useful for generic functions.
    pub trait Token {
        /// Check if token is owned by the account using evaluation on host.
        fn is_owned_by(
            &self,
            account_id: &<
                crate::data_model::prelude::Account
                as
                crate::data_model::prelude::Identifiable
            >::Id,
        ) -> bool;
    }
}

pub mod utils {
    //! Contains some utils for validators

    use crate::*;

    /// Check if `authority` is the owner of `asset_definition_id`.
    ///
    /// Wrapper around [`IsAssetDefinitionOwner`](crate::data_model::prelude::IsAssetDefinitionOwner) query.
    pub fn is_asset_definition_owner(
        asset_definition_id: &<AssetDefinition as Identifiable>::Id,
        authority: &<Account as Identifiable>::Id,
    ) -> bool {
        QueryBox::from(IsAssetDefinitionOwner::new(
            asset_definition_id.clone(),
            authority.clone(),
        ))
        .execute()
        .try_into()
        .dbg_expect("Failed to convert `IsAssetDefinitionOwner` query result into `bool`")
    }
}
