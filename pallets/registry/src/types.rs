use core::default;

use crate::Config;
use frame_support::pallet_prelude::{Decode, Encode, MaxEncodedLen, TypeInfo};
use frame_support::{
    traits::{ConstU128, ConstU32, ConstU64},
    BoundedVec,
};

/// Types representing limited strings
pub type RegistryFeedKey<T> = BoundedVec<u8, <T as Config>::MaxKeySize>;
pub type RegistryFeedUrl<T> = BoundedVec<u8, <T as Config>::MaxUrlSize>;
pub type RegistryFeedPath<T> = BoundedVec<u8, <T as Config>::MaxPathSize>;
// pub type RegistryFeedName<T> = BoundedVec<u8, <T as Config>::MaxNameSize>;

/// Feed status
#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum ApiFeedStatus {
    #[default]
    Registered,
    Active,
    Inactive,
    // occurs if a feed has too many errors
    Slashed,
}

impl ApiFeedStatus {
    pub fn is_active(&self) -> bool {
        return *self == ApiFeedStatus::Active
    }
}
