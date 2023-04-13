//! This module contains structures and implementations related to the cryptographic parts of the Iroha.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::std_instead_of_alloc, clippy::arithmetic_side_effects)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod hash;
mod merkle;
mod multihash;
mod signature;
mod varint;

#[cfg(not(feature = "std"))]
use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString as _},
    vec::Vec,
};
use core::{fmt, str::FromStr};

#[cfg(feature = "base64")]
pub use base64;
use derive_more::{DebugCustom, Display};
use getset::Getters;
pub use hash::*;
use iroha_ffi::FfiType;
use iroha_schema::IntoSchema;
pub use merkle::MerkleTree;
use multihash::Multihash;
use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
pub use signature::*;
#[cfg(feature = "std")]
pub use ursa;
#[cfg(feature = "std")]
use ursa::{
    keys::{KeyGenOption as UrsaKeyGenOption, PrivateKey as UrsaPrivateKey},
    signatures::{
        bls::{normal::Bls as BlsNormal, small::Bls as BlsSmall},
        ed25519::Ed25519Sha512,
        secp256k1::EcdsaSecp256k1Sha256,
        SignatureScheme,
    },
};

// Hiding constants is a bad idea. For one, you're forcing the user to
// create temporaries, but also you're not actually hiding any
// information that can be used in malicious ways. If you want to hide
// these, I'd prefer inlining them instead.

/// ed25519
pub const ED_25519: &str = "ed25519";
/// secp256k1
pub const SECP_256_K1: &str = "secp256k1";
/// bls normal
pub const BLS_NORMAL: &str = "bls_normal";
/// bls small
pub const BLS_SMALL: &str = "bls_small";

/// Error indicating algorithm could not be found
#[derive(Debug, Clone, Copy, Display, IntoSchema)]
#[display(fmt = "Algorithm not supported")]
pub struct NoSuchAlgorithm;

#[cfg(feature = "std")]
impl std::error::Error for NoSuchAlgorithm {}

ffi::ffi_item! {
    /// Algorithm for hashing
    #[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, DeserializeFromStr, SerializeDisplay, Decode, Encode, FfiType, IntoSchema)]
    #[repr(u8)]
    pub enum Algorithm {
        /// Ed25519
        #[display(fmt = "{ED_25519}")]
        #[default]
        Ed25519,
        /// Secp256k1
        #[display(fmt = "{SECP_256_K1}")]
        Secp256k1,
        /// BlsNormal
        #[display(fmt = "{BLS_NORMAL}")]
        BlsNormal,
        /// BlsSmall
        #[display(fmt = "{BLS_SMALL}")]
        BlsSmall,
    }
}

impl FromStr for Algorithm {
    type Err = NoSuchAlgorithm;

    fn from_str(algorithm: &str) -> Result<Self, Self::Err> {
        match algorithm {
            ED_25519 => Ok(Algorithm::Ed25519),
            SECP_256_K1 => Ok(Algorithm::Secp256k1),
            BLS_NORMAL => Ok(Algorithm::BlsNormal),
            BLS_SMALL => Ok(Algorithm::BlsSmall),
            _ => Err(NoSuchAlgorithm),
        }
    }
}

/// Options for key generation
#[derive(Debug, Clone)]
pub enum KeyGenOption {
    /// Use seed
    UseSeed(Vec<u8>),
    /// Derive from private key
    FromPrivateKey(PrivateKey),
}

#[cfg(feature = "std")]
impl TryFrom<KeyGenOption> for UrsaKeyGenOption {
    type Error = NoSuchAlgorithm;

    fn try_from(key_gen_option: KeyGenOption) -> Result<Self, Self::Error> {
        match key_gen_option {
            KeyGenOption::UseSeed(seed) => Ok(UrsaKeyGenOption::UseSeed(seed)),
            KeyGenOption::FromPrivateKey(key) => {
                let algorithm = key.digest_function();

                match algorithm {
                    Algorithm::Ed25519 | Algorithm::Secp256k1 => {
                        Ok(Self::FromSecretKey(UrsaPrivateKey(key.payload)))
                    }
                    _ => Err(Self::Error {}),
                }
            }
        }
    }
}

/// Configuration of key generation
#[derive(Debug, Clone, Default)]
pub struct KeyGenConfiguration {
    /// Options
    pub key_gen_option: Option<KeyGenOption>,
    /// Algorithm
    pub algorithm: Algorithm,
}

impl KeyGenConfiguration {
    /// Use seed
    #[must_use]
    pub fn use_seed(mut self, seed: Vec<u8>) -> Self {
        self.key_gen_option = Some(KeyGenOption::UseSeed(seed));
        self
    }

    /// Use private key
    #[must_use]
    pub fn use_private_key(mut self, private_key: PrivateKey) -> Self {
        self.key_gen_option = Some(KeyGenOption::FromPrivateKey(private_key));
        self
    }

    /// With algorithm
    #[must_use]
    pub const fn with_algorithm(mut self, algorithm: Algorithm) -> Self {
        self.algorithm = algorithm;
        self
    }
}

ffi::ffi_item! {
    /// Pair of Public and Private keys.
    #[derive(Debug, Clone, PartialEq, Eq, Getters, Serialize, FfiType)]
    #[getset(get = "pub")]
    pub struct KeyPair {
        /// Public Key.
        public_key: PublicKey,
        /// Private Key.
        private_key: PrivateKey,
    }
}

#[cfg(feature = "std")]
impl From<NoSuchAlgorithm> for Error {
    fn from(_: NoSuchAlgorithm) -> Self {
        Self::NoSuchAlgorithm
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl KeyPair {
    /// Digest function
    pub fn digest_function(&self) -> Algorithm {
        self.private_key.digest_function()
    }

    /// Construct `KeyPair` from a matching pair of public and private key.
    /// It is up to the user to ensure that the given keys indeed make a pair.
    #[cfg(not(feature = "std"))]
    pub fn new_unchecked(public_key: PublicKey, private_key: PrivateKey) -> Self {
        Self {
            public_key,
            private_key,
        }
    }

    /// Construct `KeyPair`
    ///
    /// # Errors
    /// If public and private key don't match, i.e. if they don't make a pair
    #[cfg(feature = "std")]
    pub fn new(public_key: PublicKey, private_key: PrivateKey) -> Result<Self, Error> {
        let algorithm = private_key.digest_function();

        if algorithm != public_key.digest_function() {
            return Err(Error::KeyGen(String::from("Mismatch of key algorithms")));
        }

        if PublicKey::from(private_key.clone()) != public_key {
            return Err(Error::KeyGen(String::from("Key pair mismatch")));
        }

        Ok(Self {
            public_key,
            private_key,
        })
    }

    /// Generates a pair of Public and Private key with [`Algorithm::default()`] selected as generation algorithm.
    ///
    /// # Errors
    /// Fails if decoding fails
    #[cfg(feature = "std")]
    pub fn generate() -> Result<Self, Error> {
        Self::generate_with_configuration(KeyGenConfiguration::default())
    }

    /// Generates a pair of Public and Private key with the corresponding [`KeyGenConfiguration`].
    ///
    /// # Errors
    /// Fails if decoding fails
    #[cfg(feature = "std")]
    pub fn generate_with_configuration(configuration: KeyGenConfiguration) -> Result<Self, Error> {
        let digest_function: Algorithm = configuration.algorithm;

        let key_gen_option: Option<UrsaKeyGenOption> = configuration
            .key_gen_option
            .map(TryInto::try_into)
            .transpose()?;
        let (mut public_key, mut private_key) = match configuration.algorithm {
            Algorithm::Ed25519 => Ed25519Sha512.keypair(key_gen_option),
            Algorithm::Secp256k1 => EcdsaSecp256k1Sha256::new().keypair(key_gen_option),
            Algorithm::BlsNormal => BlsNormal::new().keypair(key_gen_option),
            Algorithm::BlsSmall => BlsSmall::new().keypair(key_gen_option),
        }?;

        Ok(Self {
            public_key: PublicKey {
                digest_function,
                payload: core::mem::take(&mut public_key.0),
            },
            private_key: PrivateKey {
                digest_function,
                payload: core::mem::take(&mut private_key.0),
            },
        })
    }
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for KeyPair {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        #[derive(Deserialize)]
        struct KeyPairCandidate {
            public_key: PublicKey,
            private_key: PrivateKey,
        }

        // NOTE: Verify that key pair is valid
        let key_pair = KeyPairCandidate::deserialize(deserializer)?;
        Self::new(key_pair.public_key, key_pair.private_key).map_err(D::Error::custom)
    }
}

impl From<KeyPair> for (PublicKey, PrivateKey) {
    fn from(key_pair: KeyPair) -> Self {
        (key_pair.public_key, key_pair.private_key)
    }
}

ffi::ffi_item! {
    /// Public Key used in signatures.
    #[derive(DebugCustom, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, DeserializeFromStr, SerializeDisplay, Decode, Encode, FfiType, IntoSchema)]
    #[debug(fmt = "{{digest: {digest_function}, payload: {payload:X?}}}")]
    pub struct PublicKey {
        /// Digest function
        digest_function: Algorithm,
        /// Key payload
        payload: Vec<u8>,
    }
}

#[cfg_attr(
    all(feature = "ffi_export", not(feature = "ffi_import")),
    iroha_ffi::ffi_export
)]
#[cfg_attr(feature = "ffi_import", iroha_ffi::ffi_import)]
impl PublicKey {
    /// Key payload
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Digest function
    pub fn digest_function(&self) -> Algorithm {
        self.digest_function
    }

    #[cfg(feature = "std")]
    fn try_from_private(private_key: PrivateKey) -> Result<PublicKey, Error> {
        let digest_function = private_key.digest_function();
        let key_gen_option = Some(UrsaKeyGenOption::FromSecretKey(UrsaPrivateKey(
            private_key.payload,
        )));

        let (mut public_key, _) = match digest_function {
            Algorithm::Ed25519 => Ed25519Sha512.keypair(key_gen_option),
            Algorithm::Secp256k1 => EcdsaSecp256k1Sha256::new().keypair(key_gen_option),
            Algorithm::BlsNormal => BlsNormal::new().keypair(key_gen_option),
            Algorithm::BlsSmall => BlsSmall::new().keypair(key_gen_option),
        }?;

        Ok(PublicKey {
            digest_function: private_key.digest_function,
            payload: core::mem::take(&mut public_key.0),
        })
    }
}

impl FromStr for PublicKey {
    type Err = Error;

    // TODO: Can we check the key is valid?
    fn from_str(key: &str) -> Result<Self, Self::Err> {
        let bytes = hex_decode(key).map_err(|err| Error::Parse(err.to_string()))?;

        Multihash::try_from(bytes)
            .map_err(|err| Error::Parse(err.to_string()))
            .map(Into::into)
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let multihash: &Multihash = &self
            .clone()
            .try_into()
            .expect("Failed to get multihash representation.");
        let bytes = Vec::try_from(multihash).expect("Failed to convert multihash to bytes.");

        let mut bytes_iter = bytes.into_iter();
        let fn_code = hex::encode(bytes_iter.by_ref().take(2).collect::<Vec<_>>());
        let dig_size = hex::encode(bytes_iter.by_ref().take(1).collect::<Vec<_>>());
        let key = hex::encode_upper(bytes_iter.by_ref().collect::<Vec<_>>());

        write!(f, "{}{}{}", fn_code, dig_size, key)
    }
}

#[cfg(feature = "std")]
impl From<PrivateKey> for PublicKey {
    fn from(private_key: PrivateKey) -> Self {
        Self::try_from_private(private_key).expect("can't fail for valid `PrivateKey`")
    }
}

ffi::ffi_item! {
    /// Private Key used in signatures.
    #[derive(DebugCustom, Clone, PartialEq, Eq, Serialize, FfiType)]
    #[debug(fmt = "{{digest: {digest_function}, payload: {payload:X?}}}")]
    pub struct PrivateKey {
        /// Digest function
        digest_function: Algorithm,
        /// Key payload
        #[serde(with = "hex::serde")]
        payload: Vec<u8>,
    }
}

impl fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode_upper(&self.payload))
    }
}

#[cfg_attr(
    all(feature = "ffi_export", not(feature = "ffi_import")),
    iroha_ffi::ffi_export
)]
#[cfg_attr(feature = "ffi_import", iroha_ffi::ffi_import)]
impl PrivateKey {
    /// Key payload
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Digest function
    pub fn digest_function(&self) -> Algorithm {
        self.digest_function
    }
}

impl PrivateKey {
    /// Construct `PrivateKey` from hex encoded string without validating the key
    ///
    /// # Errors
    ///
    /// If the given payload is not hex encoded
    pub fn from_hex_unchecked(
        digest_function: Algorithm,
        payload: &(impl AsRef<[u8]> + ?Sized),
    ) -> Result<Self, Error> {
        Ok(Self {
            digest_function,
            payload: hex_decode(payload)?,
        })
    }

    /// Construct `PrivateKey` from hex encoded string
    ///
    /// # Errors
    ///
    /// - If the given payload is not hex encoded
    /// - If the given payload is not a valid private key
    #[cfg(feature = "std")]
    pub fn from_hex(
        digest_function: Algorithm,
        payload: &(impl AsRef<[u8]> + ?Sized),
    ) -> Result<Self, Error> {
        let payload = hex_decode(payload)?;

        let private_key_candidate = Self {
            digest_function,
            payload: payload.clone(),
        };

        PublicKey::try_from_private(private_key_candidate).map(|_| Self {
            digest_function,
            payload,
        })
    }
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for PrivateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        #[derive(Deserialize)]
        struct PrivateKeyCandidate {
            digest_function: Algorithm,
            payload: String,
        }

        // NOTE: Verify that private key is valid
        let private_key = PrivateKeyCandidate::deserialize(deserializer)?;
        Self::from_hex(private_key.digest_function, &private_key.payload).map_err(D::Error::custom)
    }
}

/// Shim for decoding hexadecimal strings that can have spaces
pub(crate) fn hex_decode<T: AsRef<[u8]> + ?Sized>(payload: &T) -> Result<Vec<u8>, Error> {
    let payload: Vec<u8> = payload
        .as_ref()
        .iter()
        .filter(|&e| *e as char != ' ')
        .copied()
        .collect();

    hex::decode(payload).map_err(|err| Error::Parse(err.to_string()))
}

/// Error when dealing with cryptographic functions
#[derive(Debug, Display)]
pub enum Error {
    /// Returned when trying to create an algorithm which does not exist
    #[display(fmt = "Algorithm doesn't exist")] // TODO: which algorithm
    NoSuchAlgorithm,
    /// Occurs during deserialization of a private or public key
    #[display(fmt = "Key could not be parsed. {_0}")]
    Parse(String),
    /// Returned when an error occurs during the signing process
    #[display(fmt = "Signing failed. {_0}")]
    Signing(String),
    /// Returned when an error occurs during key generation
    #[display(fmt = "Key generation failed. {_0}")]
    KeyGen(String),
    /// Returned when an error occurs during digest generation
    #[display(fmt = "Digest generation failed. {_0}")]
    DigestGen(String),
    /// Returned when an error occurs during creation of [`SignaturesOf`]
    #[display(fmt = "`SignaturesOf` must contain at least one signature")]
    EmptySignatureIter,
    /// A General purpose error message that doesn't fit in any category
    #[display(fmt = "General error. {_0}")] // This is going to cause a headache
    Other(String),
}

#[cfg(feature = "std")]
impl From<ursa::CryptoError> for Error {
    fn from(source: ursa::CryptoError) -> Self {
        match source {
            ursa::CryptoError::NoSuchAlgorithm(_) => Self::NoSuchAlgorithm,
            ursa::CryptoError::ParseError(source) => Self::Parse(source),
            ursa::CryptoError::SigningError(source) => Self::Signing(source),
            ursa::CryptoError::KeyGenError(source) => Self::KeyGen(source),
            ursa::CryptoError::DigestGenError(source) => Self::DigestGen(source),
            ursa::CryptoError::GeneralError(source) => Self::Other(source),
        }
    }
}

pub mod ffi {
    //! Definitions and implementations of FFI related functionalities

    use super::*;

    macro_rules! ffi_item {
        ($it: item) => {
            #[cfg(not(feature = "ffi_import"))]
            $it

            #[cfg(feature = "ffi_import")]
            iroha_ffi::ffi! { $it }
        };
    }

    #[cfg(any(feature = "ffi_export", feature = "ffi_import"))]
    macro_rules! ffi_fn {
        ($macro_name: ident) => {
            iroha_ffi::$macro_name! { "iroha_crypto" Clone: KeyPair, PublicKey, PrivateKey }
            iroha_ffi::$macro_name! { "iroha_crypto" Eq: KeyPair, PublicKey, PrivateKey }
            iroha_ffi::$macro_name! { "iroha_crypto" Ord: PublicKey }
            iroha_ffi::$macro_name! { "iroha_crypto" Drop: KeyPair, PublicKey, PrivateKey }
        };
    }

    iroha_ffi::handles! {KeyPair, PublicKey, PrivateKey}

    #[cfg(feature = "ffi_import")]
    ffi_fn! {decl_ffi_fn}
    #[cfg(all(feature = "ffi_export", not(feature = "ffi_import")))]
    ffi_fn! {def_ffi_fn}

    // NOTE: Makes sure that only one `dealloc` is exported per generated dynamic library
    #[cfg(any(crate_type = "dylib", crate_type = "cdylib"))]
    #[cfg(all(feature = "ffi_export", not(feature = "ffi_import")))]
    mod dylib {
        #[cfg(not(feature = "std"))]
        use alloc::alloc;
        #[cfg(feature = "std")]
        use std::alloc;

        iroha_ffi::def_ffi_fn! {dealloc}
    }

    pub(crate) use ffi_item;
}

/// The prelude re-exports most commonly used traits, structs and macros from this crate.
pub mod prelude {
    pub use super::{Algorithm, Hash, KeyPair, PrivateKey, PublicKey, Signature};
}

#[cfg(test)]
mod tests {
    #![allow(clippy::restriction)]

    #[cfg(not(feature = "std"))]
    use alloc::borrow::ToString as _;

    use parity_scale_codec::{Decode, Encode};

    use super::*;

    #[test]
    fn algorithm_serialize_deserialize_consistent() {
        for algorithm in [
            Algorithm::Ed25519,
            Algorithm::Secp256k1,
            Algorithm::BlsNormal,
            Algorithm::BlsSmall,
        ] {
            assert_eq!(
                algorithm,
                serde_json::to_string(&algorithm)
                    .and_then(|algorithm| serde_json::from_str(&algorithm))
                    .unwrap_or_else(|_| panic!("Failed to de/serialize key {:?}", &algorithm))
            )
        }
    }
    #[test]
    fn key_pair_serialize_deserialize_consistent() {
        for algorithm in [
            Algorithm::Ed25519,
            Algorithm::Secp256k1,
            Algorithm::BlsNormal,
            Algorithm::BlsSmall,
        ] {
            let key_pair = KeyPair::generate_with_configuration(
                KeyGenConfiguration::default().with_algorithm(algorithm),
            )
            .expect("Failed to generate key pair");

            assert_eq!(
                key_pair,
                serde_json::to_string(&key_pair)
                    .and_then(|key_pair| serde_json::from_str(&key_pair))
                    .unwrap_or_else(|_| panic!("Failed to de/serialize key {:?}", &key_pair))
            )
        }
    }

    #[test]
    fn encode_decode_algorithm_consistent() {
        for algorithm in [
            Algorithm::Ed25519,
            Algorithm::Secp256k1,
            Algorithm::BlsNormal,
            Algorithm::BlsSmall,
        ] {
            let encoded_algorithm = algorithm.encode();

            let decoded_algorithm =
                Algorithm::decode(&mut encoded_algorithm.as_slice()).expect("Failed to decode");
            assert_eq!(
                algorithm, decoded_algorithm,
                "Failed to decode encoded {:?}",
                &algorithm
            )
        }
    }

    #[test]
    fn key_pair_match() {
        assert!(KeyPair::new("ed012059C8A4DA1EBB5380F74ABA51F502714652FDCCE9611FAFB9904E4A3C4D382774"
            .parse()
            .expect("Public key not in mulithash format"),
        PrivateKey::from_hex(
            Algorithm::Ed25519,
            "93CA389FC2979F3F7D2A7F8B76C70DE6D5EAF5FA58D4F93CB8B0FB298D398ACC59C8A4DA1EBB5380F74ABA51F502714652FDCCE9611FAFB9904E4A3C4D382774"
        ).expect("Private key not hex encoded")).is_ok());

        assert!(KeyPair::new("ea0161040FCFADE2FC5D9104A9ACF9665EA545339DDF10AE50343249E01AF3B8F885CD5D52956542CCE8105DB3A2EC4006E637A7177FAAEA228C311F907DAAFC254F22667F1A1812BB710C6F4116A1415275D27BB9FB884F37E8EF525CC31F3945E945FA"
            .parse()
            .expect("Public key not in mulithash format"),
        PrivateKey::from_hex(
            Algorithm::BlsNormal,
            "0000000000000000000000000000000049BF70187154C57B97AF913163E8E875733B4EAF1F3F0689B31CE392129493E9"
        ).expect("Private key not hex encoded")).is_ok());
    }

    #[test]
    fn encode_decode_public_key_consistent() {
        for algorithm in [
            Algorithm::Ed25519,
            Algorithm::Secp256k1,
            Algorithm::BlsNormal,
            Algorithm::BlsSmall,
        ] {
            let key_pair = KeyPair::generate_with_configuration(
                KeyGenConfiguration::default().with_algorithm(algorithm),
            )
            .expect("Failed to generate key pair");
            let (public_key, _) = key_pair.into();

            let encoded_public_key = public_key.encode();

            let decoded_public_key =
                PublicKey::decode(&mut encoded_public_key.as_slice()).expect("Failed to decode");
            assert_eq!(
                public_key, decoded_public_key,
                "Failed to decode encoded Public Key{:?}",
                &public_key
            )
        }
    }

    #[test]
    fn invalid_private_key() {
        assert!(PrivateKey::from_hex(
            Algorithm::Ed25519,
            "0000000000000000000000000000000049BF70187154C57B97AF913163E8E875733B4EAF1F3F0689B31CE392129493E9"
        ).is_err());

        assert!(
            PrivateKey::from_hex(
                Algorithm::BlsNormal,
                "93CA389FC2979F3F7D2A7F8B76C70DE6D5EAF5FA58D4F93CB8B0FB298D398ACC59C8A4DA1EBB5380F74ABA51F502714652FDCCE9611FAFB9904E4A3C4D382774"
            ).is_err());
    }

    #[test]
    fn key_pair_mismatch() {
        assert!(KeyPair::new("ed012059C8A4DA1EBB5380F74ABA51F502714652FDCCE9611FAFB9904E4A3C4D382774"
            .parse()
            .expect("Public key not in mulithash format"),
        PrivateKey::from_hex(
            Algorithm::Ed25519,
            "3A7991AF1ABB77F3FD27CC148404A6AE4439D095A63591B77C788D53F708A02A1509A611AD6D97B01D871E58ED00C8FD7C3917B6CA61A8C2833A19E000AAC2E4"
        ).expect("Private key not valid")).is_err());

        assert!(KeyPair::new("ea0161040FCFADE2FC5D9104A9ACF9665EA545339DDF10AE50343249E01AF3B8F885CD5D52956542CCE8105DB3A2EC4006E637A7177FAAEA228C311F907DAAFC254F22667F1A1812BB710C6F4116A1415275D27BB9FB884F37E8EF525CC31F3945E945FA"
            .parse()
            .expect("Public key not in mulithash format"),
        PrivateKey::from_hex(
            Algorithm::BlsNormal,
            "000000000000000000000000000000002F57460183837EFBAC6AA6AB3B8DBB7CFFCFC59E9448B7860A206D37D470CBA3"
        ).expect("Private key not valid")).is_err());
    }

    #[test]
    fn display_public_key() {
        assert_eq!(
            format!(
                "{}",
                PublicKey {
                    digest_function: Algorithm::Ed25519,
                    payload: hex_decode(
                        "1509A611AD6D97B01D871E58ED00C8FD7C3917B6CA61A8C2833A19E000AAC2E4"
                    )
                    .expect("Failed to decode public key.")
                }
            ),
            "ed01201509A611AD6D97B01D871E58ED00C8FD7C3917B6CA61A8C2833A19E000AAC2E4"
        );
        assert_eq!(
            format!(
                "{}",
                PublicKey {
                    digest_function: Algorithm::Secp256k1,
                    payload: hex_decode(
                        "0312273E8810581E58948D3FB8F9E8AD53AAA21492EBB8703915BBB565A21B7FCC"
                    )
                    .expect("Failed to decode public key.")
                }
            ),
            "e701210312273E8810581E58948D3FB8F9E8AD53AAA21492EBB8703915BBB565A21B7FCC"
        );
        assert_eq!(
            format!(
                "{}",
                PublicKey {
                    digest_function: Algorithm::BlsNormal,
                    payload: hex_decode(
                        "04175B1E79B15E8A2D5893BF7F8933CA7D0863105D8BAC3D6F976CB043378A0E4B885C57ED14EB85FC2FABC639ADC7DE7F0020C70C57ACC38DEE374AF2C04A6F61C11DE8DF9034B12D849C7EB90099B0881267D0E1507D4365D838D7DCC31511E7"
                    )
                    .expect("Failed to decode public key.")
                }
            ),
            "ea016104175B1E79B15E8A2D5893BF7F8933CA7D0863105D8BAC3D6F976CB043378A0E4B885C57ED14EB85FC2FABC639ADC7DE7F0020C70C57ACC38DEE374AF2C04A6F61C11DE8DF9034B12D849C7EB90099B0881267D0E1507D4365D838D7DCC31511E7"
        );
        assert_eq!(
            format!(
                "{}",
                PublicKey {
                    digest_function: Algorithm::BlsSmall,
                    payload: hex_decode(
                        "040CB3231F601E7245A6EC9A647B450936F707CA7DC347ED258586C1924941D8BC38576473A8BA3BB2C37E3E121130AB67103498A96D0D27003E3AD960493DA79209CF024E2AA2AE961300976AEEE599A31A5E1B683EAA1BCFFC47B09757D20F21123C594CF0EE0BAF5E1BDD272346B7DC98A8F12C481A6B28174076A352DA8EAE881B90911013369D7FA960716A5ABC5314307463FA2285A5BF2A5B5C6220D68C2D34101A91DBFC531C5B9BBFB2245CCC0C50051F79FC6714D16907B1FC40E0C0"
                    )
                    .expect("Failed to decode public key.")
                }
            ),
            "eb01c1040CB3231F601E7245A6EC9A647B450936F707CA7DC347ED258586C1924941D8BC38576473A8BA3BB2C37E3E121130AB67103498A96D0D27003E3AD960493DA79209CF024E2AA2AE961300976AEEE599A31A5E1B683EAA1BCFFC47B09757D20F21123C594CF0EE0BAF5E1BDD272346B7DC98A8F12C481A6B28174076A352DA8EAE881B90911013369D7FA960716A5ABC5314307463FA2285A5BF2A5B5C6220D68C2D34101A91DBFC531C5B9BBFB2245CCC0C50051F79FC6714D16907B1FC40E0C0"
        )
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestJson {
        public_key: PublicKey,
        private_key: PrivateKey,
    }

    #[test]
    fn deserialize_keys_ed25519() {
        assert_eq!(
            serde_json::from_str::<'_, TestJson>("{
                \"public_key\": \"ed01201509A611AD6D97B01D871E58ED00C8FD7C3917B6CA61A8C2833A19E000AAC2E4\",
                \"private_key\": {
                    \"digest_function\": \"ed25519\",
                    \"payload\": \"3A7991AF1ABB77F3FD27CC148404A6AE4439D095A63591B77C788D53F708A02A1509A611AD6D97B01D871E58ED00C8FD7C3917B6CA61A8C2833A19E000AAC2E4\"
                }
            }").expect("Failed to deserialize."),
            TestJson {
                public_key: PublicKey {
                    digest_function: Algorithm::Ed25519,
                    payload: hex_decode(
                        "1509A611AD6D97B01D871E58ED00C8FD7C3917B6CA61A8C2833A19E000AAC2E4"
                    )
                    .expect("Failed to decode public key.")
                },
                private_key: PrivateKey {
                    digest_function: Algorithm::Ed25519,
                    payload: hex_decode("3A7991AF1ABB77F3FD27CC148404A6AE4439D095A63591B77C788D53F708A02A1509A611AD6D97B01D871E58ED00C8FD7C3917B6CA61A8C2833A19E000AAC2E4")
                    .expect("Failed to decode private key"),
                }
            }
        );
    }

    #[test]
    fn deserialize_keys_secp256k1() {
        assert_eq!(
            serde_json::from_str::<'_, TestJson>("{
                \"public_key\": \"e701210312273E8810581E58948D3FB8F9E8AD53AAA21492EBB8703915BBB565A21B7FCC\",
                \"private_key\": {
                    \"digest_function\": \"secp256k1\",
                    \"payload\": \"4DF4FCA10762D4B529FE40A2188A60CA4469D2C50A825B5F33ADC2CB78C69445\"
                }
            }").expect("Failed to deserialize."),
            TestJson {
                public_key: PublicKey {
                    digest_function: Algorithm::Secp256k1,
                    payload: hex_decode(
                        "0312273E8810581E58948D3FB8F9E8AD53AAA21492EBB8703915BBB565A21B7FCC"
                    )
                    .expect("Failed to decode public key.")
                },
                private_key: PrivateKey {
                    digest_function: Algorithm::Secp256k1,
                    payload: hex_decode("4DF4FCA10762D4B529FE40A2188A60CA4469D2C50A825B5F33ADC2CB78C69445")
                    .expect("Failed to decode private key"),
                }
            }
        );
    }

    #[test]
    fn deserialize_keys_bls() {
        assert_eq!(
            serde_json::from_str::<'_, TestJson>("{
                \"public_key\": \"ea016104175B1E79B15E8A2D5893BF7F8933CA7D0863105D8BAC3D6F976CB043378A0E4B885C57ED14EB85FC2FABC639ADC7DE7F0020C70C57ACC38DEE374AF2C04A6F61C11DE8DF9034B12D849C7EB90099B0881267D0E1507D4365D838D7DCC31511E7\",
                \"private_key\": {
                    \"digest_function\": \"bls_normal\",
                    \"payload\": \"000000000000000000000000000000002F57460183837EFBAC6AA6AB3B8DBB7CFFCFC59E9448B7860A206D37D470CBA3\"
                }
            }").expect("Failed to deserialize."),
            TestJson {
                public_key: PublicKey {
                    digest_function: Algorithm::BlsNormal,
                    payload: hex_decode(
                        "04175B1E79B15E8A2D5893BF7F8933CA7D0863105D8BAC3D6F976CB043378A0E4B885C57ED14EB85FC2FABC639ADC7DE7F0020C70C57ACC38DEE374AF2C04A6F61C11DE8DF9034B12D849C7EB90099B0881267D0E1507D4365D838D7DCC31511E7"
                    )
                    .expect("Failed to decode public key.")
                },
                private_key: PrivateKey {
                    digest_function: Algorithm::BlsNormal,
                    payload: hex_decode("000000000000000000000000000000002F57460183837EFBAC6AA6AB3B8DBB7CFFCFC59E9448B7860A206D37D470CBA3")
                    .expect("Failed to decode private key"),
                }
            }
        );
        assert_eq!(
            serde_json::from_str::<'_, TestJson>("{
                \"public_key\": \"eb01C1040CB3231F601E7245A6EC9A647B450936F707CA7DC347ED258586C1924941D8BC38576473A8BA3BB2C37E3E121130AB67103498A96D0D27003E3AD960493DA79209CF024E2AA2AE961300976AEEE599A31A5E1B683EAA1BCFFC47B09757D20F21123C594CF0EE0BAF5E1BDD272346B7DC98A8F12C481A6B28174076A352DA8EAE881B90911013369D7FA960716A5ABC5314307463FA2285A5BF2A5B5C6220D68C2D34101A91DBFC531C5B9BBFB2245CCC0C50051F79FC6714D16907B1FC40E0C0\",
                \"private_key\": {
                    \"digest_function\": \"bls_small\",
                    \"payload\": \"0000000000000000000000000000000060F3C1AC9ADDBBED8DB83BC1B2EF22139FB049EECB723A557A41CA1A4B1FED63\"
                }
            }").expect("Failed to deserialize."),
            TestJson {
                public_key: PublicKey {
                    digest_function: Algorithm::BlsSmall,
                    payload: hex_decode(
                        "040CB3231F601E7245A6EC9A647B450936F707CA7DC347ED258586C1924941D8BC38576473A8BA3BB2C37E3E121130AB67103498A96D0D27003E3AD960493DA79209CF024E2AA2AE961300976AEEE599A31A5E1B683EAA1BCFFC47B09757D20F21123C594CF0EE0BAF5E1BDD272346B7DC98A8F12C481A6B28174076A352DA8EAE881B90911013369D7FA960716A5ABC5314307463FA2285A5BF2A5B5C6220D68C2D34101A91DBFC531C5B9BBFB2245CCC0C50051F79FC6714D16907B1FC40E0C0"
                    )
                    .expect("Failed to decode public key.")
                },
                private_key: PrivateKey {
                    digest_function: Algorithm::BlsSmall,
                    payload: hex_decode("0000000000000000000000000000000060F3C1AC9ADDBBED8DB83BC1B2EF22139FB049EECB723A557A41CA1A4B1FED63")
                    .expect("Failed to decode private key"),
                }
            }
        )
    }
}
