// Copyright 2019-2022 Manta Network.
// This file is part of manta-rs.
//
// manta-rs is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// manta-rs is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with manta-rs.  If not, see <http://www.gnu.org/licenses/>.

//! Secret Key Generation
//!
//! This module contains [`KeySecret`] which implements a hierarchical deterministic key generation
//! scheme based on the [`BIP-0044`] specification. We may implement other kinds of key generation
//! schemes in the future.
//!
//! See [`CoinType`] for the coins which this key generation scheme can control.
//!
//! [`BIP-0044`]: https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki

use crate::{config::utxo::v2 as protocol_pay, crypto::constraint::arkworks::Fp};
use alloc::{format, string::String};
use core::marker::PhantomData;
use manta_accounting::{
    key::{self, AccountIndex, DeriveAddress},
    transfer::{self, utxo::v2 as protocol},
};
use manta_crypto::{
    algebra::{HasGenerator, ScalarMul},
    arkworks::{
        ed_on_bn254::FrParameters,
        ff::{Fp256, PrimeField},
    },
    rand::{CryptoRng, OsRng, Rand, RngCore, Sample},
};
use manta_util::{create_seal, seal, Array};

#[cfg(feature = "serde")]
use manta_util::serde::{Deserialize, Serialize, Serializer};

pub use bip32::{Error, Seed, XPrv as SecretKey};

create_seal! {}

/// Coin Type Id Type
pub type CoinTypeId = u128;

/// Coin Type Marker Trait
///
/// This trait identifies a coin type and its identifier for the [`BIP-0044`] specification. This
/// trait is sealed and can only be used with the existing implementations.
///
/// [`BIP-0044`]: https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki
pub trait CoinType: sealed::Sealed {
    /// The coin type id for this coin.
    ///
    /// See [`SLIP-0044`] for a list of registered coin type ids.
    ///
    /// [`SLIP-0044`]: https://github.com/satoshilabs/slips/blob/master/slip-0044.md
    const COIN_TYPE_ID: CoinTypeId;
}

/// Implements the [`CoinType`] trait for `$coin` with coin type id given by `$id`.
macro_rules! impl_coin_type {
    (
        $coin:ident,
        $doc:expr,
        $name:expr,
        $id:expr,
        $coin_type_id:ident,
        $key_secret:ident,
        $account_map:ident
    ) => {
        #[doc = $doc]
        #[doc = "Network"]
        #[doc = $name]
        #[doc = "Coin Type"]
        pub struct $coin;

        #[doc = stringify!($coin)]
        #[doc = "Coin Type Id"]
        pub const $coin_type_id: CoinTypeId = $id;

        #[doc = stringify!($coin)]
        #[doc = "[`KeySecret`] Type"]
        pub type $key_secret = KeySecret<$coin>;

        #[doc = stringify!($coin)]
        #[doc = "[`VecAccountMap`] Type"]
        pub type $account_map = VecAccountMap<$coin>;

        seal!($coin);

        impl CoinType for $coin {
            const COIN_TYPE_ID: CoinTypeId = $coin_type_id;
        }
    };
}

impl_coin_type!(
    Testnet,
    "Test",
    "`testnet`",
    1,
    TESTNET_COIN_TYPE_ID,
    TestnetKeySecret,
    TestnetAccountMap
);

impl_coin_type!(
    Manta,
    "Main",
    "`manta`",
    611,
    MANTA_COIN_TYPE_ID,
    MantaKeySecret,
    MantaAccountMap
);

impl_coin_type!(
    Calamari,
    "Canary",
    "`calamari`",
    612,
    CALAMARI_COIN_TYPE_ID,
    CalamariKeySecret,
    CalamariAccountMap
);

/// Seed Byte Array Type
type SeedBytes = Array<u8, { Seed::SIZE }>;

/// Vector Account Map Type
pub type VecAccountMap<C = Manta> = key::VecAccountMap<Account<C>>;

/// Key Secret
#[cfg_attr(
    feature = "serde",
    derive(Deserialize, Serialize),
    serde(crate = "manta_util::serde", deny_unknown_fields, transparent)
)]
#[derive(derivative::Derivative)]
#[derivative(Clone(bound = ""), Default(bound = ""))]
pub struct KeySecret<C>
where
    C: CoinType,
{
    /// Key Seed
    seed: SeedBytes,

    /// Type Parameter Marker
    __: PhantomData<C>,
}

impl<C> KeySecret<C>
where
    C: CoinType,
{
    /// Builds a [`KeySecret`] from raw bytes.
    #[inline]
    fn build(seed: [u8; Seed::SIZE]) -> Self {
        Self {
            seed: seed.into(),
            __: PhantomData,
        }
    }

    /// Builds a [`KeySecret`] from a `seed`.
    #[inline]
    fn from_seed(seed: Seed) -> Self {
        Self::build(*seed.as_bytes())
    }

    /// Converts a `mnemonic` phrase into a [`KeySecret`], locking it with `password`.
    #[inline]
    #[must_use]
    pub fn new(mnemonic: Mnemonic, password: &str) -> Self {
        Self::from_seed(mnemonic.to_seed(password))
    }
}

/// Account
pub struct Account<C = Manta>
where
    C: CoinType,
{
    key_secret: KeySecret<C>,
    index: AccountIndex,
}

impl<C> Account<C>
where
    C: CoinType,
{
    /// Creates new
    pub fn new(key_secret: KeySecret<C>, index: AccountIndex) -> Self {
        Self { key_secret, index }
    }
}

impl<C> key::Account for Account<C>
where
    C: CoinType,
{
    type SpendingKey = transfer::SpendingKey<crate::config::Config>;
    type Parameters = protocol::Parameters<protocol_pay::Config>; // todo: double-check this

    #[inline]
    fn spending_key(&self, parameters: &Self::Parameters) -> Self::SpendingKey {
        let _ = parameters;
        let xpr_secret_key = SecretKey::derive_from_path(
            self.key_secret.seed,
            &path_string::<C>(self.index)
                .parse()
                .expect("Path string is valid by construction."),
        )
        .expect("Unable to generate secret key for valid seed and path string.");
        Fp(Fp256::<FrParameters>::from_le_bytes_mod_order(
            &xpr_secret_key.to_bytes(),
        ))
    }
}

impl<C> key::CreateFromIndex for Account<C>
where
    C: CoinType,
{
    type Index = AccountIndex;

    #[inline]
    fn create_from_index(index: &Self::Index) -> Self {
        let mut rng = OsRng;
        let key_secret = rng.gen();
        Self {
            key_secret,
            index: *index,
        }
    }
}

impl<C> Sample for KeySecret<C>
where
    C: CoinType,
{
    #[inline]
    fn sample<R>(_: (), rng: &mut R) -> Self
    where
        R: RngCore + ?Sized,
    {
        let mut seed = [0; Seed::SIZE];
        rng.fill_bytes(&mut seed);
        Self::build(seed)
    }
}
use protocol::ViewingKeyDerivationFunction;

impl<C> DeriveAddress for Account<C>
where
    C: CoinType,
{
    type Address = protocol::Address<protocol_pay::Config>;
    type Parameters = protocol::Parameters<protocol_pay::Config>;
    #[inline]
    fn address(&self, parameters: &Self::Parameters) -> Self::Address {
        let generator = parameters.base.group_generator.generator();
        let spending_key = &key::Account::spending_key(&self, parameters);
        protocol::Address::new(
            generator.scalar_mul(
                &parameters
                    .base
                    .viewing_key_derivation_function
                    .viewing_key(&generator.scalar_mul(spending_key, &mut ()), &mut ()),
                &mut (),
            ),
        )
    }
}

/// Computes the [`BIP-0044`] path string for the given coin settings.
///
/// [`BIP-0044`]: https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki
#[inline]
#[must_use]
pub fn path_string<C>(account: AccountIndex) -> String
where
    C: CoinType,
{
    const BIP_44_PURPOSE_ID: u8 = 44;
    format!(
        "m/{}'/{}'/{}'",
        BIP_44_PURPOSE_ID,
        C::COIN_TYPE_ID,
        account.index()
    )
}

/// Mnemonic
#[cfg_attr(
    feature = "serde",
    derive(Deserialize, Serialize),
    serde(crate = "manta_util::serde", deny_unknown_fields, try_from = "String")
)]
#[derive(Clone)]
pub struct Mnemonic(
    /// Underlying BIP39 Mnemonic
    #[cfg_attr(feature = "serde", serde(serialize_with = "Mnemonic::serialize"))]
    bip32::Mnemonic,
);

impl Mnemonic {
    /// Create a new BIP39 mnemonic phrase from the given string.
    #[inline]
    pub fn new<S>(phrase: S) -> Result<Self, Error>
    where
        S: AsRef<str>,
    {
        bip32::Mnemonic::new(phrase, Default::default()).map(Self)
    }

    /// Samples a random [`Mnemonic`] using the entropy returned from `rng`.
    #[inline]
    pub fn sample<R>(rng: &mut R) -> Self
    where
        R: CryptoRng + RngCore + ?Sized,
    {
        Self(bip32::Mnemonic::random(rng, Default::default()))
    }

    /// Convert this mnemonic phrase into the BIP39 seed value.
    #[inline]
    pub fn to_seed(&self, password: &str) -> Seed {
        self.0.to_seed(password)
    }

    /// Serializes the underlying `mnemonic` phrase.
    #[cfg(feature = "serde")]
    #[inline]
    fn serialize<S>(mnemonic: &bip32::Mnemonic, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        mnemonic.phrase().serialize(serializer)
    }
}

impl AsRef<str> for Mnemonic {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.phrase()
    }
}

impl Eq for Mnemonic {}

impl PartialEq for Mnemonic {
    #[inline]
    fn eq(&self, rhs: &Self) -> bool {
        self.as_ref().eq(rhs.as_ref())
    }
}

impl TryFrom<String> for Mnemonic {
    type Error = Error;

    #[inline]
    fn try_from(string: String) -> Result<Self, Self::Error> {
        Self::new(string)
    }
}
