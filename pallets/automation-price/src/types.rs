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
use crate::{weights::WeightInfo, Config, InstructionSequence};

use frame_support::pallet_prelude::*;

use sp_std::prelude::*;

use staging_xcm::{latest::prelude::*, VersionedLocation};

/// The struct that stores execution payment for a task.
#[derive(Debug, Encode, Eq, PartialEq, Decode, TypeInfo, Clone)]
pub struct AssetPayment {
    pub asset_location: VersionedLocation,
    pub amount: u128,
}

/// The enum that stores all action specific data.
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub enum Action<AccountId> {
    XCMP {
        destination: Location,
        schedule_fee: Location,
        execution_fee: AssetPayment,
        encoded_call: Vec<u8>,
        encoded_call_weight: Weight,
        overall_weight: Weight,
        schedule_as: Option<AccountId>,
        instruction_sequence: InstructionSequence,
    },
}

impl<AccountId> Action<AccountId> {
    pub fn execution_weight<T: Config>(&self) -> Result<u64, DispatchError> {
        let weight = match self {
            Action::XCMP { .. } => <T as Config>::WeightInfo::run_xcmp_task(),
        };
        Ok(weight.ref_time())
    }

    pub fn schedule_fee_location<T: Config>(&self) -> Location {
        match self {
            Action::XCMP { schedule_fee, .. } => (*schedule_fee).clone(),
        }
    }
}

/// The enum represent  the type of metric we track
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub enum StatType {
    TotalTasksOverall,
    TotalTasksPerAccount,
}
