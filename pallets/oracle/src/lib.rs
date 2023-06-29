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
    use pallet_registry::ApiFeed;
    use phat_offchain_rollup::{anchor, types::*};
    use scale_info::Registry;
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
            request_id: RequestId,
            registry_feed_key: RegistryFeedKey<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        FailedToAuthenticateResponse,
        FailedToDecodeResponse,
        FailedToPushResponse,
        FailedToPushMessageToAnchor,
        FailedToGetApiFeed,
        ApiFeedNotActive,
        FailedToEncodeData,
        FailedToFindOracleFeeds,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// An oracle request for information.
        ///
        /// `registry_feed_key`: a feed identifier that can be found in the Registry.
        /// `nonce`: an incrementing number provided by the client.
        ///
        /// The method fetches the URL and path associated with the feed requested
        /// and sends off a message to the phat contract(s) via rollup request-response.
        #[pallet::weight(0)]
        #[pallet::call_index(0)]
        #[transactional]
        pub fn request(
            origin: OriginFor<T>,
            registry_feed_key: RegistryFeedKey<T>,
            nonce: u128,
        ) -> DispatchResult {
            // TODO: weight should be set dynamically based on how many oracle phat contracts
            //       are called. Same for the fees.
            let who = ensure_signed(origin)?;

            // get feed information from the registry pallet
            let api_feed: ApiFeed<T> = pallet_registry::ApiFeeds::<T>::get(&who, &registry_feed_key)
                .ok_or(Error::<T>::FailedToGetApiFeed)?;
            let feed_status = api_feed.status;
            ensure!(feed_status.is_active(), Error::<T>::ApiFeedNotActive);
            let feed_path = api_feed.path;
            let feed_url = api_feed.url;

            // generate a random request ID
            let seed = (T::OracleRandomness::random_seed(), nonce).0.encode();
            let (request_id, _) = T::OracleRandomness::random(&seed);
            let request_id = H256::from_slice(request_id.as_ref());

            // encode the phat contract request
            let data_raw = (request_id, feed_url, feed_path);
            let data = BoundedVec::try_from(data_raw.encode())
                .map_err(|_| Error::<T>::FailedToEncodeData)?;

            // update storage to keep track of this request
            FeedRequests::<T>::insert(
                request_id,
                Request {
                    registry_feed_key: registry_feed_key.clone(),
                    nonce,
                    caller: who.clone(),
                    requested_data: data.clone(),
                },
            );

            // TODO: No need to get all the names, add anchor pallet storage
            //       to remove the need for inefficient iteration by key.
            //       Also shouldn't pick the name randomly.
            //       Multiple names can be selected and messages sent to each.
            let name = anchor::pallet::SubmitterByNames::iter_keys()
                .last()
                .ok_or(Error::<T>::FailedToFindOracleFeeds)?;

            // send request to rollup
            anchor::pallet::Pallet::<T>::push_message(&name, data)
                .map_err(|_| Error::<T>::FailedToPushMessageToAnchor)?;

            Self::deposit_event(Event::OracleRequest {
                caller: who.clone(),
                request_id,
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
            
            todo!("implement an average aggergator");
            
            // let storage_map = PriceFeeds::<T>::iter().collect::<Vec<_>>();
            // let mut averages: Vec<(Bytes, u128)> = Vec::new();

            // storage_map.iter().for_each(|(_i, _j, k)| {
            //     let sum: u128 = k.iter().map(|j| j.price).sum::<u128>();
            //     let count = k.len() as u128;
            //     let average = sum / count;
            //     averages.push((_j.clone(), average));
            // });

            // for (pair, average) in averages {
            //     Averages::<T>::insert(&pair, average);
            // }
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

            // store data
            // compute aggregation

            // let feed_data = FeedData::<T>::insert(submitter, resp.request_data, resp.response_data);

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
