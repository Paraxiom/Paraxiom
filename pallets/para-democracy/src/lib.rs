#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet_democracy;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config + pallet_democracy::Config {}
