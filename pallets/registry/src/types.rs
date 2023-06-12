use frame_support::pallet_prelude::{Decode, Encode, MaxEncodedLen, TypeInfo};
use frame_support::{
    traits::{ConstU128, ConstU32, ConstU64},
    BoundedVec,
};
use crate::Config;

/// Types representing limited strings
pub type RegistryFeedKey<T> = BoundedVec<u8, <T as Config>::MaxKeySize>;
pub type RegistryFeedUrl<T> = BoundedVec<u8, <T as Config>::MaxUrlSize>;
pub type RegistryFeedPath<T> = BoundedVec<u8, <T as Config>::MaxPathSize>;

/// Feed status
#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum ApiFeedStatus {
    Registered,
    Active,
    Inactive,
    // occurs if a feed has too many errors
    Slashed,
}

impl Default for ApiFeedStatus {
    fn default() -> Self {
        ApiFeedStatus::Registered
    }
}