//! # Sample Oracle based on Offchain Rollup
#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion, transactional, traits::Randomness,
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::{AccountId32, traits::Hash};
    use sp_std::vec::Vec;
    use sp_runtime::traits::Hash;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type QuotesCount: Get<u32>;
        type MyRandomness: Randomness<Self::Hash, Self::BlockNumber>;
        // type Hashing: Hash<Output = Self::Hash>;
    }

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    pub type Bytes = BoundedVec<u8, ConstU32<64>>;

    /// Mapping from (deployer, trading_pair) to price feed quotes
    #[pallet::storage]
    #[pallet::getter(fn price_feeds)]
    pub type PriceFeeds<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        AccountId32,
        Blake2_128Concat,
        Bytes,
        // price quotes
        BoundedVec<PriceQuote, T::QuotesCount>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn averages)]
    pub type Averages<T: Config> = StorageMap<_, Twox64Concat, Bytes, u128>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // Price quote received
        QuoteReceived {
            contract: H256,
            submitter: T::AccountId,
            owner: AccountId32,
            pair: Bytes,
            price: u128,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        FailedToAuthenticateResponse,
        FailedToDecodeResponse,
        FailedToPushResponse,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sends a request to the oracle
        #[pallet::weight(0)]
        #[pallet::call_index(0)]
        #[transactional]
        pub fn request(
            origin: OriginFor<T>,
            _name: H256,
            data: Bytes,
            _nonce: u128,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            // we need seed to generate randomness
            let seed = T::Hashing::hash_of(&data);
            let randomness_seed = T::MyRandomness::random_seed();
            let (random_value, _) = T::MyRandomness::random(&randomness_seed);
            Ok(())
        }
        /// Sends a request to the oracle
        #[pallet::weight(0)]
        #[pallet::call_index(1)]
        #[transactional]
        pub fn feeds(
            origin: OriginFor<T>,
            _name: H256,
            _data: Bytes,
            _nonce: u128,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            Ok(())
        }
        /// Sends a request to the oracle
        #[pallet::weight(0)]
        #[pallet::call_index(2)]
        pub fn average(origin: OriginFor<T>) -> DispatchResult {
            ensure_signed(origin)?;
            let storage_map = PriceFeeds::<T>::iter().collect::<Vec<_>>();
            let mut averages: Vec<(Bytes, u128)> = Vec::new();

            storage_map.iter().for_each(|(_i, _j, k)| {
                let sum: u128 = k.iter().map(|j| j.price).sum::<u128>();
                let count = k.len() as u128;
                let average = sum / count;
                averages.push((_j.clone(), average));
            });

            for (pair, average) in averages {
                Averages::<T>::insert(&pair, average);
            }
            Ok(())
        }
    }

    impl<T: Config> phat_offchain_rollup::anchor::OnResponse<T::AccountId> for Pallet<T> {
        fn on_response(name: H256, submitter: T::AccountId, data: Vec<u8>) -> DispatchResult {
            let resp: ResponseRecord =
                Decode::decode(&mut &data[..]).or(Err(Error::<T>::FailedToDecodeResponse))?;
            if resp.contract_id != name {
                return Err(Error::<T>::FailedToAuthenticateResponse.into());
            }

            let storage_map = PriceFeeds::<T>::drain().collect::<Vec<_>>();
            let mut pricequotes = BoundedVec::<PriceQuote, T::QuotesCount>::default();

            for (_i, j, mut k) in storage_map {
                if j == resp.pair {
                    if k.len() == T::QuotesCount::get() as usize {
                        k.drain(..T::QuotesCount::get() as usize);
                    }
                    k.try_push({
                        PriceQuote {
                            contract_id: resp.contract_id,
                            price: resp.price,
                            timestamp_ms: resp.timestamp_ms,
                        }
                    })
                    .or(Err(Error::<T>::FailedToPushResponse))?;
                    pricequotes = k;
                }
            }
            PriceFeeds::<T>::insert(&resp.owner, resp.pair.clone(), pricequotes);

            Self::deposit_event(Event::QuoteReceived {
                contract: name,
                submitter,
                owner: resp.owner,
                pair: resp.pair,
                price: resp.price,
            });
            Ok(())
        }
    }

    // impl<T: Config> crate::anchor::OnResponse<T::AccountId> for Pallet<T> {
    //     fn on_response(name: H256, submitter: T::AccountId, data: Vec<u8>) -> DispatchResult {
    //         let resp: ResponseRecord =
    //             Decode::decode(&mut &data[..]).or(Err(Error::<T>::FailedToDecodeResponse))?;
    //         if resp.contract_id != name {
    //             return Err(Error::<T>::FailedToAuthenticateResponse.into());
    //         }

    //         let mut pricequotes = BoundedVec::<PriceQuote, ConstU32<6>>::default();
    //         let mut storage_map = PriceFeeds::<T>::drain().collect::<Vec<_>>();

    //         for (_i, j, mut k) in storage_map.clone() {
    //             if j == resp.pair {
    //                 k.try_push({
    //                     PriceQuote {
    //                         contract_id: resp.contract_id,
    //                         price: resp.price,
    //                         timestamp_ms: resp.timestamp_ms,
    //                     }
    //                 });
    //                 pricequotes = k.clone();

    //             }

    //         }
    //         PriceFeeds::<T>::insert(&resp.owner, &resp.pair.clone(), &pricequotes);

    //         Self::deposit_event(Event::QuoteReceived {
    //             contract: name,
    //             submitter,
    //             owner: resp.owner,
    //             pair: resp.pair,
    //             price: resp.price,
    //         });
    //         Ok(())
    //     }
    // }

    /// A quote from a price feed oracle
    #[derive(Debug, PartialEq, Eq, Encode, Decode, Clone, scale_info::TypeInfo, MaxEncodedLen)]
    pub struct PriceQuote {
        contract_id: H256,
        price: u128,
        timestamp_ms: u64,
    }

    /// The reponse from the oracle Phat Contract (copied from Phat Contract)
    #[derive(Debug, PartialEq, Eq, Encode, Decode, Clone, scale_info::TypeInfo)]
    pub struct ResponseRecord {
        pub owner: AccountId32,
        pub contract_id: H256,
        pub pair: Bytes,
        pub price: u128,
        pub timestamp_ms: u64,
    }
}
