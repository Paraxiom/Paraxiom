#![cfg_attr(not(feature = "std"), no_std)]


use sp_application_crypto::RuntimeAppPublic;


pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	


    
    decl_module! {
		pub struct Module<T: Trait> for enum Call where origin: T::Origin {
			fn deposit_event() = default;

			/// Sets a contract's ABI.
			#[weight = 0]
			pub fn set_contract_abi(origin, contract_id: T::AccountId, contract_abi: T::ContractAbi) -> DispatchResult {
				let origin = ensure_signed(origin)?;
				OracleReporterRegistry::<T>::insert(contract_id, contract_abi);

				Self::deposit_event(RawEvent::SetContractAbi(origin.clone(), contract_abi));
				Ok(())
			}

			/// Deletes a contract's ABI.
			#[weight = 0]
			pub fn delete_contract_abi(origin, contract_id: T::AccountId) -> DispatchResult {
				let origin = ensure_signed(origin)?;
				OracleReporterRegistry::<T>::remove(contract_id);

				Self::deposit_event(RawEvent::DeleteContractAbi(origin.clone()));
				Ok(())
			}
		}
	}

}
	

