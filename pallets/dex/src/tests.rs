use crate::{mock::*, AssetPair, Error, Event, Pools};
use frame_support::{assert_noop, assert_ok};
use frame_support::traits::fungibles::InspectEnumerable;
use frame_support::traits::fungibles::Mutate;
use sp_runtime::{DispatchError, Percent, TokenError};



type NativeBalance = <Test as crate::Config>::NativeBalance;
type Fungibles = <Test as crate::Config>::Fungibles;




// Implementation for `Ord` and `PartialOrd` traits for `mock::Test`.
// This is necessary for certain structs where `Test` is used as a generic parameter,
// and those structs derive the `RuntimeDebugNoBound` trait.
// e.g. AssetPair<T: Config>
impl Ord for Test {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for Test {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Creates a new token for the specified owner with the given asset ID.
fn create_token(owner: u64, asset_id: u32) {
	assert_ok!(
		Fungibles::force_create(
			RuntimeOrigin::root(),
			asset_id.into(),
			owner,
			false,
			1u32.into()
		)
	);
}

/// Retrieves the free balance of the native asset for the specified account.
/// 
/// # Returns
/// The free balance of the native asset for the specified account.
fn get_native_balance(who: u64) -> u128 {
	NativeBalance::free_balance(who)
}

/// Retrieves the balance of a specific asset for the specified account.
/// 
/// # Returns
/// The balance of the specified asset for the specified account.
fn get_asset_balance(who: u64, asset_id: u32) -> u128 {
	Fungibles::balance(asset_id, who)
}

/// Retrieves a sorted list of all liquidity pool pairs (AssetPairs) available in the system.
/// 
/// # Returns
/// A sorted vector containing all available liquidity pool pairs (AssetPairs) in the system.
fn get_pools() -> Vec<AssetPair<Test>> {
	let mut s: Vec<_> = Pools::<Test>::iter().map(|x| x.0).collect();
	s.sort();
	s
}

/// Retrieves a sorted list of all asset IDs available in the system.
/// 
/// # Returns
/// A sorted vector containing all available asset IDs in the system.
fn get_assets() -> Vec<u32> {
	let mut s: Vec<_> = Fungibles::asset_ids().map(|id| id).collect();
	s.sort();
	s
}

/// Retrieves events related to the Dex runtime from the system's event log.
///
/// # Returns
/// A vector containing events related to the Dex runtime.
fn get_events() -> Vec<Event<Test>> {
	let result = System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| {
			if let RuntimeEvent::Dex(inner) = e {
				Some(inner)
			} else {
				None
			}
		})
		.collect();

	System::reset_events();

	result
}

/// Test to check behavior when using maximum values in the Dex module's `quote` function.
/// The test checks scenarios where one or more of the input parameters are set to their maximum value.
#[test]
fn check_max_numbers() {
	new_test_ext().execute_with(|| {
		assert!(Dex::quote(3u128, u128::MAX, u128::MAX).is_err()); // 3 * u128::MAX / u128::MAX
		assert!(Dex::quote(u128::MAX, 3u128, u128::MAX).is_err()); // u128::MAX * u128::MAX / 3
		assert_eq!(Dex::quote(u128::MAX, u128::MAX, 1u128).ok().unwrap(), 1u128); // u128::MAX * 1u128 / u128::MAX
	});
}


#[test]
fn create_pool_successfully() {
	new_test_ext().execute_with(|| {
		let creator = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;
		let pair = AssetPair::<Test>::new(asset_a, asset_b).expect("Can create pair");
 
		// Create asset_a
		create_token(creator, asset_a);
		// Create asset_b
		create_token(creator, asset_b);

		// Ser creator initial balance
		let initial_balance = 1_000u128;
		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				creator,
				initial_balance
			)
		);

		// Create liquidity pool with asset Id
		let lp_asset = 1234u32;
		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(creator),
				asset_a,
				asset_b, 
				lp_asset
			)
		);

		let fee = 100u128;
		assert_eq!(
			get_native_balance(creator),
			initial_balance - fee 
		);

		// Get pallet account balance
		let pallet_balance = get_native_balance(PalletAccountId::get());
		assert_eq!(pallet_balance, fee);

		// Check emited events
		assert_eq!(
			get_events(),
			[Event::<Test>::PoolCreated {
				creator,
				pair: pair.clone(),
				lp_asset
			}]
		);

		// Check pair added to the pool
		assert_eq!(get_pools(), vec![pair.clone()]);
		// Check lp_asset created 
		assert_eq!(get_assets(), vec![asset_a.clone(), asset_b.clone(), lp_asset.clone()]);

		assert_noop!(
			Dex::create_pool(
				RuntimeOrigin::signed(creator),
				asset_a,
				asset_a, 
				lp_asset
			),
			Error::<Test>::IdenticalAssets
		);

		assert_noop!(
			Dex::create_pool(
				RuntimeOrigin::signed(creator),
				asset_b,
				asset_b, 
				lp_asset
			),
			Error::<Test>::IdenticalAssets
		);

		let non_existent_asset = 999;
		assert_noop!(
			Dex::create_pool(
				RuntimeOrigin::signed(creator),
				non_existent_asset,
				asset_b, 
				lp_asset
			),
			Error::<Test>::AssetNotExists
		);

		assert_noop!(
			Dex::create_pool(
				RuntimeOrigin::signed(creator),
				asset_a,
				non_existent_asset, 
				lp_asset
			),
			Error::<Test>::AssetNotExists
		);

		assert_noop!(
			Dex::create_pool(
				RuntimeOrigin::signed(creator),
				asset_a,
				asset_b, 
				lp_asset
			),
			Error::<Test>::PoolExists
		);


		// Create asset_c
		let asset_c = 3u32;
		create_token(creator, asset_c);

		assert_noop!(
			Dex::create_pool(
				RuntimeOrigin::signed(creator),
				asset_a,
				asset_c, 
				lp_asset
			),
			Error::<Test>::AssetIdAreadyTaken
		);

		// Can we create a pool if one of the token not native?
	});
}

#[test]
fn create_pool_twice_failure() {
	new_test_ext().execute_with(|| {
		let creator = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;
		let pair = AssetPair::<Test>::new(asset_a, asset_b).expect("Can create pair");
 
		// Create asset_a
		create_token(creator, asset_a);
		// Create asset_b
		create_token(creator, asset_b);

		// Ser creator initial balance
		let initial_balance = 1_000u128;
		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				creator,
				initial_balance
			)
		);

		// Create liquidity pool with asset Id
		let lp_asset = 1234u32;
		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(creator),
				asset_a,
				asset_b, 
				lp_asset
			)
		);

		let fee = 100u128;
		assert_eq!(
			get_native_balance(creator),
			initial_balance - fee 
		);

		// Get pallet account balance
		let pallet_balance = get_native_balance(PalletAccountId::get());
		assert_eq!(pallet_balance, fee);

		// Check emited events
		assert_eq!(
			get_events(),
			[Event::<Test>::PoolCreated {
				creator,
				pair: pair.clone(),
				lp_asset
			}]
		);

		assert_noop!(
			Dex::create_pool(
				RuntimeOrigin::signed(creator),
				asset_a,
				asset_b, 
				lp_asset
			),
			Error::<Test>::PoolExists
		);

		// Check pair added to the pool
		assert_eq!(get_pools(), vec![pair.clone()]);
		// Check lp_asset created 
		assert_eq!(get_assets(), vec![asset_a.clone(), asset_b.clone(), lp_asset.clone()]);

		// Can we create a pool if one of the token not native?
	});
}

#[test]
fn different_pools_should_have_different_lp_assets() {
	new_test_ext().execute_with(|| {
		let creator = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;
		let asset_c = 3u32;
		let pair_1 = AssetPair::<Test>::new(asset_a, asset_b).expect("Can create pair");

		let pair_2 = AssetPair::<Test>::new(asset_a, asset_c).expect("Can create pair");
 
		// Create asset_a
		create_token(creator, asset_a);
		// Create asset_b
		create_token(creator, asset_b);
		// Create asset_c
		create_token(creator, asset_c);

		// Ser creator initial balance
		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				creator,
				1_000u128
			)
		);

		// Create liquidity pool with asset Id
		let lp_asset_a = 1234u32;
		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(creator),
				asset_a,
				asset_b, 
				lp_asset_a
			)
		);

		let lp_asset_b = 4321u32;
		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(creator),
				asset_a,
				asset_c, 
				lp_asset_b
			)
		);

		// Check emited events
		assert_eq!(
			get_events(),
			[
				Event::<Test>::PoolCreated {
					creator,
					pair: pair_1.clone(),
					lp_asset: lp_asset_a
				},
				Event::<Test>::PoolCreated {
					creator,
					pair: pair_2.clone(),
					lp_asset: lp_asset_b
				}
			]
		);

		// Can we create a pool if one of the token not native?
	});
}


#[test]
fn add_liqudity_sucessfully() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;
		let asset_c = 3u32;

		let lp_asset_a = 1234u32;
		let lp_asset_b = 4321u32;

		// Create asset_a
		create_token(sender, asset_a);
		// Create asset_b
		create_token(sender, asset_b);
		// Create asset_c
		create_token(sender, asset_c);

		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender,
				1_000_000u128
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				asset_a,
				asset_b, 
				lp_asset_a
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				asset_a,
				asset_c, 
				lp_asset_b
			)
		);

		let initial_balance = 100_000u128;
		let min_balance = MinBalance::get();
		assert_ok!(Fungibles::mint_into(asset_a.into(), &sender, initial_balance + min_balance)); 
		assert_ok!(Fungibles::mint_into(asset_b.into(), &sender, initial_balance + min_balance)); 

		assert_ok!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				10_000u128,
				10u128,
				10_000u128,
				10u128,
				sender,
			)
		);

		let pair_1 = AssetPair::<Test>::new(asset_a, asset_b).expect("Can create pair");

		assert!(
			get_events().contains(
				&Event::<Test>::LiquidityAdded {
					sender,
					pair: pair_1,
					lp_asset: lp_asset_a,
					liqudity_provided_a: 10_000u128,
					liqudity_provided_b: 10u128,
					minted: 306u128,
				}
			)
		);
	

		let pallet_account_id = PalletAccountId::get();
		assert_eq!(get_asset_balance(pallet_account_id, asset_a.clone()), 10_000u128);
		assert_eq!(get_asset_balance(pallet_account_id, asset_b.clone()), 10u128);
		assert_eq!(get_asset_balance(sender, asset_a.clone()), 90000u128 + 10u128);
		assert_eq!(get_asset_balance(sender, asset_b.clone()), 100_000u128);
		assert_eq!(get_asset_balance(sender, lp_asset_a), 306u128);
	});
}

#[test]
fn add_tiny_liquidity_leads_to_insufficient_liquidity_error() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;
		let lp_asset_a = 1234u32;

		// Create asset_a
		create_token(sender, asset_a);
		// Create asset_b
		create_token(sender, asset_b);

		// Ser sender initial balance
		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender,
				1_000_000u128
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				asset_a,
				asset_b, 
				lp_asset_a
			)
		);

		let initial_balance = 100_000u128;
		let min_balance = MinBalance::get();
		assert_ok!(Fungibles::mint_into(asset_a.into(), &sender, initial_balance + min_balance)); 
		assert_ok!(Fungibles::mint_into(asset_b.into(), &sender, initial_balance + min_balance)); 

		assert_noop!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				0,
				1,
				1,
				1,
				sender,
			),
			Error::<Test>::InvalidDesiredAmount
		);

		assert_noop!(
			Dex::add_liquidity( 
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				1,
				1,
				1,
				1,
				sender,
			),
			Error::<Test>::InsufficientLiquidity
		);
	});
}

#[test]
fn add_tiny_liquidity_directly_to_pool_address() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;
		let asset_c = 3u32;
		let lp_asset_a = 1234u32;
		let lp_asset_b = 4321u32;

		// Create asset_a
		create_token(sender, asset_a);
		// Create asset_b
		create_token(sender, asset_b);
		// Create asset_c
		create_token(sender, asset_c);

		// Ser sender initial balance
		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender,
				1_000_000u128
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				asset_a,
				asset_b, 
				lp_asset_a
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				asset_a,
				asset_c, 
				lp_asset_b
			)
		);

		let initial_balance = 100_000u128;
		let min_balance = MinBalance::get();
		assert_ok!(Fungibles::mint_into(asset_a.into(), &sender, initial_balance + min_balance)); 
		assert_ok!(Fungibles::mint_into(asset_b.into(), &sender, initial_balance + min_balance)); 
		assert_ok!(Fungibles::mint_into(asset_c.into(), &sender, initial_balance + min_balance)); 


		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				PalletAccountId::get(),
				initial_balance
			)
		);
		
		assert_ok!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				10000,
				10,
				10000,
				10,
				sender,
			)
		);

		assert_ok!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_c.clone(),
				10000,
				10,
				10000,
				10,
				sender,
			)
		);
	});
} 

#[test]
fn remove_liquidity_sucessfully() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;

		let lp_asset_a = 1234u32;

		// Create asset_a
		create_token(sender, asset_a);
		// Create asset_b
		create_token(sender, asset_b);

		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender,
				1_000_000u128
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				asset_a,
				asset_b, 
				lp_asset_a
			)
		);

		let initial_balance = 100_000u128;
		let min_balance = MinBalance::get();
		assert_ok!(Fungibles::mint_into(asset_a.into(), &sender, initial_balance + min_balance)); 
		assert_ok!(Fungibles::mint_into(asset_b.into(), &sender, initial_balance + min_balance));

		let amount_a_desired = 100_000u128;
		let amount_b_desired = 10_000u128;
		let amount_a_min = 100_000u128;
		let amount_b_min = 10_000u128;

		assert_ok!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				amount_a_desired,
				amount_b_desired,
				amount_a_min,
				amount_b_min,
				sender,
			)
		);

		// Pool balances after add liquidity
		let pallet_account_id = PalletAccountId::get();
		assert_eq!(get_asset_balance(pallet_account_id, asset_a.clone()), 100_000u128);
		assert_eq!(get_asset_balance(pallet_account_id, asset_b.clone()), 10_000u128);
		assert_eq!(get_asset_balance(pallet_account_id, lp_asset_a.clone()), 10);

		let sender_lp_asset_balance = get_asset_balance(sender, lp_asset_a.clone());

		assert_ok!(
			Dex::remove_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				sender_lp_asset_balance,
				0,
				0,
			)
		);

		let pair_1 = AssetPair::<Test>::new(asset_a, asset_b).expect("Can create pair");

		let liqudity_removed_a = 98969u128;
		let liqudity_removed_b = 9896u128;

		let removal_fee = Dex::pool_fee(sender_lp_asset_balance);

		assert!(
			get_events().contains(
				&Event::<Test>::LiquidityRemoved {
					sender,
					pair: pair_1,
					lp_asset: lp_asset_a,
					liqudity_removed_a,
					liqudity_removed_b,
					removal_fee: removal_fee,
					burned: sender_lp_asset_balance
				}
			)
		);
		
		// Pool balances after add liquidity
		let pallet_account_id = PalletAccountId::get();
		assert_eq!(get_asset_balance(pallet_account_id, asset_a.clone()), 1031);
		assert_eq!(get_asset_balance(pallet_account_id, asset_b.clone()), 104);
		assert_eq!(get_asset_balance(pallet_account_id, lp_asset_a.clone()), 10);

		assert_eq!(
			get_asset_balance(sender, asset_a.clone()),
			initial_balance - amount_a_desired + liqudity_removed_a + min_balance
		);
		assert_eq!(
			get_asset_balance(sender, asset_b.clone()),
			initial_balance - amount_b_desired + liqudity_removed_b + min_balance
		);

		// All minted amount for sender removed
		assert_eq!(get_asset_balance(sender, lp_asset_a), 0);
	});
}

#[test]
fn can_not_redeem_more_lp_tokens_than_were_minted() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;

		let lp_asset_a = 1234u32;

		// Create asset_a
		create_token(sender, asset_a);
		// Create asset_b
		create_token(sender, asset_b);
		
		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender,
				10_000u128
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				asset_a,
				asset_b, 
				lp_asset_a
			)
		);

		let initial_balance = 10_000u128;
		let min_balance = MinBalance::get();
		assert_ok!(Fungibles::mint_into(asset_a.into(), &sender, initial_balance + min_balance)); 
		assert_ok!(Fungibles::mint_into(asset_b.into(), &sender, initial_balance + min_balance));

		let amount_a_desired = 1_000u128;
		let amount_b_desired = 100u128;
		let amount_a_min = 1_000u128;
		let amount_b_min = 100u128;

		assert_ok!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				amount_a_desired,
				amount_b_desired,
				amount_a_min,
				amount_b_min,
				sender,
			)
		);

		// Pool balances after add liquidity
		let minted_lp_amount = 306u128;
		assert_eq!(get_asset_balance(sender, lp_asset_a.clone()), minted_lp_amount);

		assert_noop!(
			Dex::remove_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				minted_lp_amount + 1,
				0,
				0,
			),
			DispatchError::Token(TokenError::FundsUnavailable)
		);
	});
}

#[test]
fn quote_prices() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;

		let lp_asset_a = 1234u32;

		// Create asset_a
		create_token(sender, asset_a);
		// Create asset_b
		create_token(sender, asset_b);

		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender,
				10_000u128
			)
		);

		let min_balance = MinBalance::get();
		assert_ok!(Fungibles::mint_into(asset_a.into(), &sender, 100_000u128 + min_balance)); 
		assert_ok!(Fungibles::mint_into(asset_b.into(), &sender, 1_000u128 + min_balance));

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				asset_a,
				asset_b, 
				lp_asset_a
			)
		);

		let amount_a_desired = 10_000u128;
		let amount_b_desired = 200u128;
		let amount_a_min = 1u128;
		let amount_b_min = 1u128;

		assert_ok!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				amount_a_desired,
				amount_b_desired,
				amount_a_min,
				amount_b_min,
				sender,
			)
		);

		assert_eq!(
			Dex::calc_price_exact_asset_for_asset(
				asset_a,
				asset_b,
				3000,
			),
			Some(60)
		);
		
		assert_eq!(
			Dex::calc_price_exact_asset_for_asset(
				asset_b,
				asset_a,
				3000,
			),
			Some(60)
		);
		
		assert_eq!(
			Dex::calc_price_asset_for_exact_asset(
				asset_a,
				asset_b,
				60,
			),
			Some(3000)
		);

		assert_eq!(
			Dex::calc_price_asset_for_exact_asset(
				asset_b,
				asset_a,
				60,
			),
			Some(3000)
		);

        let amount_in = 100;

        assert_eq!(
            Dex::calc_price_exact_asset_for_asset(
                asset_a,
				asset_b,
                amount_in,
            )
            .and_then(|amount| Dex::calc_price_asset_for_exact_asset(
                asset_a,
                asset_b,
                amount,
            )),
            Some(amount_in)
        );

        assert_eq!(
            Dex::calc_price_exact_asset_for_asset(
                asset_a,
                asset_b,
                amount_in,
            )
            .and_then(|amount| Dex::calc_price_asset_for_exact_asset(
                asset_b,
                asset_a,
                amount,
            )),
            Some(amount_in)
        );

        assert_eq!(
            Dex::calc_price_asset_for_exact_asset(
                asset_b,
                asset_a,
                amount_in,
            )
            .and_then(|amount| Dex::calc_price_exact_asset_for_asset(
                asset_a,
                asset_b,
                amount,
            )),
            Some(amount_in)
        );

        assert_eq!(
            Dex::calc_price_asset_for_exact_asset(
                asset_a,
                asset_b,
                amount_in,
            )
            .and_then(|amount| Dex::calc_price_exact_asset_for_asset(
                asset_b,
                asset_a,
                amount,
            )),
            Some(amount_in)
        );
	});
}

#[test]
fn swap_successfully() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;

		let lp_asset_a = 1234u32;

		// Create asset_a
		create_token(sender, asset_a);
		// Create asset_b
		create_token(sender, asset_b);

		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender,
				10_000u128
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				asset_a,
				asset_b, 
				lp_asset_a
			)
		);

		let min_balance = MinBalance::get();
		assert_ok!(Fungibles::mint_into(asset_a.into(), &sender, 10_000u128 + min_balance)); 
		assert_ok!(Fungibles::mint_into(asset_b.into(), &sender, 1_000u128 + min_balance));

		let liquidity_1 = 10_000u128;
		let liquidity_2 = 200u128;
		let amount_a_min = 1u128;
		let amount_b_min = 1u128;

		assert_ok!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				liquidity_1,
				liquidity_2,
				amount_a_min,
				amount_b_min,
				sender,
			)
		);

		let amount_in = 100u128;
		let expect_receive =
			Dex::get_amount_out(amount_in, liquidity_2, liquidity_1)
				.ok()
				.unwrap();

		assert_ok!(
			Dex::swap_exact_asset_for_asset(
				RuntimeOrigin::signed(sender),
				asset_b,
				asset_a,
				amount_in,
				1,
			)
		);
		
		let pallet_account_id = PalletAccountId::get();
		assert_eq!(get_asset_balance(sender, asset_a.clone()), expect_receive + min_balance);
		assert_eq!(get_asset_balance(sender, asset_b.clone()), 1_000u128 - liquidity_2 - amount_in  + min_balance);
		assert_eq!(get_asset_balance(pallet_account_id, asset_a.clone()), liquidity_1 - expect_receive);
		assert_eq!(get_asset_balance(pallet_account_id, asset_b.clone()), liquidity_2 + amount_in);
	});
}

#[test]
fn swappable_with_real_values() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let dot = 1u32;
		let usdt = 2u32;

		let lp_asset_a = 1234u32;

		// Create asset_a
		create_token(sender, dot);
		// Create asset_b
		create_token(sender, usdt);

		
		let initial_balance = 10_000u128;
		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender,
				initial_balance
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				dot,
				usdt, 
				lp_asset_a
			)
		);

		const UNIT: u128 = 1_000_000_000;
		let min_balance = MinBalance::get();
		assert_ok!(Fungibles::mint_into(dot.into(), &sender, 300_000u128 * UNIT + min_balance)); // dot
		assert_ok!(Fungibles::mint_into(usdt.into(), &sender, 1_100_000u128 * UNIT + min_balance)); // usdt

		let liquidity_dot = 200_000u128 * UNIT; // 1 dot for 5 usdt price
		let liquidity_usdt = 1_000_000u128 * UNIT;
		let amount_a_min = 1u128;
		let amount_b_min = 1u128;

		assert_ok!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender),
				dot.clone(),
				usdt.clone(),
				liquidity_dot,
				liquidity_usdt,
				amount_a_min,
				amount_b_min,
				sender,
			)
		);
		
		let amount_in = 10u128 * UNIT; // usdt

		assert_ok!(
			Dex::swap_exact_asset_for_asset(
				RuntimeOrigin::signed(sender),
				usdt,
				dot,
				amount_in,
				1,
			)
		);
		
		assert!(
			get_events().contains(
				&Event::<Test>::Swaped {
					sender, 
					asset_in: usdt, 
					amount_in,
					asset_out: dot, 
					amount_out: 1_979_980_398 // around 2 dot  
				}
			)
		);
	});
}

#[test]
fn disallow_swap_in_empty_liquidity_pool() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;

		// Create asset_a
		create_token(sender, asset_a);
		// Create asset_b
		create_token(sender, asset_b);

		let amount_in = 10u128;

		assert_noop!(
			Dex::swap_exact_asset_for_asset(
				RuntimeOrigin::signed(sender),
				asset_b,
				asset_a,
				amount_in,
				1,
			),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn prevent_swap_on_high_slippage() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;

		let lp_asset = 1234u32;

		// Create asset_a
		create_token(sender, asset_a);
		// Create asset_b
		create_token(sender, asset_b);

		let initial_balance = 10_000u128;
		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender,
				initial_balance
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				asset_a,
				asset_b, 
				lp_asset
			)
		);

		let min_balance = MinBalance::get();
		assert_ok!(Fungibles::mint_into(asset_a.into(), &sender, 10_000u128 + min_balance));
		assert_ok!(Fungibles::mint_into(asset_b.into(), &sender, 1_000u128));

		let liquidity_1 = 10_000u128;
		let liquidity_2 = 200u128;
		let amount_a_min = 1u128;
		let amount_b_min = 1u128;

		assert_ok!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				liquidity_1,
				liquidity_2,
				amount_a_min,
				amount_b_min,
				sender,
			)
		);

		let amount_in = 100;

		assert_noop!(
			Dex::swap_exact_asset_for_asset(
				RuntimeOrigin::signed(sender),
				asset_b,
				asset_a,
				amount_in,
				4000,
			),
			Error::<Test>::ProvidedMinimumNotSufficientForSwap
		);
	});
}

#[test]
fn execute_exact_token_swap_as_non_liquidity_provider() {
	new_test_ext().execute_with(|| {
		let sender_1 = 1;
		let sender_2 = 2;
		let asset_a = 1u32;
		let asset_b = 2u32;

		let lp_asset = 1234u32;

		// Create asset_a
		create_token(sender_2, asset_a);
		// Create asset_b
		create_token(sender_2, asset_b);

		let initial_balance = 10_000u128;
		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender_1,
				initial_balance
			)
		);

		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender_2,
				initial_balance
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender_1),
				asset_a,
				asset_b, 
				lp_asset
			)
		);

		let min_balance = MinBalance::get();
		assert_ok!(Fungibles::mint_into(asset_a.into(), &sender_1, 10_000u128 + min_balance));
		assert_ok!(Fungibles::mint_into(asset_a.into(), &sender_2, 10_000u128 + min_balance));
		assert_ok!(Fungibles::mint_into(asset_b.into(), &sender_2, 1_000u128));
	

		let asset_a_balances_before = 
			get_asset_balance(sender_1, asset_a.clone()) +
			get_asset_balance(sender_2, asset_a.clone()) +
			get_asset_balance(PalletAccountId::get(), asset_a.clone());
		
		let asset_b_balances_before = 
			get_asset_balance(sender_1, asset_b.clone()) +
			get_asset_balance(sender_2, asset_b.clone()) +
			get_asset_balance(PalletAccountId::get(), asset_b.clone());

		let liquidity_1 = 10_000u128;
		let liquidity_2 = 200u128;
		
		assert_ok!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender_2),
				asset_a.clone(),
				asset_b.clone(),
				liquidity_1,
				liquidity_2,
				1,
				1,
				sender_2,
			)
		);
		
		assert_eq!(get_asset_balance(sender_1, asset_a.clone()), 10_000u128 + min_balance);
		assert_eq!(get_asset_balance(sender_1, asset_b.clone()), 0);

		let exchange_out = 50;
		let expect_in = Dex::get_amount_in(exchange_out, liquidity_1, liquidity_2)
			.ok()
			.unwrap();

		assert_ok!(
			Dex::swap_asset_for_exact_asset(
				RuntimeOrigin::signed(sender_1),
				asset_a,
				asset_b,
				exchange_out,
				3500,
			)
		);

		assert_eq!(get_asset_balance(sender_1, asset_a.clone()), 10_000u128 + min_balance - expect_in);
		assert_eq!(get_asset_balance(PalletAccountId::get(), asset_a.clone()), liquidity_1 + expect_in);
		assert_eq!(get_asset_balance(sender_1, asset_b.clone()), exchange_out);
		assert_eq!(get_asset_balance(PalletAccountId::get(), asset_b.clone()), liquidity_2 - exchange_out);

		assert_eq!(
			asset_a_balances_before, 
			get_asset_balance(sender_1, asset_a.clone()) + 
			get_asset_balance(sender_2, asset_a.clone()) +
			get_asset_balance(PalletAccountId::get(), asset_a.clone())
		);

		assert_eq!(
			asset_b_balances_before,
			get_asset_balance(sender_1, asset_b.clone()) +
			get_asset_balance(sender_2, asset_b.clone()) +
			get_asset_balance(PalletAccountId::get(), asset_b.clone())
		);
	});
}

#[test]
fn swap_of_same_asset_not_allowed() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let asset_a = 1u32;
		let asset_b = 2u32;

		let lp_asset = 1234u32;

		// Create asset_a
		create_token(sender, asset_a);
		// Create asset_b
		create_token(sender, asset_b);

		let initial_balance = 10_000u128;
		assert_ok!(
			NativeBalance::force_set_balance(
				RuntimeOrigin::root(),
				sender,
				initial_balance
			)
		);

		assert_ok!(
			Dex::create_pool(
				RuntimeOrigin::signed(sender),
				asset_a,
				asset_b, 
				lp_asset
			)
		);

		let min_balance = MinBalance::get();
		assert_ok!(Fungibles::mint_into(asset_a.into(), &sender, 10_000u128 + min_balance));
		assert_ok!(Fungibles::mint_into(asset_b.into(), &sender, 1_000u128));


		let liquidity_1 = 1000;
		let liquidity_2 = 20;
		assert_ok!(
			Dex::add_liquidity(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_b.clone(),
				liquidity_1,
				liquidity_2,
				1,
				1,
				sender
			)
		);

		let amount_in = 10u128;
		assert_noop!(
			Dex::swap_exact_asset_for_asset(
				RuntimeOrigin::signed(sender),
				asset_a.clone(),
				asset_a.clone(),
				amount_in,
				1,
			),
			Error::<Test>::IdenticalAssets
		);

		assert_noop!(
			Dex::swap_exact_asset_for_asset(
				RuntimeOrigin::signed(sender),
				asset_b.clone(),
				asset_b.clone(),
				amount_in,
				1,
			),
			Error::<Test>::IdenticalAssets
		);
	});
}
