#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::fungibles;
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_support::traits::fungible;
use frame_support::sp_runtime::traits::AccountIdConversion;
use frame_support::traits::fungibles::Inspect;

pub type AssetIdOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<
	<T as frame_system::Config>::AccountId,
>>::AssetId;

pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<
	<T as frame_system::Config>::AccountId,
>>::Balance;

pub type AssetBalanceOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<
	<T as frame_system::Config>::AccountId,
>>::Balance;

#[frame_support::pallet]
pub mod pallet {

use crate::*;
	use frame_support::{
		pallet_prelude::{*}, traits::{
			fungible::{self}, fungibles::{self}, tokens::{
				Fortitude::{Polite}, Precision::{Exact}, Preservation::{Expendable, Preserve}
			},
		}, Blake2_128Concat, PalletId
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::{One, Zero}, Percent};
	use frame_support::traits::fungibles::Create;
	use frame_support::traits::fungibles::Mutate as FsMutate;
	use frame_support::traits::fungible::Mutate as FMutate;
	use sp_runtime::traits::IntegerSquareRoot;
	use sp_runtime::traits::CheckedAdd;
	use sp_runtime::traits::CheckedSub;
	use sp_runtime::traits::CheckedMul;
	use sp_runtime::traits::CheckedDiv;
	use sp_runtime::Saturating;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances Pallet.
		type NativeBalance: fungible::Inspect<Self::AccountId>
			+ fungible::Mutate<Self::AccountId>
			+ fungible::hold::Inspect<Self::AccountId>
			+ fungible::hold::Mutate<Self::AccountId>
			+ fungible::freeze::Inspect<Self::AccountId>
			+ fungible::freeze::Mutate<Self::AccountId>;

		/// Type to access the Assets Pallet.
		type Fungibles: fungibles::Inspect<Self::AccountId, Balance = BalanceOf<Self>, AssetId = u32>
			+ fungibles::Mutate<Self::AccountId>
			+ fungibles::Create<Self::AccountId>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type PoolCreateFee: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MinLiquidity: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MinBalance: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type PoolFeePercent: Get<u8>;
	}

	/// A struct representing a pair of ordered assets
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebugNoBound, MaxEncodedLen, TypeInfo, Ord, PartialOrd)]
	#[scale_info(skip_type_params(T))]
	pub struct AssetPair<T: Config> {
		/// The first asset in the pair.
		asset_a: AssetIdOf<T>,
		/// The second asset in the pair.
		asset_b: AssetIdOf<T>,
	}

	impl<T: Config> AssetPair<T> {
		pub fn new(asset_a: AssetIdOf<T>, asset_b: AssetIdOf<T>) -> Result<Self, Error<T>> {
			ensure!(asset_a != asset_b, Error::<T>::IdenticalAssets);
			
			let (ordered_asset_a, ordered_asset_b) = if asset_a < asset_b {
				(asset_a, asset_b)
			} else {
				(asset_b, asset_a)
			};
	
			Ok(
				AssetPair {
					asset_a: ordered_asset_a,
					asset_b: ordered_asset_b,
				}
			)
		}
	}

	/// A struct representing information about a liquidity pool.
	/// Contains the identifier of the associated liquidity pool (LP) token.
	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebugNoBound, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct PoolInfo<T: Config> {
		pub lp_asset: AssetIdOf<T>,
		pub reserve_a: BalanceOf<T>,
		pub reserve_b: BalanceOf<T>,
	}

	impl<T: Config> PoolInfo<T> {
		fn add_reserves(&self, amount_a: &BalanceOf<T>, amount_b: &BalanceOf<T>) -> Result<Self, Error<T>> {
			let reserve_a = self.reserve_a.checked_add(&amount_a).ok_or(Error::<T>::Overflow)?;
			let reserve_b = self.reserve_b.checked_add(&amount_b).ok_or(Error::<T>::Overflow)?;

			Ok(
				PoolInfo {
					lp_asset: self.lp_asset,
					reserve_a,
					reserve_b
				}
			)
		}

		fn sub_reserves(&self, amount_a: &BalanceOf<T>, amount_b: &BalanceOf<T>) -> Result<Self, Error<T>> {
			let reserve_a = self.reserve_a.checked_sub(&amount_a).ok_or(Error::<T>::Overflow)?;
			let reserve_b = self.reserve_b.checked_sub(&amount_b).ok_or(Error::<T>::Overflow)?;

			Ok(
				PoolInfo {
					lp_asset: self.lp_asset,
					reserve_a,
					reserve_b
				}
			)
		}

		fn add_sub_reserves(&self, amount_a: &BalanceOf<T>, amount_b: &BalanceOf<T>) -> Result<Self, Error<T>> {
			let reserve_a = self.reserve_a.checked_add(&amount_a).ok_or(Error::<T>::Overflow)?;
			let reserve_b = self.reserve_b.checked_sub(&amount_b).ok_or(Error::<T>::Overflow)?;

			Ok(
				PoolInfo {
					lp_asset: self.lp_asset,
					reserve_a,
					reserve_b
				}
			)
		}
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	pub type Pools<T: Config> = StorageMap<_, Blake2_128Concat, AssetPair<T>, PoolInfo<T>>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a new liquidity pool is created.
		PoolCreated { creator: T::AccountId, pair: AssetPair<T>, lp_asset: AssetIdOf<T> },

		/// Event emitted when liquidity is added to a pool.
		LiquidityAdded { sender: T::AccountId, pair: AssetPair<T>, lp_asset: AssetIdOf<T>, liqudity_provided_a: BalanceOf<T>, liqudity_provided_b: BalanceOf<T>, minted: BalanceOf<T> },
		
		/// Event emitted when liquidity is removed from a pool.
		LiquidityRemoved { sender: T::AccountId, pair: AssetPair<T>, lp_asset: AssetIdOf<T>, liqudity_removed_a: BalanceOf<T>, liqudity_removed_b: BalanceOf<T>, burned: BalanceOf<T>, removal_fee: BalanceOf<T> },
		
		/// Event emitted when assets are swapped in a trade.
		Swaped { sender: T::AccountId, asset_in: AssetIdOf<T>, amount_in: BalanceOf<T>, asset_out: AssetIdOf<T>, amount_out: BalanceOf<T> } 
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Indicates an arithmetic overflow.
		Overflow,

		/// Indicates an attempt to create identical assets.
		IdenticalAssets,

		/// Indicates an attempt to create a pool that already exists.
		PoolExists,

		/// Indicates that a requested pool was not found.
		PoolNotFound,

		/// Indicates an invalid pool asset ID.
		InvalidPoolAssetId,

		/// Indicates that an asset does not exist.
		AssetNotExists,

		/// Indicates an attempt to use an already taken asset ID.
		AssetIdAreadyTaken,

		/// Indicates that a liquidity pool token does not exist.
		LiquidityPoolTokenNotExists,

		/// Indicates an invalid desired amount.
		InvalidDesiredAmount,

		/// Indicates an invalid amount.
		InvalidAmount,

		/// Indicates an invalid liquidity amount.
		InvalidLiquidityAmount,

		/// Indicates an insufficient amount for a particular operation.
		InsufficientAmount,

		/// Indicates insufficient liquidity for a particular operation.
		InsufficientLiquidity,

		/// Indicates that the amount is less than the minimal required.
		AmountLessThanMinimal,

		/// Indicates that the reserve is zero.
		ReserveIsZero,

		/// Indicates that the provided minimum is not sufficient for a swap.
		ProvidedMinimumNotSufficientForSwap,

		/// Indicates that the provided maximum is not sufficient for a swap.
		ProvidedMaximumNotSufficientForSwap,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new liquidity pool with the specified assets and LP token.
		/// This function performs various checks before creating the pool and minting the LP token.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::default())]
		pub fn create_pool(
			origin: OriginFor<T>,
			asset_a: AssetIdOf<T>,
			asset_b: AssetIdOf<T>,
			lp_asset: AssetIdOf<T>
		) -> DispatchResult {
			let creator = ensure_signed(origin)?;

			// Check if assets not identical
			ensure!(asset_a != asset_b, Error::<T>::IdenticalAssets);

			// Check if asset_a not exists
			ensure!(Self::asset_exists(&asset_a), Error::<T>::AssetNotExists);
			// Check if asset_b not exists
			ensure!(Self::asset_exists(&asset_b), Error::<T>::AssetNotExists);

			let pair = AssetPair::<T>::new(asset_a, asset_b)?;

			// Check it pair not exists yet
			ensure!(!Pools::<T>::contains_key(&pair), Error::<T>::PoolExists);

			ensure!(!Self::asset_exists(&lp_asset), Error::<T>::AssetIdAreadyTaken);

			// Pay the creation pool fees
			T::NativeBalance::transfer(&creator, &Self::account_id(), T::PoolCreateFee::get(), Preserve)?;
			
			// Create a new LP token with a existential deposit of 1.
			T::Fungibles::create(
				lp_asset,
				Self::account_id(),
				true,
				T::MinBalance::get()
			)?;

			// Create a PoolInfo struct with LP asset information.
			let pool_info = PoolInfo::<T> {
				lp_asset: lp_asset.clone(),
				reserve_a: Zero::zero(),
				reserve_b: Zero::zero(),
			};
			
			// Insert the new pool information into the Pools storage.
			Pools::<T>::insert(pair.clone(), pool_info);

			// Emit a PoolCreated event to notify clients about the new pool creation.
			Self::deposit_event(Event::PoolCreated { creator, pair, lp_asset });

			Ok(())
		}

		/// Add liquidity to an existing pool by providing assets A and B.
		/// This function calculates the optimal amount of assets to contribute based on the current reserves.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::default())]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			asset_a: AssetIdOf<T>,
			asset_b: AssetIdOf<T>,
			amount_a_desired: BalanceOf<T>,
			amount_b_desired: BalanceOf<T>,
			amount_a_min: BalanceOf<T>,
			amount_b_min: BalanceOf<T>,
			mint_to: T::AccountId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_add_liquidity(
				sender,
				asset_a,
				asset_b,
				amount_a_desired,
				amount_b_desired,
				amount_a_min,
				amount_b_min,
				mint_to,
			)
		}

		/// Remove liquidity from an existing pool by burning LP tokens.
		/// This function calculates the amount of assets A and B to be received in return for burned LP tokens.
		#[pallet::call_index(3)]
		#[pallet::weight(Weight::default())]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			asset_a: AssetIdOf<T>,
			asset_b: AssetIdOf<T>,
			lp_token_burn: BalanceOf<T>,
			amount_a_min_receive: BalanceOf<T>,
			amount_b_min_receive: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_remove_liquidity(
				sender,
				asset_a,
				asset_b,
				lp_token_burn,
				amount_a_min_receive,
				amount_b_min_receive,
			)
		}

		/// Execute a swap where the sender provides a specific amount of input asset to receive at least a minimum amount of output asset.
		/// This function performs the swap and calculates the resulting amount based on the current reserves.
		#[pallet::call_index(4)]
		#[pallet::weight(Weight::default())]
		pub fn swap_exact_asset_for_asset(
			origin: OriginFor<T>,
			exact_in: AssetIdOf<T>,
			asset_out: AssetIdOf<T>,
			amount_in: BalanceOf<T>,
			amount_out_min: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_swap_exact_asset_for_asset(
				sender,
				exact_in,
				asset_out,
				amount_in,
				amount_out_min,
			)?;

			Ok(())
		}

		/// Execute a swap where the sender provides a maximum amount of input asset to receive a specific amount of output asset.
		/// This function performs the swap and calculates the resulting amount based on the current reserves.
		#[pallet::call_index(5)]
		#[pallet::weight(Weight::default())]
		pub fn swap_asset_for_exact_asset(
			origin: OriginFor<T>,
			asset_in: AssetIdOf<T>,
			exact_out: AssetIdOf<T>,
			amount_out: BalanceOf<T>,
			amount_in_max: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_swap_asset_for_exact_asset(
				sender,
				asset_in,
				exact_out,
				amount_out,
				amount_in_max,
			)?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn do_add_liquidity(
			sender: T::AccountId,
			asset_a: AssetIdOf<T>,
			asset_b: AssetIdOf<T>,
			amount_a_desired: BalanceOf<T>,
			amount_b_desired: BalanceOf<T>,
			amount_a_min: BalanceOf<T>,
			amount_b_min: BalanceOf<T>,
			mint_to: T::AccountId,
		) -> Result<(), DispatchError> {
			ensure!(asset_a != asset_b, Error::<T>::IdenticalAssets);

			let pair = AssetPair::<T>::new(asset_a, asset_b)?;

			let pool_info = Pools::<T>::get(&pair).ok_or(Error::<T>::PoolNotFound)?;

			// Ensure that the LP token associated with the pool exists.
			ensure!(Self::asset_exists(&pool_info.lp_asset), Error::<T>::LiquidityPoolTokenNotExists);

			ensure!(amount_a_desired > Zero::zero() && amount_b_desired > Zero::zero(), Error::<T>::InvalidDesiredAmount);
			
			// Retrieve the current reserves of assets A and B.
			let reserve_a = pool_info.reserve_a;
			let reserve_b = pool_info.reserve_b;

			// Calculate the optimal amounts of assets A and B to contribute.
			let amount_a: BalanceOf<T>;
			let amount_b: BalanceOf<T>;
			if reserve_a.is_zero() || reserve_b.is_zero() {
				// If either reserve is zero, use the desired amounts.
				amount_a = amount_a_desired;
				amount_b = amount_b_desired;
			} else {
				// Calculate the optimal amount of asset B based on the desired amount of asset A.
				let amount_b_optimal = Self::quote(amount_a_desired, reserve_a, reserve_b)?;
			
				// Compare with the desired amount of asset B and ensure it meets the minimum requirement.
				if amount_b_optimal <= amount_b_desired {
					ensure!(amount_b_optimal >= amount_b_min, Error::<T>::InsufficientAmount);
					amount_a = amount_a_desired;
					amount_b = amount_b_optimal;
				} else {
					// Calculate the optimal amount of asset A based on the desired amount of asset B.
					let amount_a_optimal = Self::quote(amount_b_desired.clone(), reserve_b, reserve_a)?;

					ensure!(amount_a_optimal <= amount_a_desired, Error::<T>::InsufficientAmount);
					ensure!(amount_a_optimal >= amount_a_min, Error::<T>::InsufficientAmount);

					amount_a = amount_a_optimal;
					amount_b = amount_b_desired;
				}
			}

			// Transfer assets from sender account to pallet account
			T::Fungibles::transfer(asset_a, &sender, &Self::account_id(), amount_a, Preserve)?;
			T::Fungibles::transfer(asset_b, &sender, &Self::account_id(), amount_b, Preserve)?;

			let total_supply = Self::total_issuance(pool_info.lp_asset.clone());

			// Calculate the amount of LP tokens to mint based on the provided assets.
			let lp_asset_amount: BalanceOf<T>;
			if total_supply.is_zero() {
				lp_asset_amount = Self::get_lp_amount_for_zero_supply(amount_a, amount_b)?;
				
				T::Fungibles::mint_into(
					pool_info.lp_asset.clone(),
					&Self::account_id(),
					T::MinLiquidity::get(),
				)?;
			} else {
				let a = Self::mul_div(amount_a, total_supply, reserve_a)?;
                let b = Self::mul_div(amount_b, total_supply, reserve_b)?;
                lp_asset_amount = a.min(b);
			}

			// Mint the calculated amount of LP tokens to the specified account.
			T::Fungibles::mint_into(pool_info.lp_asset.clone(), &mint_to, lp_asset_amount)?;

			// Update reserves
			let pool_info = pool_info.add_reserves(&amount_a, &amount_b)?;
			Pools::<T>::set(&pair, Some(pool_info.clone()));

			Self::deposit_event(Event::<T>::LiquidityAdded {
				sender,
				pair,
				liqudity_provided_a: amount_a,
				liqudity_provided_b: amount_b,
				lp_asset: pool_info.lp_asset, 
				minted: lp_asset_amount
			});

			Ok(())
		}

		pub fn update_reserves() {

		}

		pub fn do_remove_liquidity(
			sender: T::AccountId,
			asset_a: AssetIdOf<T>,
			asset_b: AssetIdOf<T>,
			lp_token_burn: BalanceOf<T>,
			amount_a_min_receive: BalanceOf<T>,
			amount_b_min_receive: BalanceOf<T>,
		) -> Result<(), DispatchError> {
			ensure!(asset_a != asset_b, Error::<T>::IdenticalAssets);

			ensure!(lp_token_burn > Zero::zero(), Error::<T>::InvalidLiquidityAmount);

			let pair  = AssetPair::<T>::new(asset_a, asset_b)?;
			let pool_info = Pools::<T>::get(pair.clone())
				.ok_or(Error::<T>::PoolNotFound)?;
			
			let reserve_a = pool_info.reserve_a;
			let reserve_b = pool_info.reserve_b;

			let total_supply = Self::total_issuance(pool_info.lp_asset.clone());
			let removal_fee_amount = Self::pool_fee(lp_token_burn);

			let lp_redeem_amount = lp_token_burn.saturating_sub(removal_fee_amount);

			// Calculate the amounts of assets A and B to be received in return for the burned LP tokens.
			let amount_a = Self::mul_div(lp_redeem_amount, reserve_a, total_supply)?;
			let amount_b = Self::mul_div(lp_redeem_amount, reserve_b, total_supply)?;

			ensure!(
				!amount_a.is_zero() && amount_a >= amount_a_min_receive,
				Error::<T>::InsufficientAmount
			);
			ensure!(
				!amount_b.is_zero() && amount_b >= amount_b_min_receive,
				Error::<T>::InsufficientAmount
			);
			// Ensure that the remaining reserves of assets A and B meet the minimum balance requirements.
			let reserve_a_left = reserve_a.saturating_sub(amount_a);
			let reserve_b_left = reserve_b.saturating_sub(amount_b);
			
			ensure!(
				reserve_a_left >= T::Fungibles::minimum_balance(asset_a.clone()),
				Error::<T>::AmountLessThanMinimal
			);
			ensure!(
				reserve_b_left >= T::Fungibles::minimum_balance(asset_b.clone()),
				Error::<T>::AmountLessThanMinimal
			);

			// Burn the specified amount of LP tokens from the sender's account.
			T::Fungibles::burn_from(pool_info.lp_asset.clone(), &sender, lp_token_burn, Exact, Polite)?;

			// Update reserves
			let pool_info = pool_info.sub_reserves(&amount_a, &amount_b)?;
			Pools::<T>::set(&pair, Some(pool_info.clone()));

			// Transfer the received amounts of assets A and B back to the sender's account.
			T::Fungibles::transfer(asset_a, &Self::account_id(), &sender, amount_a, Expendable)?;
			T::Fungibles::transfer(asset_b, &Self::account_id(), &sender, amount_b, Expendable)?;

			Self::deposit_event(Event::LiquidityRemoved {
				sender, 
				pair, 
				lp_asset: pool_info.lp_asset, 
				liqudity_removed_a: amount_a,
				liqudity_removed_b: amount_b,
				burned: lp_token_burn,
				removal_fee: removal_fee_amount
			});

			Ok(())
		}
		
		/// Internal helper function for executing a swap where the sender provides a specific amount of input asset to receive at least a minimum amount of output asset.
		/// This function performs the swap and calculates the resulting amount based on the current reserves.
		pub fn do_swap_exact_asset_for_asset(
			sender: T::AccountId,
			exact_in: AssetIdOf<T>,
			asset_out: AssetIdOf<T>,
			amount_in: BalanceOf<T>,
			amount_out_min: BalanceOf<T>
		) -> Result<BalanceOf<T>, DispatchError> {
			ensure!(amount_in > Zero::zero(), Error::<T>::InvalidAmount);
			ensure!(amount_out_min > Zero::zero(), Error::<T>::InvalidAmount);

			let pair = AssetPair::<T>::new(exact_in, asset_out)?;
			let mut pool_info = Pools::<T>::get(&pair).ok_or(Error::<T>::PoolNotFound)?;

			let reserve_in = pool_info.reserve_a;
			let reserve_out = pool_info.reserve_b;

			let amount_out: BalanceOf<T>;
			// Calculate the amount of output asset based on the provided input amount and reserves.
			if exact_in < asset_out {
				amount_out = Self::get_amount_out(amount_in.clone(), reserve_in, reserve_out)?;
				pool_info.reserve_a = pool_info.reserve_a.checked_add(&amount_in).ok_or(Error::<T>::Overflow)?;
				pool_info.reserve_b = pool_info.reserve_b.checked_sub(&amount_out).ok_or(Error::<T>::Overflow)?;
			} else {
				amount_out = Self::get_amount_out(amount_in.clone(), reserve_out, reserve_in)?;
				pool_info.reserve_a = pool_info.reserve_a.checked_sub(&amount_out).ok_or(Error::<T>::Overflow)?;
				pool_info.reserve_b = pool_info.reserve_b.checked_add(&amount_in).ok_or(Error::<T>::Overflow)?;
			}

			ensure!(amount_out >= amount_out_min, Error::<T>::ProvidedMinimumNotSufficientForSwap);

			// Transfer the provided amount of input asset from the sender to the pallet's account.
			T::Fungibles::transfer(exact_in.clone(), &sender, &Self::account_id(), amount_in.clone(), Preserve)?;
			
			// Transfer the calculated amount of output asset from the pallet's account to the sender.
			T::Fungibles::transfer(asset_out.clone(), &Self::account_id(), &sender, amount_out.clone(), Preserve)?;

			// Update reserves
			Pools::<T>::set(&pair, Some(pool_info.clone()));

			Self::deposit_event(
				Event::Swaped {
					sender,
					asset_in: exact_in,
					amount_in,
					asset_out,
					amount_out,
				}
			);

			Ok(amount_out)
		}

		/// Internal helper function for executing a swap where the sender provides a maximum amount of input asset to receive a specific amount of output asset.
		/// This function performs the swap and calculates the resulting amount based on the current reserves.
		pub fn do_swap_asset_for_exact_asset(
			sender: T::AccountId,
			asset_in: AssetIdOf<T>,
			exact_out: AssetIdOf<T>,
			amount_out: BalanceOf<T>,
			amount_in_max: BalanceOf<T>
		) -> Result<BalanceOf<T>, DispatchError> {
			ensure!(amount_out > Zero::zero(), Error::<T>::InvalidAmount);
			ensure!(amount_in_max > Zero::zero(), Error::<T>::InvalidAmount);

			let pair = AssetPair::<T>::new(asset_in, exact_out)?;
			let mut pool_info = Pools::<T>::get(&pair).ok_or(Error::<T>::PoolNotFound)?;

			let reserve_in = pool_info.reserve_a;
			let reserve_out = pool_info.reserve_b;

			let amount_in: BalanceOf<T>;
			// Calculate the amount of input asset based on the desired output amount and reserves.
			if asset_in < exact_out {
				amount_in = Self::get_amount_in(amount_out.clone(), reserve_in, reserve_out)?;
				pool_info.reserve_a = pool_info.reserve_a.checked_add(&amount_in).ok_or(Error::<T>::Overflow)?;
				pool_info.reserve_b = pool_info.reserve_b.checked_sub(&amount_out).ok_or(Error::<T>::Overflow)?;
			} else {
				amount_in = Self::get_amount_in(amount_out.clone(), reserve_out, reserve_in)?;
				pool_info.reserve_a = pool_info.reserve_a.checked_sub(&amount_out).ok_or(Error::<T>::Overflow)?;
				pool_info.reserve_b = pool_info.reserve_b.checked_add(&amount_in).ok_or(Error::<T>::Overflow)?;
			}

			ensure!(amount_in <= amount_in_max, Error::<T>::ProvidedMaximumNotSufficientForSwap);

			// Transfer the calculated amount of input asset from the sender to the pallet's account.
			T::Fungibles::transfer(asset_in.clone(), &sender, &Self::account_id(), amount_in.clone(), Preserve)?;
		
			// Transfer the desired amount of output asset from the pallet's account to the sender.
			T::Fungibles::transfer(exact_out.clone(), &Self::account_id(), &sender, amount_out.clone(), Preserve)?;

			// Update reserves
			Pools::<T>::set(&pair, Some(pool_info.clone()));

			Self::deposit_event(
				Event::Swaped {
					sender,
					asset_in,
					amount_in,
					asset_out: exact_out,
					amount_out,
				}
			);

			Ok(amount_out)
		}

		/// Calculate the amount of output asset that will be received for a given amount of input asset in a swap.
		/// Takes into account the current reserves of the input and output assets.
		pub fn get_amount_out(
			amount_in: BalanceOf<T>,
			reserve_in: BalanceOf<T>,
			reserve_out: BalanceOf<T>,
		) -> Result<BalanceOf<T>, Error<T>> {
			ensure!(!reserve_in.is_zero() || !reserve_out.is_zero(), Error::<T>::ReserveIsZero);

			// Apply a fee to the input amount.
			let amount_in_with_fee = amount_in
				.checked_sub(&Self::pool_fee(amount_in))
                .ok_or(Error::<T>::Overflow)?;

			// Calculate the numerator.
            let numerator =
                amount_in_with_fee.checked_mul(&reserve_out).ok_or(Error::<T>::Overflow)?;

			// Calculate the denominator.
            let denominator = reserve_in
                .checked_add(&amount_in_with_fee)
                .ok_or(Error::<T>::Overflow)?;

			// Calculate the result using the formula: result = numerator / denominator.
            let result = numerator.checked_div(&denominator).ok_or(Error::<T>::Overflow)?;

            result.try_into().map_err(|_| Error::<T>::Overflow)
		}

		/// Calculate the amount of input asset required to receive a specific amount of output asset in a swap.
		/// Takes into account the current reserves of the input and output assets.
		pub fn get_amount_in(
			amount_out: BalanceOf<T>,
			reserve_in: BalanceOf<T>,
			reserve_out: BalanceOf<T>,
		) -> Result<BalanceOf<T>, Error<T>> {
			ensure!(!reserve_in.is_zero() || !reserve_out.is_zero(), Error::<T>::ReserveIsZero);

			ensure!(amount_out < reserve_out, Error::<T>::InsufficientAmount);

			// Calculate the numerator.
			let numerator = reserve_in
				.checked_mul(&amount_out)
				.ok_or(Error::<T>::Overflow)?;

			// Calculate the denominator.
			let denominator = reserve_out
				.checked_sub(&Self::pool_fee(amount_out))
				.ok_or(Error::<T>::Overflow)?;

			// Calculate the result using the formula: result = (numerator / denominator) + 1.
			let result = numerator
				.checked_div(&denominator)
				.ok_or(Error::<T>::Overflow)?
				.checked_add(&One::one())
				.ok_or(Error::<T>::Overflow)?;

			result.try_into().map_err(|_| Error::<T>::Overflow)
		}

		/// Calculate the estimated output amount for swapping a given `amount` of `asset_a` to `asset_b`.
		pub fn calc_price_exact_asset_for_asset(
			asset_a: AssetIdOf<T>,
			asset_b: AssetIdOf<T>,
			amount: BalanceOf<T>,
		) -> Option<BalanceOf<T>> {
			let pair = AssetPair::<T>::new(asset_a, asset_b).ok()?;
			let pool_info = Pools::<T>::get(&pair).ok_or(Error::<T>::PoolNotFound).ok()?;

			let balance_1 = pool_info.reserve_a;
			let balance_2 = pool_info.reserve_b;
			
			if !balance_1.is_zero() {
				Self::quote(amount, balance_1, balance_2).ok()
			} else {
				None
			}
		}

		/// Calculate the estimated input amount for swapping a given `amount` of `asset_b` for `asset_a`.
		pub fn calc_price_asset_for_exact_asset(
			asset_a: AssetIdOf<T>,
			asset_b: AssetIdOf<T>,
			amount: BalanceOf<T>,
		) -> Option<BalanceOf<T>> {
			let pair = AssetPair::<T>::new(asset_a, asset_b).ok()?;
			let pool_info = Pools::<T>::get(&pair).ok_or(Error::<T>::PoolNotFound).ok()?;

			let balance_1 = pool_info.reserve_a;
			let balance_2 = pool_info.reserve_b;
			
			if !balance_1.is_zero() {
				Self::quote(amount, balance_2, balance_1).ok()
			} else {
				None
			}
		}

		pub fn get_reserves(
			asset_a: AssetIdOf<T>,
			asset_b: AssetIdOf<T>,
		) -> Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
			let pair = AssetPair::<T>::new(asset_a, asset_b)?;
			let pool_info = Pools::<T>::get(&pair).ok_or(Error::<T>::PoolNotFound)?;

			Ok((pool_info.reserve_a, pool_info.reserve_b))
		}

		/// Get the account ID of the pallet, derived from the pallet's identifier.
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

		/// Calculate the pool fee for a given amount based on a fixed percentage (3%).
		pub fn pool_fee(amount: BalanceOf<T>) -> BalanceOf<T> {
			Percent::from_percent(T::PoolFeePercent::get()) * amount
		}
		
		/// Get the reducible balance of a specific asset owned by an account.
		pub fn balance(owner: T::AccountId, asset_id: AssetIdOf<T>) -> BalanceOf<T> {
			T::Fungibles::reducible_balance(asset_id, &owner, Expendable, Polite)
		}

		/// Check if an asset with the given identifier exists in the fungibles system.
		pub fn asset_exists(asset_id: &AssetIdOf<T>) -> bool {
			T::Fungibles::asset_exists(asset_id.clone())
		}

		/// Get the total issuance of a specific asset in the fungibles system.
		pub fn total_issuance(asset_id: AssetIdOf<T>) -> BalanceOf<T> {
			T::Fungibles::total_issuance(asset_id)
		}
	
		/// Get the reducible balance of a specific `asset` owned by the given `owner`.
		pub fn get_balance(owner: T::AccountId, asset: AssetIdOf<T>) -> BalanceOf<T> {
			T::Fungibles::reducible_balance(asset, &owner, Expendable, Polite)
		}

		/// Calculate the amount of the second asset (reserve_b) that would be received in exchange for a given amount of the first asset (reserve_a).
		/// Uses the provided amount, reserve_a, and reserve_b in the calculation.
		pub fn quote(
			amount: BalanceOf<T>,
			reserve_a: BalanceOf<T>,
			reserve_b: BalanceOf<T>
		) -> Result<BalanceOf<T>, Error<T>> {
			// Calculate the result using the formula: (amount * reserve_b) / reserve_a.
			Self::mul_div(amount, reserve_b, reserve_a)
		}

		fn mul_div(a: BalanceOf<T>, b: BalanceOf<T>, c: BalanceOf<T>) -> Result<BalanceOf<T>, Error<T>> {
			let result = a
				.checked_mul(&b)
				.ok_or(Error::<T>::Overflow)?
				.checked_div(&c)
				.ok_or(Error::<T>::Overflow)?;

			result.try_into().map_err(|_| Error::<T>::Overflow)
		}

		/// Calculate the amount of liquidity pool (LP) tokens to be minted when the total supply is zero.
		/// Uses the provided amount_a and amount_b in the calculation.
		pub fn get_lp_amount_for_zero_supply(
			amount_a: BalanceOf<T>,
			amount_b: BalanceOf<T>
		) -> Result<BalanceOf<T>, Error<T>> {
			// Calculate the result using the formula: sqrt(amount_a * amount_b) - MinLiquidity.
			let result = amount_a
				.checked_mul(&amount_b)
				.ok_or(Error::<T>::Overflow)?
				.integer_sqrt()
				.checked_sub(&T::MinLiquidity::get().into())
				.ok_or(Error::<T>::InsufficientLiquidity)?;
			
			// Try to convert the result to the balance type and handle any potential errors.
			result.try_into().map_err(|_| Error::<T>::Overflow)
		}
    }
}
