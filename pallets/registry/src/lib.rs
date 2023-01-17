#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;
use frame_support::{
	traits::{ConstU32, ConstU64, ConstU128,},
    BoundedVec, 
};
use frame_support::pallet_prelude::{
		Encode, Decode, TypeInfo, MaxEncodedLen,
	};
use sp_std::{
    borrow::ToOwned, convert::TryFrom, convert::TryInto, 
    prelude::*, str, vec, vec::Vec
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;



#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::{*, OptionQuery, ValueQuery}, traits::Bounded};
	use frame_system::pallet_prelude::*;

	pub type RegStrT = BoundedVec<u8, ConstU32<512>>;

	#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct ApiFeed<BlockNumber> {
		requested_block_number: BlockNumber,
		url: Option<RegStrT>,
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Storage map for the feeds
    #[pallet::storage]
	#[pallet::getter(fn api_feeds)]
	pub type ApiFeeds<T: Config> =
		StorageDoubleMap<
		_, Twox64Concat, T::AccountId, 
		Twox64Concat, RegStrT, 
		ApiFeed<T::BlockNumber>
		>;

    /// Storage map for the feed URL Endpoint
    #[pallet::storage]
	#[pallet::getter(fn reporter)]
	// pub type ReporterConfig<T> = StorageValue<_, Twox64Concat, OptionQuery, AccountId, BoundedVec<u8, <T as Config>::StrLimit>>;
	pub(super) type Reporter<T: Config> = StorageMap
    <
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<BoundedVec<u8,  ConstU32<100> >,  ConstU32<100>> ,
        ValueQuery 
    >;
    

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New feed is submitted.
		RegFeed {
			sender: T::AccountId,
            key: RegStrT,
            feed: ApiFeed<T::BlockNumber>,
		},
        /// Apifeed is removed.
		UnRegFeed {
			sender: T::AccountId,
            key: RegStrT,
            feed: ApiFeed<T::BlockNumber>,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn reg_feed(
            origin: OriginFor<T>,
            key: RegStrT,
            url: RegStrT,
        ) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let block_number = <frame_system::Pallet<T>>::block_number();
			let feed = ApiFeed {
					requested_block_number: block_number,
					url: Some(url),
				};
			ApiFeeds::<T>::insert(&who, &key, feed.clone());

			// Emit an event.
			Self::deposit_event(Event::RegFeed { sender: who, key, feed });
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn unreg_feed(
            origin: OriginFor<T>,
            key: RegStrT,
        ) -> DispatchResult {
            let who = ensure_signed(origin.clone())?;

			let feed_exists = ApiFeeds::<T>::contains_key(&who, &key);
			if feed_exists {
				let feed = Self::api_feeds(&who, &key).unwrap();
				<ApiFeeds<T>>::remove(&who, &key);
				Self::deposit_event(Event::UnRegFeed { sender: who, key, feed });
				Ok(())
			} else {
				Err(DispatchError::CannotLookup)
			}

        }

	}
}
