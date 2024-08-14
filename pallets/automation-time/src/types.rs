use crate::{weights::WeightInfo, Config, Error, Pallet};

use frame_support::{dispatch::GetDispatchInfo, pallet_prelude::*, traits::Get};

use sp_runtime::traits::{AtLeast32BitUnsigned, CheckedConversion};
use sp_std::prelude::*;

// use pallet_automation_time_rpc_runtime_api::AutomationAction;

use staging_xcm::{latest::prelude::*, VersionedMultiLocation};
use staging_xcm::opaque::v3::{MultiLocation, MultiAsset};

pub type Seconds = u64;
pub type UnixTime = u64;

/// The struct that stores execution payment for a task.
#[derive(Debug, Encode, Eq, PartialEq, Decode, TypeInfo, Clone)]
pub struct AssetPayment {
	pub asset_location: VersionedMultiLocation,
	pub amount: u128,
}

/// The enum that stores all action specific data.
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub enum Action {
	DynamicDispatch {
		encoded_call: Vec<u8>,
	},
}

impl Action {
	pub fn execution_weight<T: Config>(&self) -> Result<u64, DispatchError> {
		let weight = match self {
			Action::DynamicDispatch { encoded_call } => {
				let scheduled_call: <T as Config>::RuntimeCall =
					Decode::decode(&mut &**encoded_call)
						.map_err(|_| Error::<T>::CallCannotBeDecoded)?;
				<T as Config>::WeightInfo::run_dynamic_dispatch_action()
					.saturating_add(scheduled_call.get_dispatch_info().weight)
			},
		};
		Ok(weight.ref_time())
	}

	pub fn schedule_fee_location<T: Config>(&self) -> MultiLocation {
		MultiLocation::default()
	}
}

/// API Param for Scheduling
#[derive(Clone, Debug, Decode, Encode, PartialEq, TypeInfo)]
pub enum ScheduleParam {
	Fixed { execution_times: Vec<UnixTime> },
	Recurring { next_execution_time: UnixTime, frequency: Seconds },
}

impl ScheduleParam {
	/// Convert from ScheduleParam to Schedule
	pub fn validated_into<T: Config>(self) -> Result<Schedule, DispatchError> {
		match self {
			Self::Fixed { execution_times, .. } =>
				Schedule::new_fixed_schedule::<T>(execution_times),
			Self::Recurring { next_execution_time, frequency } =>
				Schedule::new_recurring_schedule::<T>(next_execution_time, frequency),
		}
	}

	/// Number of known executions at the time of scheduling the task
	pub fn number_of_executions(&self) -> u32 {
		match self {
			Self::Fixed { execution_times } =>
				execution_times.len().try_into().expect("bounded by u32"),
			Self::Recurring { .. } => 1,
		}
	}
}

#[derive(Clone, Debug, Decode, Encode, PartialEq, TypeInfo)]
pub enum Schedule {
	Fixed { execution_times: Vec<UnixTime>, executions_left: u32 },
	Recurring { next_execution_time: UnixTime, frequency: Seconds },
}

impl Schedule {
	pub fn new_fixed_schedule<T: Config>(
		mut execution_times: Vec<UnixTime>,
	) -> Result<Self, DispatchError> {
		Pallet::<T>::clean_execution_times_vector(&mut execution_times);
		let executions_left = execution_times.len() as u32;
		let schedule = Self::Fixed { execution_times, executions_left };
		schedule.valid::<T>()?;
		Ok(schedule)
	}

	pub fn new_recurring_schedule<T: Config>(
		next_execution_time: UnixTime,
		frequency: Seconds,
	) -> Result<Self, DispatchError> {
		let schedule = Self::Recurring { next_execution_time, frequency };
		schedule.valid::<T>()?;
		Ok(schedule)
	}

	pub fn known_executions_left(&self) -> u32 {
		match self {
			Self::Fixed { executions_left, .. } => *executions_left,
			Self::Recurring { .. } => 1,
		}
	}

	fn valid<T: Config>(&self) -> DispatchResult {
		match self {
			Self::Fixed { execution_times, .. } => {
				let number_of_executions: u32 = execution_times
					.len()
					.checked_into()
					.ok_or(Error::<T>::TooManyExecutionsTimes)?;
				if number_of_executions == 0 {
					Err(Error::<T>::InvalidTime)?;
				}
				if number_of_executions > T::MaxExecutionTimes::get() {
					Err(Error::<T>::TooManyExecutionsTimes)?;
				}
				for time in execution_times.iter() {
					Pallet::<T>::is_valid_time(*time)?;
				}
			},
			Self::Recurring { next_execution_time, frequency } => {
				Pallet::<T>::is_valid_time(*next_execution_time)?;
				// Validate frequency by ensuring that the next proposed execution is at a valid time
				let next_recurrence =
					next_execution_time.checked_add(*frequency).ok_or(Error::<T>::TimeTooFarOut)?;
				if *next_execution_time == next_recurrence {
					Err(Error::<T>::InvalidTime)?;
				}
				Pallet::<T>::is_valid_time(next_recurrence)?;
			},
		}
		Ok(())
	}
}

/// The struct that stores all information needed for a task.
#[derive(Debug, Encode, Decode, TypeInfo, Clone)]
#[scale_info(skip_type_params(MaxExecutionTimes))]
pub struct Task<AccountId> {
	pub owner_id: AccountId,
	pub task_id: Vec<u8>,
	pub schedule: Schedule,
	pub action: Action,
	pub abort_errors: Vec<Vec<u8>>,
}

impl<AccountId: Ord> PartialEq for Task<AccountId> {
	fn eq(&self, other: &Self) -> bool {
		self.owner_id == other.owner_id &&
			self.task_id == other.task_id &&
			self.action == other.action &&
			self.schedule == other.schedule
	}
}

impl<AccountId: Ord> Eq for Task<AccountId> {}

impl<AccountId: Clone> Task<AccountId> {
	pub fn new(
		owner_id: AccountId,
		task_id: Vec<u8>,
		schedule: Schedule,
		action: Action,
		abort_errors: Vec<Vec<u8>>,
	) -> Self {
		Self { owner_id, task_id, schedule, action, abort_errors }
	}

	pub fn create_event_task<T: Config>(
		owner_id: AccountId,
		task_id: Vec<u8>,
		execution_times: Vec<UnixTime>,
		message: Vec<u8>,
		abort_errors: Vec<Vec<u8>>,
	) -> Result<Self, DispatchError> {
		let call: <T as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark_with_event { remark: message }.into();
		let action = Action::DynamicDispatch { encoded_call: call.encode() };
		let schedule = Schedule::new_fixed_schedule::<T>(execution_times)?;
		Ok(Self::new(owner_id, task_id, schedule, action, abort_errors))
	}

	pub fn execution_times(&self) -> Vec<UnixTime> {
		match &self.schedule {
			Schedule::Fixed { execution_times, .. } => execution_times.to_vec(),
			Schedule::Recurring { next_execution_time, .. } => {
				vec![*next_execution_time]
			},
		}
	}
}

#[derive(Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub struct MissedTaskV2<AccountId, TaskId> {
	pub owner_id: AccountId,
	pub task_id: TaskId,
	pub execution_time: UnixTime,
}

impl<AccountId, TaskId> MissedTaskV2<AccountId, TaskId> {
	pub fn new(owner_id: AccountId, task_id: TaskId, execution_time: UnixTime) -> Self {
		Self { owner_id, task_id, execution_time }
	}
}

#[derive(Debug, Decode, Eq, Encode, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ScheduledTasks<AccountId, TaskId> {
	pub tasks: Vec<(AccountId, TaskId)>,
	pub weight: u128,
}
impl<A, B> Default for ScheduledTasks<A, B> {
	fn default() -> Self {
		Self { tasks: vec![], weight: 0 }
	}
}
impl<AccountId, TaskId> ScheduledTasks<AccountId, TaskId> {
	pub fn try_push<T: Config>(
		&mut self,
		task_id: TaskId,
		task: &Task<AccountId>,
	) -> Result<&mut Self, DispatchError>
	where
		AccountId: Clone,
	{
		let action_weight = task.action.execution_weight::<T>()?;
		let weight =
			self.weight.checked_add(action_weight as u128).ok_or(Error::<T>::TimeSlotFull)?;
		// A hard limit on tasks/slot prevents unforseen performance consequences
		// that could occur when scheduling a huge number of lightweight tasks.
		// Also allows us to make reasonable assumptions for worst case benchmarks.
		if self.tasks.len() as u32 >= T::MaxTasksPerSlot::get() ||
			weight > T::MaxWeightPerSlot::get()
		{
			Err(Error::<T>::TimeSlotFull)?
		}

		self.weight = weight;
		self.tasks.push((task.owner_id.clone(), task_id));
		Ok(self)
	}
}
