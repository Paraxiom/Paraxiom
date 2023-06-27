use crate::Config;
use frame_support::pallet_prelude::{ConstU32, Decode, Encode, MaxEncodedLen, TypeInfo};
use frame_support::BoundedVec;

use phat_offchain_rollup::types::ValueBytes;
use sp_core::H256;
use sp_runtime::AccountId32;

use pallet_registry::types::RegistryFeedKey;

pub type RequestId = H256;
pub type Bytes = BoundedVec<u8, ConstU32<64>>;
pub type ResponseData = ValueBytes;
pub type RequestData = ValueBytes;

/// A Request for feed data
#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Request<T: Config> {
    pub registry_feed_key: RegistryFeedKey<T>,
    pub caller: T::AccountId,
    pub nonce: u128,
    pub requested_data: RequestData,
}

/// A quote from a price feed oracle
#[derive(Debug, PartialEq, Eq, Encode, Decode, Clone, scale_info::TypeInfo, MaxEncodedLen)]
pub struct PriceQuote {
    pub contract_id: H256,
    pub price: u128,
    pub timestamp_ms: u64,
}

/// The reponse from the oracle Phat Contract (copied from Phat Contract)
#[derive(Debug, PartialEq, Eq, Encode, Decode, Clone, scale_info::TypeInfo)]
pub struct ResponseRecord<T: Config> {
    pub owner: T::AccountId,
    pub phat_contract_id: H256,
    pub request_id: RequestId,
    pub response_data: ResponseData,
    pub timestamp_ms: u64,
}
