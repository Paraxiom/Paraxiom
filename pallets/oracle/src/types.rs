use core::default;

use crate::Config;
use frame_support::pallet_prelude::{Decode, Encode, MaxEncodedLen, TypeInfo};
use frame_support::{
    traits::{ConstU128, ConstU32, ConstU64},
    BoundedVec
};
use sp_core::H256;


pub type RequestId = H256;

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Request<T: Config> { 
    pub requester: T::AccountId,
    pub nonce: u64,
}