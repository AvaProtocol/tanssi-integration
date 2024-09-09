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
#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{parameter_types, traits::Get};
use orml_traits::currency::MutationHooks;
use sp_std::marker::PhantomData;

pub mod constants;
pub mod fees;

pub struct CurrencyHooks<T, DustAccount>(PhantomData<T>, DustAccount);
impl<T, DustAccount> MutationHooks<T::AccountId, T::CurrencyId, T::Balance>
    for CurrencyHooks<T, DustAccount>
where
    T: orml_tokens::Config,
    DustAccount: Get<<T as frame_system::Config>::AccountId>,
{
    type OnDust = orml_tokens::TransferDust<T, DustAccount>;
    type OnSlash = ();
    type PreDeposit = ();
    type PostDeposit = ();
    type PreTransfer = ();
    type PostTransfer = ();
    type OnNewTokenAccount = ();
    type OnKilledTokenAccount = ();
}

pub mod config {

    pub mod orml_asset_registry {
        use crate::*;
        use ava_protocol_primitives::{assets::CustomMetadata, Balance};
        use orml_traits::asset_registry::AssetMetadata;

        parameter_types! {
            pub const StringLimit: u32 = 50;
        }

        pub type AssetMetadataOf = AssetMetadata<Balance, CustomMetadata, StringLimit>;
        // type CurrencyAdapter<Runtime> = orml_tokens::MultiTokenCurrencyAdapter<Runtime>;

        // pub struct SequentialIdWithCreation<T>(PhantomData<T>);
        // impl<T> AssetProcessor<TokenId, AssetMetadataOf> for SequentialIdWithCreation<T>
        // where
        // 	T: orml_asset_registry::Config,
        // 	T: orml_tokens::Config,
        // 	T: pallet_treasury::Config,
        // 	TokenId: From<<T as orml_tokens::Config>::CurrencyId>,
        // {
        // 	fn pre_register(
        // 		id: Option<TokenId>,
        // 		asset_metadata: AssetMetadataOf,
        // 	) -> Result<(TokenId, AssetMetadataOf), DispatchError> {
        // 		let next_id = CurrencyAdapter::<T>::get_next_currency_id();
        // 		let asset_id = id.unwrap_or(next_id.into());
        // 		let treasury_account =
        // 			config::TreasuryPalletIdOf::<T>::get().into_account_truncating();

        // 		match asset_id.cmp(&next_id.into()) {
        // 			Ordering::Equal =>
        // 				CurrencyAdapter::<T>::create(&treasury_account, Default::default())
        // 					.and_then(|created_asset_id| {
        // 						match created_asset_id.cmp(&asset_id.into()) {
        // 							Ordering::Equal => Ok((asset_id, asset_metadata)),
        // 							_ =>
        // 								Err(orml_asset_registry::Error::<T>::InvalidAssetId.into()),
        // 						}
        // 					}),
        // 			Ordering::Less => Ok((asset_id, asset_metadata)),
        // 			_ => Err(orml_asset_registry::Error::<T>::InvalidAssetId.into()),
        // 		}
        // 	}
        // }

        // pub struct AssetAuthority<T>(PhantomData<T>);
        // impl<T> EnsureOriginWithArg<T::RuntimeOrigin, Option<u32>> for AssetAuthority<T>
        // where
        // 	T: frame_system::Config,
        // {
        // 	type Success = ();

        // 	fn try_origin(
        // 		origin: T::RuntimeOrigin,
        // 		_asset_id: &Option<u32>,
        // 	) -> Result<Self::Success, T::RuntimeOrigin> {
        // 		<EnsureRoot<_> as EnsureOrigin<T::RuntimeOrigin>>::try_origin(origin)
        // 	}

        // 	#[cfg(feature = "runtime-benchmarks")]
        // 	fn try_successful_origin(_: &Option<u32>) -> Result<T::RuntimeOrigin, ()> {
        // 		Ok(T::RuntimeOrigin::root())
        // 	}
        // }
    }
}
