#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
use frame_support::pallet_prelude::{Decode, Encode, MaxEncodedLen, TypeInfo};
use frame_support::{
    traits::{ConstU128, ConstU32, ConstU64},
    BoundedVec,
};
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;
use sp_std::{borrow::ToOwned, convert::TryFrom, convert::TryInto, prelude::*, str, vec, vec::Vec};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod types;

#[frame_support::pallet]
pub mod pallet {
    use crate::types::*;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::{OptionQuery, ValueQuery, *},
        traits::Bounded,
    };
    use frame_system::pallet_prelude::*;

    #[derive(Encode, Decode, Default, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct ApiFeed<BlockNumber, RegistryString, RegistryPath> {
        started_at: BlockNumber,
        url: RegistryString,
        path: RegistryPath,
        status: ApiFeedStatus,
    }

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Constants
        #[pallet::constant]
        type MaxUrlSize: Get<u32>;
        #[pallet::constant]
        type MaxKeySize: Get<u32>;
        #[pallet::constant]
        type MaxPathSize: Get<u32>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// A 2D storage map of all feeds which the registry keeps track of.
    ///
    /// The main mapping is [Account ID] -> [Feed Key] -> [ApiFeed / Feed URL].
    #[pallet::storage]
    #[pallet::getter(fn api_feeds)]
    pub type ApiFeeds<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        T::AccountId,
        Twox64Concat,
        RegistryFeedKey<T>,
        ApiFeed<T::BlockNumber, RegistryFeedUrl<T>, RegistryFeedPath<T>>,
    >;

    // Events
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New feed registered
        FeedRegistered {
            caller: T::AccountId,
            key: RegistryFeedKey<T>,
            feed: ApiFeed<T::BlockNumber, RegistryFeedUrl<T>, RegistryFeedPath<T>>,
        },
        /// Existing feed unregistered
        FeedUnregistered {
            caller: T::AccountId,
            key: RegistryFeedKey<T>,
            feed: ApiFeed<T::BlockNumber, RegistryFeedUrl<T>, RegistryFeedPath<T>>,
        },
    }

    // Errors
    #[pallet::error]
    pub enum Error<T> {
        /// Error names should be descriptive.
        NoneValue,
        /// Errors should have helpful documentation associated with them.
        StorageOverflow,
    }

    // Hooks
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Non-callable (internal) Methods
    impl<T: Config> Pallet<T> {
        /// Checks if a feed exists and is active.
        pub fn is_active(account_id: T::AccountId, key: RegistryFeedKey<T>) -> bool {
            if let Some(feed) = <ApiFeeds<T>>::get(&account_id, &key) {
                return feed.status == ApiFeedStatus::Active;
            }

            false
        }
    }

    // Extrinsics
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Adds an API feed (in the form of a URL) to the storage to keep track of it and request
        /// data from it.
        ///
        /// This origin must be root until further mechanisms for adding feeds is introduced such as
        /// Parachains adding feeds and putting up a staked amount as an economic incentive to avoid
        /// providing bad data.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
        pub fn register_feed(
            origin: OriginFor<T>,
            key: RegistryFeedKey<T>,
            url: RegistryFeedUrl<T>,
            path: RegistryFeedPath<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let block_number = <frame_system::Pallet<T>>::block_number();
            let feed = ApiFeed {
                started_at: block_number,
                url,
                path,
                status: ApiFeedStatus::Registered,
            };

            // Insert the feed into storage.
            <ApiFeeds<T>>::insert(&who, &key, feed.clone());
            // Emit an event.
            Self::deposit_event(Event::FeedRegistered {
                caller: who,
                key,
                feed,
            });

            Ok(())
        }

        /// Removes a feed from storage.
        ///
        /// The origin must be the same as the creator which first registered the feed. The only
        /// other scenario which would cause an feed to be removed is having bad data (getting slashed)
        /// or other uptime metrics (e.g. the feed errors out too many times).
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
        pub fn unregister_feed(origin: OriginFor<T>, key: RegistryFeedKey<T>) -> DispatchResult {
            let who = ensure_signed(origin.clone())?;

            // Remove the feed from storage if it exists.
            // This also implicitly checks that the feed is owned by the origin account.
            if let Some(feed) = <ApiFeeds<T>>::get(&who, &key) {
                <ApiFeeds<T>>::remove(&who, &key);
                Self::deposit_event(Event::FeedUnregistered {
                    caller: who,
                    key,
                    feed,
                });
                Ok(())
            } else {
                // otherwise, return an error
                Err(DispatchError::CannotLookup)
            }
        }
    }
}
