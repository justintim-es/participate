#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame

use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch, traits::{Get, Randomness, Currency}, debug, ensure};
use frame_support::weights::{Weight, Pays, DispatchClass};
use frame_system::ensure_signed;
use frame_system::ensure_root;
use frame_system::offchain::{AppCrypto, CreateSignedTransaction};
use sp_runtime::offchain::http;
use sp_runtime::offchain::storage::StorageValueRef;
use sp_runtime::offchain as participate_offchain;
use sp_runtime::offchain::storage_lock::{StorageLock, BlockAndTime};
use sp_std::vec::Vec;
use sp_std::vec;
use codec::Decode;
use codec::Encode;
use sp_std::boxed::Box;
mod ringbuffer;
use ringbuffer::{RingBufferTrait, RingBufferTransient};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;


/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Trait: frame_system::Trait {
	/// Because this pallet emits events, it depends on the runtime's definition of an event.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Randomness: Randomness<Self::Hash>;
	type Currency: Currency<Self::AccountId> + Send + Sync;
}

// The pallet's runtime storage items.
// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
	// A unique name is used to ensure that the pallet's storage items are isolated.
	// This name may be updated, but each pallet in the runtime must use a unique name.
	// ---------------------------------vvvvvvvvvvvvvv
	trait Store for Module<T: Trait> as TemplateModule {
		// Learn more about declaring storage items:
		// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
		Something get(fn something): Option<u32>;
		Requests: u8;
		Block: T::BlockNumber;
		Humanify: double_map hasher(blake2_128_concat) Vec<u8>, hasher(blake2_128_concat) Vec<u8> => bool;
		Peers get(fn peers): map hasher(blake2_128_concat) Vec<u8> => T::Hash;
		Passphrases: map hasher(blake2_128_concat) T::Hash => bool;
		Blocks: map hasher(blake2_128_concat) T::Hash => Option<T::BlockNumber>;
		BufferMap get(fn get_value): map hasher(twox_64_concat) u8 => T::Hash;
		BufferRange get(fn range): (u8, u8) = (0, 0);
		Hashes: map hasher(blake2_128_concat) T::Hash => Option<T::AccountId>;
		Participates: u8;
	}
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
	pub enum Event<T> 
	where 
	AccountId = <T as frame_system::Trait>::AccountId, 
	Hash = <T as frame_system::Trait>::Hash, 
	BN = <T as frame_system::Trait>::BlockNumber {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, AccountId),
		Participated(AccountId, Hash, BN),
		Earned(AccountId, Hash),
		Dropped(Hash),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		GreaterSmallerThen,
	}
}

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", 	which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;
		fn on_initialize(block: T::BlockNumber) -> Weight {
			<Block<T>>::put(block);
			0
		}
		#[weight = 10_000 + T::DbWeight::get().writes(1)]
		pub fn add_humanify(origin, peer_id: Vec<u8>, passphrase: Vec<u8>) -> dispatch::DispatchResult {
			let origin = ensure_root(origin)?;
			Humanify::insert(peer_id, passphrase, true);
			Ok(())
		}
		#[weight = 10_000 + T::DbWeight::get().writes(1)]
		pub fn add_passphrase(origin, peer_id: Vec<u8>, passphrase: Vec<u8>) -> dispatch::DispatchResult {
			let origin = ensure_root(origin)?;
			let hash = T::Randomness::random(&peer_id[..]);
			<Peers<T>>::insert(peer_id, &hash);
			<Passphrases<T>>::insert(hash, true);
			Ok(())
		}
		#[weight = (100_000, DispatchClass::Normal, Pays::No)]
		pub fn participate(origin, random: u8) -> dispatch::DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(random > 1 && random < 11, Error::<T>::GreaterSmallerThen);
			let random_hash = T::Randomness::random(&[Participates::get()]);
			Participates::mutate(|n| *n += 1);
			<Hashes<T>>::insert(&random_hash, &origin);
			let block = <Block<T>>::get();
			let mut queue = Self::queue_transient();
			let mut i = 0;
			let mut truschue = true;
			while let Some(hash) = queue.pop() {
				if let Some(blocks) = <Blocks<T>>::get(&hash) {
					if blocks < block {
						if let Some(account) = <Hashes<T>>::get(&hash) {
							T::Currency::deposit_creating(&account, 1000000000.into());
							<Blocks<T>>::insert(&hash, block + 7500.into());
							Self::deposit_event(RawEvent::Earned(account, hash.clone()));
						}
					}
				}
				if i % random != 0 && !truschue {
					queue.push(hash.clone());
				} else {
					<Blocks<T>>::take(&hash);
					Self::deposit_event(RawEvent::Dropped(hash.clone()));
				}
				truschue = false;
				i += 1;
			}
			<Blocks<T>>::insert(&random_hash, block + 7500.into());
			queue.push(random_hash.clone());
			Self::deposit_event(RawEvent::Participated(origin, random_hash, block));
			debug::info!("got here");
			Ok(())
		}

		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[weight = 10_000 + T::DbWeight::get().writes(1)]
		pub fn do_something(origin, something: u32) -> dispatch::DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://substrate.dev/docs/en/knowledgebase/runtime/origin
			let who = ensure_signed(origin)?;

			// Update storage.
			Something::put(something);

			// Emit an event.
			Self::deposit_event(RawEvent::SomethingStored(something, who));
			// Return a successful DispatchResult
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn cause_error(origin) -> dispatch::DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match Something::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValue)?,
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					Something::put(new);
					Ok(())
				},
			}
		}
	}
}

impl<T: Trait> Module<T> {
	fn queue_transient() -> Box<dyn RingBufferTrait<T::Hash>> {
		Box::new(RingBufferTransient::<
			T::Hash,
			<Self as Store>::BufferRange,
			<Self as Store>::BufferMap,
			u8,
		>::new())
	}
	pub fn get_living_hashes() -> Vec<T::Hash> {
		let mut queue = Self::queue_transient();
		let mut veschec = Vec::new();
		while let Some(hash) = queue.pop() {
			veschec.push(hash);
		}
		veschec
	}
}