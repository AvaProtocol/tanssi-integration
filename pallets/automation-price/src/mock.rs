// This file is part of OAK Blockchain.

// Copyright (C) 2022 OAK Network
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use crate as pallet_automation_price;
use crate::TaskId;

use frame_support::{
	assert_ok, construct_runtime, parameter_types,
	traits::{ConstU32, Everything},
	weights::Weight,
	PalletId,
};
use frame_system::{self as system, RawOrigin};
use orml_traits::parameter_type_with_key;
use ava_protocol_primitives::EnsureProxy;
use sp_core::H256;
use sp_runtime::{
	traits::{AccountIdConversion, BlakeTwo256, Convert, IdentityLookup},
	AccountId32, Perbill, BuildStorage,
};
use sp_std::{marker::PhantomData, vec::Vec};
use staging_xcm::latest::{prelude::*, Junctions::*};

type Block = system::mocking::MockBlock<Test>;

use crate::weights::WeightInfo;

pub type Balance = u128;
pub type AccountId = AccountId32;
pub type CurrencyId = u32;

pub const ALICE: [u8; 32] = [1u8; 32];
pub const BOB: [u8; 32] = [2u8; 32];
pub const DELEGATOR_ACCOUNT: [u8; 32] = [3u8; 32];
pub const PROXY_ACCOUNT: [u8; 32] = [4u8; 32];

pub const PARA_ID: u32 = 2000;
pub const NATIVE: CurrencyId = 0;
pub const NATIVE_LOCATION: Location = Location { parents: 0, interior: Here };
pub const NATIVE_EXECUTION_WEIGHT_FEE: u128 = 12;
pub const FOREIGN_CURRENCY_ID: CurrencyId = 1;

pub fn get_moonbase_asset_location() -> Location {
	Location { parents: 1, interior: X2([Parachain(1000u32), PalletInstance(3u8)].into()) }
}

pub const EXCHANGE1: &[u8] = "EXCHANGE1".as_bytes();

pub const CHAIN1: &[u8] = "KUSAMA".as_bytes();
pub const CHAIN2: &[u8] = "DOT".as_bytes();

pub const ASSET1: &[u8] = "TUR".as_bytes();
pub const ASSET2: &[u8] = "USDC".as_bytes();
pub const ASSET3: &[u8] = "KSM".as_bytes();
pub const MOCK_XCMP_FEE: u128 = 10_000_000_u128;

construct_runtime!(
	pub enum Test
	{
		System: system,
		Timestamp: pallet_timestamp,
		Balances: pallet_balances,
		ParachainInfo: parachain_info,
		Tokens: orml_tokens,
		Currencies: orml_currencies,
		AutomationPrice: pallet_automation_price,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 51;
}

// impl system::Config for Test {
// 	type BaseCallFilter = Everything;
// 	type BlockWeights = ();
// 	type BlockLength = ();
// 	type DbWeight = ();
// 	type RuntimeOrigin = RuntimeOrigin;
// 	type RuntimeCall = RuntimeCall;
// 	type Index = u64;
// 	type BlockNumber = u64;
// 	type Hash = H256;
// 	type Hashing = BlakeTwo256;
// 	type AccountId = AccountId32;
// 	type Lookup = IdentityLookup<Self::AccountId>;
// 	type Header = Header;
// 	type RuntimeEvent = RuntimeEvent;
// 	//type RuntimeEvent = From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
// 	type BlockHashCount = BlockHashCount;
// 	type Version = ();
// 	type PalletInfo = PalletInfo;
// 	type AccountData = pallet_balances::AccountData<Balance>;
// 	type OnNewAccount = ();
// 	type OnKilledAccount = ();
// 	type SystemWeightInfo = ();
// 	type SS58Prefix = SS58Prefix;
// 	type OnSetCode = ();
// 	type MaxConsumers = frame_support::traits::ConstU32<16>;
// }

impl system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Block = Block;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type RuntimeTask = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}
impl pallet_balances::Config for Test {
	type MaxLocks = MaxLocks;
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type WeightInfo = ();
}

impl parachain_info::Config for Test {}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}
parameter_types! {
	pub DustAccount: AccountId = PalletId(*b"auto/dst").into_account_truncating();
}

impl orml_tokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = i64;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = ();
	type MaxLocks = ConstU32<100_000>;
	type MaxReserves = ConstU32<100_000>;
	type ReserveIdentifier = [u8; 8];
	type DustRemovalWhitelist = frame_support::traits::Nothing;
}

impl orml_currencies::Config for Test {
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}
pub type AdaptedBasicCurrency = orml_currencies::BasicCurrencyAdapter<Test, Balances, i64, u64>;

parameter_types! {
	pub const MinimumPeriod: u64 = 1000;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

impl pallet_automation_price::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MaxTasksPerSlot = MaxTasksPerSlot;
	type MaxTasksPerAccount = MaxTasksPerAccount;
	type MaxTasksOverall = MaxTasksOverall;
	type MaxBlockWeight = MaxBlockWeight;
	type MaxWeightPercentage = MaxWeightPercentage;
	type WeightInfo = MockWeight<Test>;
	type ExecutionWeightFee = ExecutionWeightFee;
	type CurrencyId = CurrencyId;
	type MultiCurrency = Currencies;
	type Currency = Balances;
	type CurrencyIdConvert = MockTokenIdConvert;
	type FeeHandler = FeeHandler<Test, ()>;
	type FeeConversionRateProvider = MockConversionRateProvider;
	type UniversalLocation = UniversalLocation;
	type SelfParaId = parachain_info::Pallet<Test>;
	type XcmpTransactor = MockXcmpTransactor<Test, Balances>;

	type EnsureProxy = MockEnsureProxy;
}

parameter_types! {
	pub const MaxTasksPerSlot: u32 = 2;
	// Mock value, purposely set to a small number so easiser to test limit reached
	pub const MaxTasksOverall: u32 = 1024;
	pub const MaxTasksPerAccount: u32 = 16;
	#[derive(Debug)]
	pub const MaxScheduleSeconds: u64 = 24 * 60 * 60;
	pub const MaxBlockWeight: u64 = 20_000_000;
	pub const MaxWeightPercentage: Perbill = Perbill::from_percent(40);
	pub const ExecutionWeightFee: Balance = NATIVE_EXECUTION_WEIGHT_FEE;

	// When unit testing dynamic dispatch, we use the real weight value of the extrinsics call
	// This is an external lib that we don't own so we try to not mock, follow the rule don't mock
	// what you don't own
	// One of test we do is Balances::transfer call, which has its weight define here:
	// https://github.com/paritytech/polkadot-sdk/blob/polkadot-v0.9.38/frame/balances/src/weights.rs#L61-L73
	// When logging the final calculated amount, its value is 73_314_000.
	//
	// in our unit test, we test a few transfers with dynamic dispatch. On top
	// of that, there is also weight of our call such as fetching the tasks,
	// move from schedule slot to tasks queue,.. so the weight of a schedule
	// transfer with dynamic dispatch is even higher.
	//
	// and because we test run a few of them so I set it to ~10x value of 73_314_000
	pub const MaxWeightPerSlot: u128 = 700_000_000;
	pub const XmpFee: u128 = 1_000_000;
	pub const GetNativeCurrencyId: CurrencyId = NATIVE;
}

pub struct MockWeight<T>(PhantomData<T>);
impl<Test: frame_system::Config> pallet_automation_price::WeightInfo for MockWeight<Test> {
	fn emit_event() -> Weight {
		Weight::from_parts(20_000_000_u64, 0u64)
	}

	fn asset_price_update_extrinsic(v: u32) -> Weight {
		Weight::from_parts(220_000_000_u64 * v as u64, 0u64)
	}

	fn initialize_asset_extrinsic(_v: u32) -> Weight {
		Weight::from_parts(220_000_000_u64, 0u64)
	}

	fn schedule_xcmp_task_extrinsic() -> Weight {
		Weight::from_parts(24_000_000_u64, 0u64)
	}

	fn cancel_task_extrinsic() -> Weight {
		Weight::from_parts(20_000_000_u64, 0u64)
	}

	fn run_xcmp_task() -> Weight {
		Weight::from_parts(200_000_000_u64, 0u64)
	}

	fn remove_task() -> Weight {
		Weight::from_parts(20_000_000_u64, 0u64)
	}
}

pub struct MockXcmpTransactor<T, C>(PhantomData<(T, C)>);
impl<T, C> pallet_xcmp_handler::XcmpTransactor<T::AccountId, CurrencyId>
	for MockXcmpTransactor<T, C>
where
	T: Config + pallet::Config<Currency = C>,
	C: frame_support::traits::ReservableCurrency<T::AccountId>,
{
	fn transact_xcm(
		_destination: Location,
		_location: staging_xcm::latest::Location,
		_fee: u128,
		_caller: T::AccountId,
		_transact_encoded_call: sp_std::vec::Vec<u8>,
		_transact_encoded_call_weight: Weight,
		_overall_weight: Weight,
		_flow: InstructionSequence,
	) -> Result<(), sp_runtime::DispatchError> {
		Ok(())
	}

	fn pay_xcm_fee(
		_: CurrencyId,
		_: T::AccountId,
		_: u128,
	) -> Result<(), sp_runtime::DispatchError> {
		Ok(())
	}
}

pub struct MockConversionRateProvider;
impl FixedConversionRateProvider for MockConversionRateProvider {
	fn get_fee_per_second(location: &Location) -> Option<u128> {
		get_fee_per_second(location)
	}
}

pub struct MockTokenIdConvert;
impl Convert<CurrencyId, Option<Location>> for MockTokenIdConvert {
	fn convert(id: CurrencyId) -> Option<Location> {
		if id == NATIVE {
			Some(Location::new(0, Here))
		} else if id == FOREIGN_CURRENCY_ID {
			Some(Location::new(1, Parachain(PARA_ID)))
		} else {
			None
		}
	}
}

impl Convert<Location, Option<CurrencyId>> for MockTokenIdConvert {
	fn convert(location: Location) -> Option<CurrencyId> {
		if location == Location::new(0, Here) {
			Some(NATIVE)
		} else if location == Location::new(1, Parachain(PARA_ID)) {
			Some(FOREIGN_CURRENCY_ID)
		} else {
			None
		}
	}
}

// TODO: We should extract this and share code with automation-time
pub struct MockEnsureProxy;
impl EnsureProxy<AccountId> for MockEnsureProxy {
	fn ensure_ok(_delegator: AccountId, _delegatee: AccountId) -> Result<(), &'static str> {
		if _delegator == DELEGATOR_ACCOUNT.into() && _delegatee == PROXY_ACCOUNT.into() {
			Ok(())
		} else {
			Err("proxy error: expected `ProxyType::Any`")
		}
	}
}

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Rococo;
	// The universal location within the global consensus system
	pub UniversalLocation: InteriorLocation = X2([GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into())].into());
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(state_block_time: u64) -> sp_io::TestExternalities {
	let genesis_storage = system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(genesis_storage);
	ext.execute_with(|| System::set_block_number(1));
	ext.execute_with(|| Timestamp::set_timestamp(state_block_time));
	ext
}

pub fn events() -> Vec<RuntimeEvent> {
	let events = System::events();
	let evt = events.into_iter().map(|evt| evt.event).collect::<Vec<_>>();

	System::reset_events();

	evt
}

// A utility test function to pluck out the task id from events, useful when dealing with multiple
// task scheduling
pub fn get_task_ids_from_events() -> Vec<TaskId> {
	System::events()
		.into_iter()
		.filter_map(|e| match e.event {
			RuntimeEvent::AutomationPrice(crate::Event::TaskScheduled { task_id, .. }) => {
				Some(task_id)
			},
			_ => None,
		})
		.collect::<Vec<_>>()
}

pub fn get_xcmp_funds(account: AccountId) {
	let double_action_weight = MockWeight::<Test>::run_xcmp_task() * 2;
	let action_fee = ExecutionWeightFee::get() * u128::from(double_action_weight.ref_time());
	let max_execution_fee = action_fee * u128::from(1u32);
	let with_xcm_fees = max_execution_fee + XmpFee::get();
	Balances::force_set_balance(RawOrigin::Root.into(), account, with_xcm_fees).unwrap();
}

#[derive(Clone)]
pub struct MockAssetFeePerSecond {
	pub asset_location: Location,
	pub fee_per_second: u128,
}

pub fn get_asset_fee_per_second_config() -> Vec<MockAssetFeePerSecond> {
	let asset_fee_per_second: [MockAssetFeePerSecond; 3] = [
		MockAssetFeePerSecond {
			asset_location: Location { parents: 1, interior: Parachain(2000).into() },
			fee_per_second: 416_000_000_000,
		},
		MockAssetFeePerSecond {
			asset_location: Location {
				parents: 1,
				interior: X2([Parachain(2110), GeneralKey { length: 4, data: [0; 32] }].into()),
			},
			fee_per_second: 416_000_000_000,
		},
		MockAssetFeePerSecond {
			asset_location: get_moonbase_asset_location(),
			fee_per_second: 10_000_000_000_000_000_000,
		},
	];
	asset_fee_per_second.to_vec()
}

pub fn get_fee_per_second(location: &Location) -> Option<u128> {
	let location = location.clone()
		.reanchored(
			&Location::new(1, Parachain(<Test as Config>::SelfParaId::get().into())),
			&<Test as Config>::UniversalLocation::get(),
		)
		.expect("Reanchor location failed");

	let found_asset = get_asset_fee_per_second_config().into_iter().find(|item| match item {
		MockAssetFeePerSecond { asset_location, .. } => *asset_location == location,
	});

	if found_asset.is_some() {
		Some(found_asset.unwrap().fee_per_second)
	} else {
		None
	}
}

// setup a sample default asset to support test
pub fn setup_asset(sender: &AccountId32, chain: Vec<u8>) {
	let _ = AutomationPrice::initialize_asset(
		RawOrigin::Root.into(),
		chain,
		EXCHANGE1.to_vec(),
		ASSET1.to_vec(),
		ASSET2.to_vec(),
		10,
		vec![sender.clone()],
	);
}

// setup a few sample assets, initialize it with sane default vale and set a price to support test cases
pub fn setup_assets_and_prices(sender: &AccountId32, block_time: u128) {
	let _ = AutomationPrice::initialize_asset(
		RawOrigin::Root.into(),
		CHAIN1.to_vec(),
		EXCHANGE1.to_vec(),
		ASSET1.to_vec(),
		ASSET2.to_vec(),
		10,
		vec![sender.clone()],
	);

	let _ = AutomationPrice::initialize_asset(
		RawOrigin::Root.into(),
		CHAIN2.to_vec(),
		EXCHANGE1.to_vec(),
		ASSET2.to_vec(),
		ASSET3.to_vec(),
		10,
		vec![sender.clone()],
	);

	let _ = AutomationPrice::initialize_asset(
		RawOrigin::Root.into(),
		CHAIN2.to_vec(),
		EXCHANGE1.to_vec(),
		ASSET1.to_vec(),
		ASSET3.to_vec(),
		10,
		vec![sender.clone()],
	);

	// This fixture function initialize 3 asset pairs, and set their price to 1000, 5000, 10_000
	const PAIR1_PRICE: u128 = 1000_u128;
	const PAIR2_PRICE: u128 = 5000_u128;
	const PAIR3_PRICE: u128 = 10_000_u128;
	assert_ok!(AutomationPrice::update_asset_prices(
		RuntimeOrigin::signed(sender.clone()),
		vec![CHAIN1.to_vec()],
		vec![EXCHANGE1.to_vec()],
		vec![ASSET1.to_vec()],
		vec![ASSET2.to_vec()],
		vec![PAIR1_PRICE],
		vec![block_time],
		vec![1000],
	));

	assert_ok!(AutomationPrice::update_asset_prices(
		RuntimeOrigin::signed(sender.clone()),
		vec![CHAIN2.to_vec()],
		vec![EXCHANGE1.to_vec()],
		vec![ASSET2.to_vec()],
		vec![ASSET3.to_vec()],
		vec![PAIR2_PRICE],
		vec![block_time],
		vec![1000],
	));

	assert_ok!(AutomationPrice::update_asset_prices(
		RuntimeOrigin::signed(sender.clone()),
		vec![CHAIN2.to_vec()],
		vec![EXCHANGE1.to_vec()],
		vec![ASSET1.to_vec()],
		vec![ASSET3.to_vec()],
		vec![PAIR3_PRICE],
		vec![block_time],
		vec![1000],
	));
}
