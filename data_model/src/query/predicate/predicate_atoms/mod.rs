pub mod account;
pub mod asset;
pub mod block;
pub mod domain;
pub mod parameter;
pub mod peer;
pub mod permission;
pub mod role;
pub mod trigger;

use iroha_crypto::PublicKey;
use iroha_schema::IntoSchema;
use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

use super::{
    predicate_ast_extensions::AstPredicateExt as _,
    predicate_combinators::{AndAstPredicate, NotAstPredicate, OrAstPredicate},
    projectors::BaseProjector,
    AstPredicate, CompoundPredicate, HasPredicateBox, HasPrototype,
};
use crate::{metadata::Metadata, name::Name, prelude::PredicateTrait};

/// Adds common methods to a predicate box.
///
/// Implements:
/// 1. `build` and `build_fragment` methods for building a predicate using the dsl.
/// 2. base-case `AstPredicate` for the predicate box (emits an atom expression).
/// 3. `Not`, `BitAnd`, and `BitOr` operators for combining predicates.
macro_rules! impl_predicate_box {
    ($($ty:ty),+: $predicate_ty:ty) => {
        impl $predicate_ty {
            pub fn build<F, O>(predicate: F) -> CompoundPredicate<Self>
            where
                F: FnOnce(<Self as HasPrototype>::Prototype<BaseProjector<Self>>) -> O,
                O: AstPredicate<Self>,
            {
                predicate(Default::default()).normalize()
            }

            pub fn build_fragment<F, O>(predicate: F) -> O
            where
                F: FnOnce(<Self as HasPrototype>::Prototype<BaseProjector<Self>>) -> O,
                O: AstPredicate<Self>,
            {
                predicate(Default::default())
            }
        }

        $(
            impl HasPredicateBox for $ty {
                type PredicateBoxType = $predicate_ty;
            }
        )+

        impl AstPredicate<$predicate_ty> for $predicate_ty {
            fn normalize_with_proj<OutputType, Proj>(self, proj: Proj) -> CompoundPredicate<OutputType>
            where
                Proj: Fn($predicate_ty) -> OutputType + Copy,
            {
                CompoundPredicate::Atom(proj(self))
            }
        }

        impl core::ops::Not for $predicate_ty
        where
            Self: AstPredicate<$predicate_ty>,
        {
            type Output = NotAstPredicate<Self>;

            fn not(self) -> Self::Output {
                NotAstPredicate(self)
            }
        }

        impl<PRhs> core::ops::BitAnd<PRhs> for $predicate_ty
        where
            Self: AstPredicate<$predicate_ty>,
            PRhs: AstPredicate<$predicate_ty>,
        {
            type Output = AndAstPredicate<Self, PRhs>;

            fn bitand(self, rhs: PRhs) -> Self::Output {
                AndAstPredicate(self, rhs)
            }
        }

        impl<PRhs> core::ops::BitOr<PRhs> for $predicate_ty
        where
            Self: AstPredicate<$predicate_ty>,
            PRhs: AstPredicate<$predicate_ty>,
        {
            type Output = OrAstPredicate<Self, PRhs>;

            fn bitor(self, rhs: PRhs) -> Self::Output {
                OrAstPredicate(self, rhs)
            }
        }
    };
}
pub(crate) use impl_predicate_box;

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, Deserialize, Serialize, IntoSchema)]
pub enum StringPredicateBox {
    /// Forward to [`String`] equality.
    Equals(String),
    /// Forward to [`str::contains()`]
    Contains(String),
    /// Forward to [`str::starts_with()`]
    StartsWith(String),
    /// Forward to [`str::ends_with()`]
    EndsWith(String),
}

impl_predicate_box!(String, Name: StringPredicateBox);

impl<T> PredicateTrait<T> for StringPredicateBox
where
    T: AsRef<str>,
{
    fn applies(&self, input: &T) -> bool {
        let input = input.as_ref();
        match self {
            StringPredicateBox::Contains(content) => input.contains(content),
            StringPredicateBox::StartsWith(content) => input.starts_with(content),
            StringPredicateBox::EndsWith(content) => input.ends_with(content),
            StringPredicateBox::Equals(content) => *input == *content,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, Deserialize, Serialize, IntoSchema)]
pub enum MetadataPredicateBox {
    // TODO: populate this with something. Seeing as how we can change it to be just a JsonString, not populating it right now
}

impl_predicate_box!(Metadata: MetadataPredicateBox);

impl PredicateTrait<Metadata> for MetadataPredicateBox {
    fn applies(&self, _input: &Metadata) -> bool {
        match self {
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, Deserialize, Serialize, IntoSchema)]
pub enum PublicKeyPredicateBox {
    // object-specific predicates
    Equals(PublicKey),
}

impl_predicate_box!(PublicKey: PublicKeyPredicateBox);

impl PredicateTrait<PublicKey> for PublicKeyPredicateBox {
    fn applies(&self, input: &PublicKey) -> bool {
        match self {
            PublicKeyPredicateBox::Equals(expected) => expected == input,
        }
    }
}
