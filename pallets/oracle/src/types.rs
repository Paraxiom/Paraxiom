use crate::Config;
use frame_support::pallet_prelude::{Decode, Encode, MaxEncodedLen, TypeInfo};

use sp_core::H256;

use pallet_registry::types::RegistryFeedKey;

pub type RequestId = H256;

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Request<T: Config> {
    pub registry_feed_key: RegistryFeedKey<T>,
    pub caller: T::AccountId,
    pub nonce: u128,
}
