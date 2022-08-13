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

//! Manta-Pay UTXO Model Version 1 Configuration

use crate::{
    config::{
        poseidon::{
            Spec2 as Poseidon2, Spec3 as Poseidon3, Spec4 as Poseidon4, Spec5 as Poseidon5,
        },
        Compiler, ConstraintField, EmbeddedScalar, EmbeddedScalarField, EmbeddedScalarVar, Group,
        GroupCurve, GroupVar,
    },
    crypto::{
        constraint::arkworks::{rem_mod_prime, Boolean, Fp, FpVar, PrimeModulus},
        ecc::arkworks::ScalarVar,
        poseidon::{self, encryption::BlockArray, hash::Hasher, ParameterFieldType},
    },
};
use blake2::{
    digest::{Update, VariableOutput},
    Blake2s256, Blake2sVar, Digest,
};
use core::marker::PhantomData;
use manta_accounting::asset::Asset;
use manta_crypto::{
    algebra::HasGenerator,
    arkworks::{
        algebra::affine_point_as_bytes,
        ff::{try_into_u128, PrimeField},
        serialize::CanonicalSerialize,
    },
    constraint::measure::print_measurement,
    eclair::{
        alloc::{Allocate, Constant},
        num::U128,
        ops::Rem,
    },
    encryption, hash,
    hash::ArrayHashFunction,
    merkle_tree,
    rand::{Rand, RngCore, Sample},
    signature::schnorr,
};

pub use manta_accounting::transfer::{
    self,
    utxo::{self, v1 as protocol},
};

///
pub type AssetId = Fp<ConstraintField>;

///
pub type AssetIdVar = FpVar<ConstraintField>;

///
pub type AssetValue = u128;

///
pub type AssetValueVar = U128<FpVar<ConstraintField>>;

///
pub type AuthorizationContext = utxo::auth::AuthorizationContext<Parameters>;

///
pub type AuthorizationContextVar = utxo::auth::AuthorizationContext<ParametersVar>;

///
pub type AuthorizationProof = utxo::auth::AuthorizationProof<Parameters>;

///
pub type AuthorizationProofVar = utxo::auth::AuthorizationProof<ParametersVar>;

///
pub type ProofAuthorizationKey = Group;

///
pub type ProofAuthorizationKeyVar = GroupVar;

///
pub type ViewingKey = EmbeddedScalar;

///
pub type ViewingKeyVar = EmbeddedScalarVar;

///
pub type ReceivingKey = Group;

///
pub type ReceivingKeyVar = GroupVar;

///
pub type UtxoAccumulatorItem = Fp<ConstraintField>;

///
pub type UtxoAccumulatorItemVar = FpVar<ConstraintField>;

///
pub type UtxoAccumulatorWitness = utxo::UtxoAccumulatorWitness<Parameters>;

///
pub type UtxoAccumulatorWitnessVar = utxo::UtxoAccumulatorWitness<ParametersVar, Compiler>;

///
pub type UtxoAccumulatorOutput = utxo::UtxoAccumulatorOutput<Parameters>;

///
pub type UtxoAccumulatorOutputVar = utxo::UtxoAccumulatorOutput<ParametersVar, Compiler>;

///
pub type Parameters = protocol::Parameters<Config>;

///
pub type ParametersVar = protocol::BaseParameters<Config<Compiler>, Compiler>;

///
pub type AssociatedData = utxo::AssociatedData<Parameters>;

///
pub type Utxo = utxo::Utxo<Parameters>;

///
pub type UtxoVar = utxo::Utxo<ParametersVar>;

///
pub type Note = utxo::Note<Parameters>;

///
pub type NoteVar = utxo::Note<ParametersVar>;

///
pub type Nullifier = utxo::Nullifier<Parameters>;

///
pub type NullifierVar = utxo::Nullifier<ParametersVar>;

///
pub type Identifier = utxo::Identifier<Parameters>;

///
pub type Address = utxo::Address<Parameters>;

///
pub type MintSecret = protocol::MintSecret<Config>;

///
pub type MintSecretVar = protocol::MintSecret<Config<Compiler>, Compiler>;

///
pub type SpendSecret = protocol::SpendSecret<Config>;

///
pub type SpendSecretVar = protocol::SpendSecret<Config<Compiler>, Compiler>;

///
pub struct GroupGenerator(Group);

impl HasGenerator<Group> for GroupGenerator {
    #[inline]
    fn generator(&self) -> &Group {
        &self.0
    }
}

impl Sample for GroupGenerator {
    #[inline]
    fn sample<R>(_: (), rng: &mut R) -> Self
    where
        R: RngCore + ?Sized,
    {
        Self(rng.gen())
    }
}

///
pub struct GroupGeneratorVar(GroupVar);

impl HasGenerator<GroupVar, Compiler> for GroupGeneratorVar {
    #[inline]
    fn generator(&self) -> &GroupVar {
        &self.0
    }
}

impl Constant<Compiler> for GroupGeneratorVar {
    type Type = GroupGenerator;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut Compiler) -> Self {
        Self(this.0.as_constant(compiler))
    }
}

///
pub struct UtxoCommitmentSchemeDomainTag;

impl poseidon::hash::DomainTag<Poseidon5> for UtxoCommitmentSchemeDomainTag {
    #[inline]
    fn domain_tag() -> <Poseidon5 as ParameterFieldType>::ParameterField {
        Fp(0u8.into()) // FIXME: Use a real domain tag
    }
}

impl<COM> Constant<COM> for UtxoCommitmentSchemeDomainTag {
    type Type = Self;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut COM) -> Self {
        let _ = (this, compiler);
        Self
    }
}

///
pub struct UtxoCommitmentScheme<COM = ()>(Hasher<Poseidon5, UtxoCommitmentSchemeDomainTag, 5, COM>)
where
    Poseidon5: poseidon::Specification<COM>;

impl Sample for UtxoCommitmentScheme {
    #[inline]
    fn sample<R>(distribution: (), rng: &mut R) -> Self
    where
        R: RngCore + ?Sized,
    {
        Self(rng.sample(distribution))
    }
}

impl Constant<Compiler> for UtxoCommitmentScheme<Compiler> {
    type Type = UtxoCommitmentScheme;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut Compiler) -> Self {
        Self(this.0.as_constant(compiler))
    }
}

impl protocol::UtxoCommitmentScheme for UtxoCommitmentScheme {
    type AssetId = AssetId;
    type AssetValue = AssetValue;
    type ReceivingKey = ReceivingKey;
    type Randomness = Fp<ConstraintField>;
    type Commitment = Fp<ConstraintField>;

    #[inline]
    fn commit(
        &self,
        randomness: &Self::Randomness,
        asset_id: &Self::AssetId,
        asset_value: &Self::AssetValue,
        receiving_key: &Self::ReceivingKey,
        compiler: &mut (),
    ) -> Self::Commitment {
        self.0.hash(
            [
                randomness,
                asset_id,
                &Fp((*asset_value).into()),
                &Fp(receiving_key.0.x),
                &Fp(receiving_key.0.y),
            ],
            compiler,
        )
    }
}

impl protocol::UtxoCommitmentScheme<Compiler> for UtxoCommitmentScheme<Compiler> {
    type AssetId = AssetIdVar;
    type AssetValue = AssetValueVar;
    type ReceivingKey = ReceivingKeyVar;
    type Randomness = FpVar<ConstraintField>;
    type Commitment = FpVar<ConstraintField>;

    #[inline]
    fn commit(
        &self,
        randomness: &Self::Randomness,
        asset_id: &Self::AssetId,
        asset_value: &Self::AssetValue,
        receiving_key: &Self::ReceivingKey,
        compiler: &mut Compiler,
    ) -> Self::Commitment {
        print_measurement(
            "UTXO COMMITMENT SCHEME",
            |compiler| {
                self.0.hash(
                    [
                        randomness,
                        asset_id,
                        asset_value.as_ref(),
                        &receiving_key.0.x,
                        &receiving_key.0.y,
                    ],
                    compiler,
                )
            },
            compiler,
        )
    }
}

///
#[derive(Default)]
pub struct ViewingKeyDerivationFunctionDomainTag;

impl poseidon::hash::DomainTag<Poseidon2> for ViewingKeyDerivationFunctionDomainTag {
    #[inline]
    fn domain_tag() -> <Poseidon2 as ParameterFieldType>::ParameterField {
        Fp(0u8.into()) // FIXME: Use a real domain tag
    }
}

impl<COM> Constant<COM> for ViewingKeyDerivationFunctionDomainTag {
    type Type = Self;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut COM) -> Self {
        let _ = (this, compiler);
        Self
    }
}

///
pub struct ViewingKeyDerivationFunction<COM = ()>(
    Hasher<Poseidon2, ViewingKeyDerivationFunctionDomainTag, 2, COM>,
)
where
    Poseidon2: poseidon::Specification<COM>;

impl Sample for ViewingKeyDerivationFunction {
    #[inline]
    fn sample<R>(distribution: (), rng: &mut R) -> Self
    where
        R: RngCore + ?Sized,
    {
        Self(rng.sample(distribution))
    }
}

impl Constant<Compiler> for ViewingKeyDerivationFunction<Compiler> {
    type Type = ViewingKeyDerivationFunction;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut Compiler) -> Self {
        Self(this.0.as_constant(compiler))
    }
}

impl protocol::ViewingKeyDerivationFunction for ViewingKeyDerivationFunction {
    type ProofAuthorizationKey = ProofAuthorizationKey;
    type ViewingKey = ViewingKey;

    #[inline]
    fn viewing_key(
        &self,
        proof_authorization_key: &Self::ProofAuthorizationKey,
        compiler: &mut (),
    ) -> Self::ViewingKey {
        Fp(rem_mod_prime::<ConstraintField, EmbeddedScalarField>(
            self.0
                .hash(
                    [
                        &Fp(proof_authorization_key.0.x),
                        &Fp(proof_authorization_key.0.y),
                    ],
                    compiler,
                )
                .0,
        ))
    }
}

impl protocol::ViewingKeyDerivationFunction<Compiler> for ViewingKeyDerivationFunction<Compiler> {
    type ProofAuthorizationKey = ProofAuthorizationKeyVar;
    type ViewingKey = ViewingKeyVar;

    #[inline]
    fn viewing_key(
        &self,
        proof_authorization_key: &Self::ProofAuthorizationKey,
        compiler: &mut Compiler,
    ) -> Self::ViewingKey {
        print_measurement(
            "VIEWING KEY DERIVATION FUNCTION",
            |compiler| {
                ScalarVar::new(
                    self.0
                        .hash(
                            [&proof_authorization_key.0.x, &proof_authorization_key.0.y],
                            compiler,
                        )
                        .rem(PrimeModulus::<EmbeddedScalarField>::default(), compiler),
                )
            },
            compiler,
        )
    }
}

///
#[derive(derivative::Derivative)]
#[derivative(Default)]
pub struct IncomingEncryptionSchemeConverter<COM = ()>(PhantomData<COM>);

impl encryption::HeaderType for IncomingEncryptionSchemeConverter {
    type Header = encryption::EmptyHeader;
}

impl encryption::HeaderType for IncomingEncryptionSchemeConverter<Compiler> {
    type Header = encryption::EmptyHeader<Compiler>;
}

impl encryption::convert::header::Header for IncomingEncryptionSchemeConverter {
    type TargetHeader = encryption::Header<IncomingPoseidonEncryptionScheme>;

    #[inline]
    fn as_target(source: &Self::Header, _: &mut ()) -> Self::TargetHeader {
        let _ = source;
        vec![]
    }
}

impl encryption::convert::header::Header<Compiler> for IncomingEncryptionSchemeConverter<Compiler> {
    type TargetHeader = encryption::Header<IncomingPoseidonEncryptionScheme<Compiler>>;

    #[inline]
    fn as_target(source: &Self::Header, _: &mut Compiler) -> Self::TargetHeader {
        let _ = source;
        vec![]
    }
}

impl encryption::EncryptionKeyType for IncomingEncryptionSchemeConverter {
    type EncryptionKey = Group;
}

impl encryption::EncryptionKeyType for IncomingEncryptionSchemeConverter<Compiler> {
    type EncryptionKey = GroupVar;
}

impl encryption::convert::key::Encryption for IncomingEncryptionSchemeConverter {
    type TargetEncryptionKey = encryption::EncryptionKey<IncomingPoseidonEncryptionScheme>;

    #[inline]
    fn as_target(source: &Self::EncryptionKey, _: &mut ()) -> Self::TargetEncryptionKey {
        vec![Fp(source.0.x), Fp(source.0.y)]
    }
}

impl encryption::convert::key::Encryption<Compiler>
    for IncomingEncryptionSchemeConverter<Compiler>
{
    type TargetEncryptionKey =
        encryption::EncryptionKey<IncomingPoseidonEncryptionScheme<Compiler>>;

    #[inline]
    fn as_target(source: &Self::EncryptionKey, _: &mut Compiler) -> Self::TargetEncryptionKey {
        vec![source.0.x.clone(), source.0.y.clone()]
    }
}

impl encryption::DecryptionKeyType for IncomingEncryptionSchemeConverter {
    type DecryptionKey = Group;
}

impl encryption::DecryptionKeyType for IncomingEncryptionSchemeConverter<Compiler> {
    type DecryptionKey = GroupVar;
}

impl encryption::convert::key::Decryption for IncomingEncryptionSchemeConverter {
    type TargetDecryptionKey = encryption::DecryptionKey<IncomingPoseidonEncryptionScheme>;

    #[inline]
    fn as_target(source: &Self::DecryptionKey, _: &mut ()) -> Self::TargetDecryptionKey {
        vec![Fp(source.0.x), Fp(source.0.y)]
    }
}

impl encryption::convert::key::Decryption<Compiler>
    for IncomingEncryptionSchemeConverter<Compiler>
{
    type TargetDecryptionKey =
        encryption::DecryptionKey<IncomingPoseidonEncryptionScheme<Compiler>>;

    #[inline]
    fn as_target(source: &Self::DecryptionKey, _: &mut Compiler) -> Self::TargetDecryptionKey {
        vec![source.0.x.clone(), source.0.y.clone()]
    }
}

impl encryption::PlaintextType for IncomingEncryptionSchemeConverter {
    type Plaintext = protocol::IncomingPlaintext<Config>;
}

impl encryption::PlaintextType for IncomingEncryptionSchemeConverter<Compiler> {
    type Plaintext = protocol::IncomingPlaintext<Config<Compiler>, Compiler>;
}

impl encryption::convert::plaintext::Forward for IncomingEncryptionSchemeConverter {
    type TargetPlaintext = encryption::Plaintext<IncomingPoseidonEncryptionScheme>;

    #[inline]
    fn as_target(source: &Self::Plaintext, _: &mut ()) -> Self::TargetPlaintext {
        BlockArray(
            [poseidon::encryption::PlaintextBlock(
                vec![
                    source.utxo_commitment_randomness,
                    source.asset.id,
                    Fp(source.asset.value.into()),
                ]
                .into(),
            )]
            .into(),
        )
    }
}

impl encryption::convert::plaintext::Forward<Compiler>
    for IncomingEncryptionSchemeConverter<Compiler>
{
    type TargetPlaintext = encryption::Plaintext<IncomingPoseidonEncryptionScheme<Compiler>>;

    #[inline]
    fn as_target(source: &Self::Plaintext, _: &mut Compiler) -> Self::TargetPlaintext {
        BlockArray(
            [poseidon::encryption::PlaintextBlock(
                vec![
                    source.utxo_commitment_randomness.clone(),
                    source.asset.id.clone(),
                    source.asset.value.as_ref().clone(),
                ]
                .into(),
            )]
            .into(),
        )
    }
}

impl encryption::DecryptedPlaintextType for IncomingEncryptionSchemeConverter {
    type DecryptedPlaintext = Option<<Self as encryption::PlaintextType>::Plaintext>;
}

impl encryption::convert::plaintext::Reverse for IncomingEncryptionSchemeConverter {
    type TargetDecryptedPlaintext =
        encryption::DecryptedPlaintext<IncomingPoseidonEncryptionScheme>;

    #[inline]
    fn into_source(target: Self::TargetDecryptedPlaintext, _: &mut ()) -> Self::DecryptedPlaintext {
        if target.0 && target.1.len() == 1 {
            let block = &target.1[0].0;
            if block.len() == 3 {
                Some(protocol::IncomingPlaintext::new(
                    Fp(block[0].0),
                    Asset::new(Fp(block[1].0), try_into_u128(block[2].0)?),
                ))
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<COM> Constant<COM> for IncomingEncryptionSchemeConverter<COM> {
    type Type = IncomingEncryptionSchemeConverter;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut COM) -> Self {
        let _ = (this, compiler);
        Self::default()
    }
}

///
pub type IncomingPoseidonEncryptionScheme<COM = ()> =
    poseidon::encryption::FixedDuplexer<1, Poseidon3, COM>;

///
pub type IncomingBaseEncryptionScheme<COM = ()> = encryption::convert::key::Converter<
    encryption::convert::header::Converter<
        encryption::convert::plaintext::Converter<
            IncomingPoseidonEncryptionScheme<COM>,
            IncomingEncryptionSchemeConverter<COM>,
        >,
        IncomingEncryptionSchemeConverter<COM>,
    >,
    IncomingEncryptionSchemeConverter<COM>,
>;

///
pub struct UtxoAccumulatorItemHashDomainTag;

impl poseidon::hash::DomainTag<Poseidon4> for UtxoAccumulatorItemHashDomainTag {
    #[inline]
    fn domain_tag() -> <Poseidon4 as ParameterFieldType>::ParameterField {
        Fp(0u8.into()) // FIXME: Use a real domain tag
    }
}

impl<COM> Constant<COM> for UtxoAccumulatorItemHashDomainTag {
    type Type = Self;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut COM) -> Self {
        let _ = (this, compiler);
        Self
    }
}

///
pub struct UtxoAccumulatorItemHash<COM = ()>(
    Hasher<Poseidon4, UtxoAccumulatorItemHashDomainTag, 4, COM>,
)
where
    Poseidon4: poseidon::Specification<COM>;

impl Sample for UtxoAccumulatorItemHash {
    #[inline]
    fn sample<R>(distribution: (), rng: &mut R) -> Self
    where
        R: RngCore + ?Sized,
    {
        Self(rng.sample(distribution))
    }
}

impl Constant<Compiler> for UtxoAccumulatorItemHash<Compiler> {
    type Type = UtxoAccumulatorItemHash;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut Compiler) -> Self {
        Self(this.0.as_constant(compiler))
    }
}

impl protocol::UtxoAccumulatorItemHash for UtxoAccumulatorItemHash {
    type Bool = bool;
    type AssetId = AssetId;
    type AssetValue = AssetValue;
    type Commitment = Fp<ConstraintField>;
    type Item = UtxoAccumulatorItem;

    #[inline]
    fn hash(
        &self,
        is_transparent: &Self::Bool,
        public_asset_id: &Self::AssetId,
        public_asset_value: &Self::AssetValue,
        commitment: &Self::Commitment,
        compiler: &mut (),
    ) -> Self::Item {
        self.0.hash(
            [
                &Fp((*is_transparent).into()),
                public_asset_id,
                &Fp((*public_asset_value).into()),
                commitment,
            ],
            compiler,
        )
    }
}

impl protocol::UtxoAccumulatorItemHash<Compiler> for UtxoAccumulatorItemHash<Compiler> {
    type Bool = Boolean<ConstraintField>;
    type AssetId = AssetIdVar;
    type AssetValue = AssetValueVar;
    type Commitment = FpVar<ConstraintField>;
    type Item = UtxoAccumulatorItemVar;

    #[inline]
    fn hash(
        &self,
        is_transparent: &Self::Bool,
        public_asset_id: &Self::AssetId,
        public_asset_value: &Self::AssetValue,
        commitment: &Self::Commitment,
        compiler: &mut Compiler,
    ) -> Self::Item {
        print_measurement(
            "UTXO ACCUMULATOR ITEM HASH",
            |compiler| {
                self.0.hash(
                    [
                        &(is_transparent.clone()).into(),
                        public_asset_id,
                        public_asset_value.as_ref(),
                        commitment,
                    ],
                    compiler,
                )
            },
            compiler,
        )
    }
}

/// Leaf Hash Configuration Type
pub type LeafHash = merkle_tree::IdentityLeafHash<UtxoAccumulatorItem>;

/// Leaf Hash Variable Configuration Type
pub type LeafHashVar = merkle_tree::IdentityLeafHash<UtxoAccumulatorItemVar, Compiler>;

///
pub struct InnerHashDomainTag;

impl poseidon::hash::DomainTag<Poseidon2> for InnerHashDomainTag {
    #[inline]
    fn domain_tag() -> <Poseidon2 as ParameterFieldType>::ParameterField {
        Fp(0u8.into()) // FIXME: Use a real domain tag
    }
}

impl<COM> Constant<COM> for InnerHashDomainTag {
    type Type = Self;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut COM) -> Self {
        let _ = (this, compiler);
        Self
    }
}

/// Inner Hash Configuration
pub struct InnerHash<COM = ()>(PhantomData<COM>);

impl merkle_tree::InnerHash for InnerHash {
    type LeafDigest = UtxoAccumulatorItem;
    type Parameters = Hasher<Poseidon2, InnerHashDomainTag, 2>;
    type Output = Fp<ConstraintField>;

    #[inline]
    fn join(
        parameters: &Self::Parameters,
        lhs: &Self::Output,
        rhs: &Self::Output,
        compiler: &mut (),
    ) -> Self::Output {
        parameters.hash([lhs, rhs], compiler)
    }

    #[inline]
    fn join_leaves(
        parameters: &Self::Parameters,
        lhs: &Self::LeafDigest,
        rhs: &Self::LeafDigest,
        compiler: &mut (),
    ) -> Self::Output {
        parameters.hash([lhs, rhs], compiler)
    }
}

impl merkle_tree::InnerHash<Compiler> for InnerHash<Compiler> {
    type LeafDigest = UtxoAccumulatorItemVar;
    type Parameters = Hasher<Poseidon2, InnerHashDomainTag, 2, Compiler>;
    type Output = FpVar<ConstraintField>;

    #[inline]
    fn join(
        parameters: &Self::Parameters,
        lhs: &Self::Output,
        rhs: &Self::Output,
        compiler: &mut Compiler,
    ) -> Self::Output {
        print_measurement(
            "INNER HASH",
            |compiler| parameters.hash([lhs, rhs], compiler),
            compiler,
        )
    }

    #[inline]
    fn join_leaves(
        parameters: &Self::Parameters,
        lhs: &Self::LeafDigest,
        rhs: &Self::LeafDigest,
        compiler: &mut Compiler,
    ) -> Self::Output {
        print_measurement(
            "INNER HASH",
            |compiler| parameters.hash([lhs, rhs], compiler),
            compiler,
        )
    }
}

/// Merkle Tree Configuration
pub struct MerkleTreeConfiguration;

impl MerkleTreeConfiguration {
    /// Width of the Merkle Forest
    pub const FOREST_WIDTH: usize = 256;
}

impl merkle_tree::HashConfiguration for MerkleTreeConfiguration {
    type LeafHash = LeafHash;
    type InnerHash = InnerHash;
}

impl merkle_tree::HashConfiguration<Compiler> for MerkleTreeConfiguration {
    type LeafHash = LeafHashVar;
    type InnerHash = InnerHash<Compiler>;
}

impl merkle_tree::Configuration for MerkleTreeConfiguration {
    const HEIGHT: usize = 20;
}

impl merkle_tree::Configuration<Compiler> for MerkleTreeConfiguration {
    const HEIGHT: usize = 20;
}

impl<COM> Constant<COM> for MerkleTreeConfiguration {
    type Type = Self;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut COM) -> Self {
        let _ = (this, compiler);
        Self
    }
}

/// UTXO Accumulator Model
pub type UtxoAccumulatorModel = merkle_tree::Parameters<MerkleTreeConfiguration>;

/// UTXO Accumulator Model Variable
pub type UtxoAccumulatorModelVar = merkle_tree::Parameters<MerkleTreeConfiguration, Compiler>;

impl merkle_tree::forest::Configuration for MerkleTreeConfiguration {
    type Index = u8;

    #[inline]
    fn tree_index(leaf: &merkle_tree::Leaf<Self>) -> Self::Index {
        let mut hasher = Blake2sVar::new(1).unwrap();
        let mut buffer = Vec::new();
        leaf.0
            .serialize_unchecked(&mut buffer)
            .expect("Serializing is not allowed to fail.");
        hasher.update(&buffer);
        let mut result = [0];
        hasher
            .finalize_variable(&mut result)
            .expect("Hashing is not allowed to fail.");
        result[0]
    }
}

#[cfg(any(feature = "test", test))]
impl merkle_tree::test::HashParameterSampling for MerkleTreeConfiguration {
    type LeafHashParameterDistribution = ();
    type InnerHashParameterDistribution = ();

    #[inline]
    fn sample_leaf_hash_parameters<R>(
        distribution: Self::LeafHashParameterDistribution,
        rng: &mut R,
    ) -> merkle_tree::LeafHashParameters<Self>
    where
        R: RngCore + ?Sized,
    {
        let _ = (distribution, rng);
    }

    #[inline]
    fn sample_inner_hash_parameters<R>(
        distribution: Self::InnerHashParameterDistribution,
        rng: &mut R,
    ) -> merkle_tree::InnerHashParameters<Self>
    where
        R: RngCore + ?Sized,
    {
        rng.sample(distribution)
    }
}

///
pub struct NullifierCommitmentSchemeDomainTag;

impl poseidon::hash::DomainTag<Poseidon3> for NullifierCommitmentSchemeDomainTag {
    #[inline]
    fn domain_tag() -> <Poseidon3 as ParameterFieldType>::ParameterField {
        Fp(0u8.into()) // FIXME: Use a real domain tag
    }
}

impl<COM> Constant<COM> for NullifierCommitmentSchemeDomainTag {
    type Type = Self;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut COM) -> Self {
        let _ = (this, compiler);
        Self
    }
}

///
pub struct NullifierCommitmentScheme<COM = ()>(
    Hasher<Poseidon3, NullifierCommitmentSchemeDomainTag, 3, COM>,
)
where
    Poseidon3: poseidon::Specification<COM>;

impl Sample for NullifierCommitmentScheme {
    #[inline]
    fn sample<R>(distribution: (), rng: &mut R) -> Self
    where
        R: RngCore + ?Sized,
    {
        Self(rng.sample(distribution))
    }
}

impl Constant<Compiler> for NullifierCommitmentScheme<Compiler> {
    type Type = NullifierCommitmentScheme;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut Compiler) -> Self {
        Self(this.0.as_constant(compiler))
    }
}

impl protocol::NullifierCommitmentScheme for NullifierCommitmentScheme {
    type ProofAuthorizationKey = ProofAuthorizationKey;
    type UtxoAccumulatorItem = Fp<ConstraintField>;
    type Commitment = Fp<ConstraintField>;

    #[inline]
    fn commit(
        &self,
        proof_authorization_key: &Self::ProofAuthorizationKey,
        item: &Self::UtxoAccumulatorItem,
        compiler: &mut (),
    ) -> Self::Commitment {
        self.0.hash(
            [
                &Fp(proof_authorization_key.0.x),
                &Fp(proof_authorization_key.0.y),
                item,
            ],
            compiler,
        )
    }
}

impl protocol::NullifierCommitmentScheme<Compiler> for NullifierCommitmentScheme<Compiler> {
    type ProofAuthorizationKey = ProofAuthorizationKeyVar;
    type UtxoAccumulatorItem = FpVar<ConstraintField>;
    type Commitment = FpVar<ConstraintField>;

    #[inline]
    fn commit(
        &self,
        proof_authorization_key: &Self::ProofAuthorizationKey,
        item: &Self::UtxoAccumulatorItem,
        compiler: &mut Compiler,
    ) -> Self::Commitment {
        print_measurement(
            "NULLIFIER COMMITMENT SCHEME",
            |compiler| {
                self.0.hash(
                    [
                        &proof_authorization_key.0.x,
                        &proof_authorization_key.0.y,
                        item,
                    ],
                    compiler,
                )
            },
            compiler,
        )
    }
}

///
#[derive(derivative::Derivative)]
#[derivative(Default)]
pub struct OutgoingEncryptionSchemeConverter<COM = ()>(PhantomData<COM>);

impl encryption::HeaderType for OutgoingEncryptionSchemeConverter {
    type Header = encryption::EmptyHeader;
}

impl encryption::HeaderType for OutgoingEncryptionSchemeConverter<Compiler> {
    type Header = encryption::EmptyHeader<Compiler>;
}

impl encryption::convert::header::Header for OutgoingEncryptionSchemeConverter {
    type TargetHeader = encryption::Header<OutgoingPoseidonEncryptionScheme>;

    #[inline]
    fn as_target(source: &Self::Header, _: &mut ()) -> Self::TargetHeader {
        let _ = source;
        vec![]
    }
}

impl encryption::convert::header::Header<Compiler> for OutgoingEncryptionSchemeConverter<Compiler> {
    type TargetHeader = encryption::Header<OutgoingPoseidonEncryptionScheme<Compiler>>;

    #[inline]
    fn as_target(source: &Self::Header, _: &mut Compiler) -> Self::TargetHeader {
        let _ = source;
        vec![]
    }
}

impl encryption::EncryptionKeyType for OutgoingEncryptionSchemeConverter {
    type EncryptionKey = Group;
}

impl encryption::EncryptionKeyType for OutgoingEncryptionSchemeConverter<Compiler> {
    type EncryptionKey = GroupVar;
}

impl encryption::convert::key::Encryption for OutgoingEncryptionSchemeConverter {
    type TargetEncryptionKey = encryption::EncryptionKey<OutgoingPoseidonEncryptionScheme>;

    #[inline]
    fn as_target(source: &Self::EncryptionKey, _: &mut ()) -> Self::TargetEncryptionKey {
        vec![Fp(source.0.x), Fp(source.0.y)]
    }
}

impl encryption::convert::key::Encryption<Compiler>
    for OutgoingEncryptionSchemeConverter<Compiler>
{
    type TargetEncryptionKey =
        encryption::EncryptionKey<OutgoingPoseidonEncryptionScheme<Compiler>>;

    #[inline]
    fn as_target(source: &Self::EncryptionKey, _: &mut Compiler) -> Self::TargetEncryptionKey {
        vec![source.0.x.clone(), source.0.y.clone()]
    }
}

impl encryption::DecryptionKeyType for OutgoingEncryptionSchemeConverter {
    type DecryptionKey = Group;
}

impl encryption::DecryptionKeyType for OutgoingEncryptionSchemeConverter<Compiler> {
    type DecryptionKey = GroupVar;
}

impl encryption::convert::key::Decryption for OutgoingEncryptionSchemeConverter {
    type TargetDecryptionKey = encryption::DecryptionKey<OutgoingPoseidonEncryptionScheme>;

    #[inline]
    fn as_target(source: &Self::DecryptionKey, _: &mut ()) -> Self::TargetDecryptionKey {
        vec![Fp(source.0.x), Fp(source.0.y)]
    }
}

impl encryption::convert::key::Decryption<Compiler>
    for OutgoingEncryptionSchemeConverter<Compiler>
{
    type TargetDecryptionKey =
        encryption::DecryptionKey<OutgoingPoseidonEncryptionScheme<Compiler>>;

    #[inline]
    fn as_target(source: &Self::DecryptionKey, _: &mut Compiler) -> Self::TargetDecryptionKey {
        vec![source.0.x.clone(), source.0.y.clone()]
    }
}

impl encryption::PlaintextType for OutgoingEncryptionSchemeConverter {
    type Plaintext = Asset<AssetId, AssetValue>;
}

impl encryption::PlaintextType for OutgoingEncryptionSchemeConverter<Compiler> {
    type Plaintext = Asset<AssetIdVar, AssetValueVar>;
}

impl encryption::convert::plaintext::Forward for OutgoingEncryptionSchemeConverter {
    type TargetPlaintext = encryption::Plaintext<OutgoingPoseidonEncryptionScheme>;

    #[inline]
    fn as_target(source: &Self::Plaintext, _: &mut ()) -> Self::TargetPlaintext {
        BlockArray(
            [poseidon::encryption::PlaintextBlock(
                vec![source.id, Fp(source.value.into())].into(),
            )]
            .into(),
        )
    }
}

impl encryption::convert::plaintext::Forward<Compiler>
    for OutgoingEncryptionSchemeConverter<Compiler>
{
    type TargetPlaintext = encryption::Plaintext<OutgoingPoseidonEncryptionScheme<Compiler>>;

    #[inline]
    fn as_target(source: &Self::Plaintext, _: &mut Compiler) -> Self::TargetPlaintext {
        BlockArray(
            [poseidon::encryption::PlaintextBlock(
                vec![source.id.clone(), source.value.as_ref().clone()].into(),
            )]
            .into(),
        )
    }
}

impl encryption::DecryptedPlaintextType for OutgoingEncryptionSchemeConverter {
    type DecryptedPlaintext = Option<<Self as encryption::PlaintextType>::Plaintext>;
}

impl encryption::convert::plaintext::Reverse for OutgoingEncryptionSchemeConverter {
    type TargetDecryptedPlaintext =
        encryption::DecryptedPlaintext<OutgoingPoseidonEncryptionScheme>;

    #[inline]
    fn into_source(target: Self::TargetDecryptedPlaintext, _: &mut ()) -> Self::DecryptedPlaintext {
        if target.0 && target.1.len() == 1 {
            let block = &target.1[0].0;
            if block.len() == 2 {
                Some(Asset::new(block[0], try_into_u128(block[1].0)?))
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<COM> Constant<COM> for OutgoingEncryptionSchemeConverter<COM> {
    type Type = OutgoingEncryptionSchemeConverter;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut COM) -> Self {
        let _ = (this, compiler);
        Self::default()
    }
}

///
pub type OutgoingPoseidonEncryptionScheme<COM = ()> =
    poseidon::encryption::FixedDuplexer<1, Poseidon2, COM>;

///
pub type OutgoingBaseEncryptionScheme<COM = ()> = encryption::convert::key::Converter<
    encryption::convert::header::Converter<
        encryption::convert::plaintext::Converter<
            OutgoingPoseidonEncryptionScheme<COM>,
            OutgoingEncryptionSchemeConverter<COM>,
        >,
        OutgoingEncryptionSchemeConverter<COM>,
    >,
    OutgoingEncryptionSchemeConverter<COM>,
>;

///
#[derive(Clone)]
pub struct SchnorrHashFunction;

impl hash::security::PreimageResistance for SchnorrHashFunction {}

impl schnorr::HashFunction<Group> for SchnorrHashFunction {
    type Message = Vec<u8>;

    #[inline]
    fn hash(
        &self,
        verifying_key: &Group,
        nonce_point: &Group,
        message: &Self::Message,
        _: &mut (),
    ) -> EmbeddedScalar {
        let mut hasher = Blake2s256::new();
        Digest::update(&mut hasher, b"domain tag"); // FIXME: Use specific domain tag
        Digest::update(
            &mut hasher,
            affine_point_as_bytes::<GroupCurve>(&verifying_key.0),
        );
        Digest::update(
            &mut hasher,
            affine_point_as_bytes::<GroupCurve>(&nonce_point.0),
        );
        Digest::update(&mut hasher, message);
        let bytes: [u8; 32] = hasher.finalize().into();
        Fp(EmbeddedScalarField::from_le_bytes_mod_order(&bytes))
    }
}

impl Sample for SchnorrHashFunction {
    #[inline]
    fn sample<R>(distribution: (), rng: &mut R) -> Self
    where
        R: RngCore + ?Sized,
    {
        let _ = (distribution, rng);
        Self
    }
}

///
#[derive(derivative::Derivative)]
#[derivative(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Config<COM = ()>(PhantomData<COM>);

impl<COM> Constant<COM> for Config<COM> {
    type Type = Config;

    #[inline]
    fn new_constant(this: &Self::Type, compiler: &mut COM) -> Self {
        let _ = (this, compiler);
        Self::default()
    }
}

impl protocol::BaseConfiguration for Config {
    type Bool = bool;
    type AssetId = AssetId;
    type AssetValue = AssetValue;
    type Scalar = EmbeddedScalar;
    type Group = Group;
    type GroupGenerator = GroupGenerator;
    type UtxoCommitmentScheme = UtxoCommitmentScheme;
    type ViewingKeyDerivationFunction = ViewingKeyDerivationFunction;
    type IncomingCiphertext =
        <IncomingBaseEncryptionScheme as encryption::CiphertextType>::Ciphertext;
    type IncomingBaseEncryptionScheme = IncomingBaseEncryptionScheme;
    type UtxoAccumulatorItemHash = UtxoAccumulatorItemHash;
    type UtxoAccumulatorModel = UtxoAccumulatorModel;
    type NullifierCommitmentScheme = NullifierCommitmentScheme;
    type OutgoingCiphertext =
        <OutgoingBaseEncryptionScheme as encryption::CiphertextType>::Ciphertext;
    type OutgoingBaseEncryptionScheme = OutgoingBaseEncryptionScheme;
}

impl protocol::BaseConfiguration<Compiler> for Config<Compiler> {
    type Bool = Boolean<ConstraintField>;
    type AssetId = AssetIdVar;
    type AssetValue = AssetValueVar;
    type Scalar = EmbeddedScalarVar;
    type Group = GroupVar;
    type GroupGenerator = GroupGeneratorVar;
    type UtxoCommitmentScheme = UtxoCommitmentScheme<Compiler>;
    type ViewingKeyDerivationFunction = ViewingKeyDerivationFunction<Compiler>;
    type IncomingCiphertext =
        <IncomingBaseEncryptionScheme<Compiler> as encryption::CiphertextType>::Ciphertext;
    type IncomingBaseEncryptionScheme = IncomingBaseEncryptionScheme<Compiler>;
    type UtxoAccumulatorItemHash = UtxoAccumulatorItemHash<Compiler>;
    type UtxoAccumulatorModel = UtxoAccumulatorModelVar;
    type NullifierCommitmentScheme = NullifierCommitmentScheme<Compiler>;
    type OutgoingCiphertext =
        <OutgoingBaseEncryptionScheme<Compiler> as encryption::CiphertextType>::Ciphertext;
    type OutgoingBaseEncryptionScheme = OutgoingBaseEncryptionScheme<Compiler>;
}

impl protocol::Configuration for Config {
    type SchnorrHashFunction = SchnorrHashFunction;
}
