//! # Sample Oracle based on Offchain Rollup
#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;
pub use pallet::*;

mod types;

#[frame_support::pallet]
pub mod pallet {
    use core::convert::TryFrom;

    use crate::types::*;
    use frame_support::{
        dispatch::DispatchResult, pallet_prelude::*, traits::Randomness, traits::StorageVersion,
        transactional, Blake2_128Concat, Twox64Concat,
    };
    use frame_system::pallet_prelude::*;
    use pallet_registry::types::RegistryFeedKey;
    use phat_offchain_rollup::{anchor, types::*};
    use sp_core::H256;
    use sp_runtime::AccountId32;
    use sp_std::vec::Vec;

    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_registry::Config + phat_offchain_rollup::anchor::Config
    {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type QuotesCount: Get<u32>;
        type DataCount: Get<u32>;
        type OracleRandomness: Randomness<Self::Hash, Self::BlockNumber>;
    }

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

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

    /// Mapping from (deployer, RequestData, FeedData)
    #[pallet::storage]
    #[pallet::getter(fn feed_data)]
    pub type FeedData<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        AccountId32,
        Blake2_128Concat,
        Bytes,
        BoundedVec<RequestData, T::DataCount>,
    >;

    /// Mapping for request ID -> (caller, payload, nonce)
    #[pallet::storage]
    pub type FeedRequests<T: Config> = StorageMap<_, Twox64Concat, RequestId, Request<T>>;

    #[pallet::storage]
    #[pallet::getter(fn averages)]
    pub type Averages<T: Config> = StorageMap<_, Twox64Concat, Bytes, u128>;

    // Events
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Price quote received
        QuoteReceived {
            contract: H256,
            submitter: T::AccountId,
            owner: AccountId32,
            pair: Bytes,
            price: u128,
        },

        ResponseRecordReceived {
            submitter: T::AccountId,
            phat_contract_id: H256,
            request_id: RequestId,
            response_data: ResponseData,
            timestamp_ms: u64,
        },

        /// New feed requested
        OracleRequest {
            caller: T::AccountId,
            request_key: RequestId,
            registry_feed_key: RegistryFeedKey<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        FailedToAuthenticateResponse,
        FailedToDecodeResponse,
        FailedToPushResponse,
        FailedToPushMessageToAnchor,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sends a request to the oracle
        #[pallet::weight(0)]
        #[pallet::call_index(0)]
        #[transactional]
        pub fn request(
            origin: OriginFor<T>,
            registry_feed_key: RegistryFeedKey<T>,
            name: H256,
            data: ValueBytes,
            nonce: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // generate a random seed from the randomness pallet and mix it
            // with the data to get a unique seed for this request
            let seed = (T::OracleRandomness::random_seed().0, data.clone()).encode();
            // use random seed to create an idempotent request_id
            let (request_id, _) = T::OracleRandomness::random(&seed);
            let request_key = H256::from_slice(request_id.as_ref());

            // update storage to keep track of this request
            FeedRequests::<T>::insert(
                request_key,
                Request {
                    registry_feed_key: registry_feed_key.clone(),
                    nonce,
                    caller: who.clone(),
                    requested_data: data.clone(),
                },
            );

            // sends request to rollup
            anchor::pallet::Pallet::<T>::push_message(&name, data)
                .map_err(|_| Error::<T>::FailedToPushMessageToAnchor)?;

            Self::deposit_event(Event::OracleRequest {
                caller: who.clone(),
                request_key,
                registry_feed_key,
            });

            Ok(())
        }

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
            let resp: ResponseRecord<T> =
                Decode::decode(&mut &data[..]).map_err(|_| Error::<T>::FailedToDecodeResponse)?;

            ensure!(
                resp.phat_contract_id == name,
                Error::<T>::FailedToAuthenticateResponse
            );

            // // TODO: remember to remove the request from `FeedRequests` storage
            // //       since its no longer needed.

            // let storage_map = PriceFeeds::<T>::drain().collect::<Vec<_>>();
            // let mut pricequotes = BoundedVec::<PriceQuote, T::QuotesCount>::default();

            // for (_i, j, mut k) in storage_map {
            //     if j == resp.pair {
            //         if k.len() == T::QuotesCount::get() as usize {
            //             k.drain(..T::QuotesCount::get() as usize);
            //         }
            //         k.try_push({
            //             PriceQuote {
            //                 contract_id: resp.contract_id,
            //                 price: resp.price,
            //                 timestamp_ms: resp.timestamp_ms,
            //             }
            //         })
            //         .or(Err(Error::<T>::FailedToPushResponse))?;
            //         pricequotes = k;
            //     }
            // }
            // PriceFeeds::<T>::insert(&resp.owner, resp.pair.clone(), pricequotes);

            Self::deposit_event(Event::ResponseRecordReceived {
                phat_contract_id: name,
                submitter,
                response_data: resp.response_data,
                request_id: resp.request_id,
                timestamp_ms: resp.timestamp_ms,
            });

            Ok(())
        }
    }
}
