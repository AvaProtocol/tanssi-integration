// This file is part of Ava Protocol.

// Copyright (C) 2022 Ava Protocol
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

use crate::{self as pallet_xcmp_handler};
use core::cell::RefCell;
use frame_support::{
    parameter_types,
    traits::{ConstU32, Everything, Nothing},
};
use frame_system as system;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain_primitives::primitives::Sibling;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, Convert, IdentityLookup},
    AccountId32, BuildStorage,
};
use staging_xcm::latest::{prelude::*, Weight};
use staging_xcm_builder::{
    AccountId32Aliases, AllowUnpaidExecutionFrom, EnsureXcmOrigin, FixedWeightBounds,
    FrameTransactionalProcessor, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
    SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation,
};
use staging_xcm_executor::{
    traits::{TransactAsset, WeightTrader},
    AssetsInHolding, XcmExecutor,
};

use orml_traits::parameter_type_with_key;

use ava_protocol_primitives::AbsoluteAndRelativeReserveProvider;

type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;
pub type Balance = u128;
pub type CurrencyId = u32;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const LOCAL_PARA_ID: u32 = 2114;
pub const NATIVE: CurrencyId = 0;

frame_support::construct_runtime!(
    pub enum Test
    {
        System: system,
        Balances: pallet_balances,
        ParachainInfo: parachain_info,
        XcmpHandler: pallet_xcmp_handler,
        XcmPallet: pallet_xcm,
        CumulusXcm: cumulus_pallet_xcm,
        Currencies: orml_currencies,
        Tokens: orml_tokens,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 51;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

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
    type AccountId = AccountId;
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

pub struct AccountIdToLocation;
impl Convert<AccountId, Location> for AccountIdToLocation {
    fn convert(account: AccountId) -> Location {
        Junction::AccountId32 {
            network: None,
            id: account.into(),
        }
        .into()
    }
}

thread_local! {
    pub static SENT_XCM: RefCell<Vec<(Location,Xcm<()>)>>  = RefCell::new(Vec::new());
    pub static TRANSACT_ASSET: RefCell<Vec<(Asset,Location)>>  = RefCell::new(Vec::new());
}

pub(crate) fn sent_xcm() -> Vec<(Location, Xcm<()>)> {
    SENT_XCM.with(|q| (*q.borrow()).clone())
}

pub(crate) fn transact_asset() -> Vec<(Asset, Location)> {
    TRANSACT_ASSET.with(|q| (*q.borrow()).clone())
}

pub type LocationToAccountId = (
    ParentIsPreset<AccountId>,
    SiblingParachainConvertsVia<Sibling, AccountId>,
    AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type XcmOriginToCallOrigin = (
    SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
    RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
    SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
    XcmPassthrough<RuntimeOrigin>,
);

/// Sender that returns error if call equals [9,9,9]
pub struct TestSendXcm;
impl SendXcm for TestSendXcm {
    type Ticket = ();

    fn validate(
        destination: &mut Option<Location>,
        message: &mut Option<opaque::Xcm>,
    ) -> SendResult<Self::Ticket> {
        let err_message = Xcm(vec![Transact {
            origin_kind: OriginKind::Native,
            require_weight_at_most: Weight::from_parts(100_000, 0),
            call: vec![9, 1, 1].into(),
        }]);
        if message.clone().unwrap() == err_message {
            Err(SendError::Transport("Destination location full"))
        } else {
            SENT_XCM.with(|q| {
                q.borrow_mut()
                    .push((((*destination).clone()).unwrap(), message.clone().unwrap()))
            });
            Ok(((), Assets::new()))
        }
    }

    fn deliver(_: Self::Ticket) -> Result<XcmHash, SendError> {
        Ok(XcmHash::default())
    }
}

// XCMP Mocks
parameter_types! {
    pub const UnitWeightCost: u64 = 10;
    pub const MaxInstructions: u32 = 100;
}
pub struct DummyWeightTrader;
impl WeightTrader for DummyWeightTrader {
    fn new() -> Self {
        DummyWeightTrader
    }

    fn buy_weight(
        &mut self,
        _weight: Weight,
        _payment: AssetsInHolding,
        _context: &XcmContext,
    ) -> Result<AssetsInHolding, XcmError> {
        Ok(AssetsInHolding::default())
    }
}
pub struct DummyAssetTransactor;
impl TransactAsset for DummyAssetTransactor {
    fn deposit_asset(what: &Asset, who: &Location, _context: Option<&XcmContext>) -> XcmResult {
        let asset = what.clone();
        TRANSACT_ASSET.with(|q| q.borrow_mut().push((asset, who.clone())));
        Ok(())
    }

    fn withdraw_asset(
        what: &Asset,
        who: &Location,
        _maybe_context: Option<&XcmContext>,
    ) -> Result<AssetsInHolding, XcmError> {
        let asset = what.clone();
        TRANSACT_ASSET.with(|q| q.borrow_mut().push((asset.clone(), who.clone())));
        Ok(asset.into())
    }
}

parameter_types! {
    pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
    pub UniversalLocation: InteriorLocation =
        Parachain(2114).into();
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
}
pub struct XcmConfig;
impl staging_xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = TestSendXcm;
    type AssetTransactor = DummyAssetTransactor;
    type OriginConverter = XcmOriginToCallOrigin;
    type IsReserve = ();
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader = DummyWeightTrader;
    type ResponseHandler = ();
    type AssetTrap = XcmPallet;
    type AssetClaims = XcmPallet;
    type SubscriptionService = XcmPallet;

    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = ConstU32<64>;
    type AssetLocker = ();
    type AssetExchanger = ();
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = RuntimeCall;
    type SafeCallFilter = Everything;
    type Aliasers = Nothing;
    type TransactionalProcessor = FrameTransactionalProcessor;
    type HrmpNewChannelOpenRequestHandler = ();
    type HrmpChannelAcceptedHandler = ();
    type HrmpChannelClosingHandler = ();
    type XcmRecorder = ();
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
    pub ReachableDest: Option<Location> = Some(Parent.into());
}

impl pallet_xcm::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = ();
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Nothing;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = LocationToAccountId;
    type MaxLockers = ConstU32<8>;
    type MaxRemoteLockConsumers = ConstU32<0>;
    type WeightInfo = pallet_xcm::TestWeightInfo;
    type RemoteLockConsumerIdentifier = ();
    type AdminOrigin = system::EnsureRoot<AccountId>;
}

impl cumulus_pallet_xcm::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub struct TokenIdConvert;
impl Convert<CurrencyId, Option<Location>> for TokenIdConvert {
    fn convert(_id: CurrencyId) -> Option<Location> {
        // Mock implementation with default value
        Some(Location {
            parents: 1,
            interior: Here,
        })
    }
}

parameter_types! {
    pub const GetNativeCurrencyId: CurrencyId = NATIVE;
    pub Ancestry: Location = Parachain(ParachainInfo::parachain_id().into()).into();
    pub SelfLocation: Location = Location::new(1, Parachain(ParachainInfo::parachain_id().into()));
}

impl pallet_xcmp_handler::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CurrencyId = CurrencyId;
    type MultiCurrency = Currencies;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type SelfParaId = parachain_info::Pallet<Test>;
    type AccountIdToLocation = AccountIdToLocation;
    type CurrencyIdToLocation = TokenIdConvert;
    type UniversalLocation = UniversalLocation;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmSender = TestSendXcm;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type ReserveProvider = AbsoluteAndRelativeReserveProvider<SelfLocation>;
    type SelfLocation = SelfLocation;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    parachain_info::GenesisConfig::<Test> {
        parachain_id: LOCAL_PARA_ID.into(),
        ..Default::default()
    }
    .assimilate_storage(&mut storage)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
