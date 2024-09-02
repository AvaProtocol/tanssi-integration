use crate::{weights::WeightInfo, Config, Error, InstructionSequence, Pallet};

use frame_support::{dispatch::GetDispatchInfo, pallet_prelude::*, traits::Get};

use sp_runtime::traits::CheckedConversion;
use sp_std::prelude::*;

// use pallet_automation_time_rpc_runtime_api::AutomationAction;

use staging_xcm::{latest::prelude::*, VersionedLocation};

pub type Seconds = u64;
pub type UnixTime = u64;

/// The struct that stores execution payment for a task.
#[derive(Debug, Encode, Eq, PartialEq, Decode, TypeInfo, Clone)]
pub struct AssetPayment {
    pub asset_location: VersionedLocation,
    pub amount: u128,
}

/// The enum that stores all action specific data.
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub enum Action<AccountId> {
    XCMP {
        destination: Location,
        schedule_fee: Location,
        execution_fee: Box<AssetPayment>,
        encoded_call: Vec<u8>,
        encoded_call_weight: Weight,
        overall_weight: Weight,
        schedule_as: Option<AccountId>,
        instruction_sequence: InstructionSequence,
    },
    DynamicDispatch {
        encoded_call: Vec<u8>,
    },
}

impl<AccountId> Action<AccountId> {
    pub fn execution_weight<T: Config>(&self) -> Result<u64, DispatchError> {
        let weight = match self {
            Action::XCMP { .. } => <T as Config>::WeightInfo::run_xcmp_task(),
            Action::DynamicDispatch { encoded_call } => {
                let scheduled_call: <T as Config>::RuntimeCall =
                    Decode::decode(&mut &**encoded_call)
                        .map_err(|_| Error::<T>::CallCannotBeDecoded)?;
                <T as Config>::WeightInfo::run_dynamic_dispatch_action()
                    .saturating_add(scheduled_call.get_dispatch_info().weight)
            }
        };
        Ok(weight.ref_time())
    }

    pub fn schedule_fee_location<T: Config>(&self) -> Location {
        match self {
            Action::XCMP { schedule_fee, .. } => schedule_fee.clone(),
            _ => Location::default(),
        }
    }
}

/// API Param for Scheduling
#[derive(Clone, Debug, Decode, Encode, PartialEq, TypeInfo)]
pub enum ScheduleParam {
    Fixed {
        execution_times: Vec<UnixTime>,
    },
    Recurring {
        next_execution_time: UnixTime,
        frequency: Seconds,
    },
}

impl ScheduleParam {
    /// Convert from ScheduleParam to Schedule
    pub fn validated_into<T: Config>(self) -> Result<Schedule, DispatchError> {
        match self {
            Self::Fixed {
                execution_times, ..
            } => Schedule::new_fixed_schedule::<T>(execution_times),
            Self::Recurring {
                next_execution_time,
                frequency,
            } => Schedule::new_recurring_schedule::<T>(next_execution_time, frequency),
        }
    }

    /// Number of known executions at the time of scheduling the task
    pub fn number_of_executions(&self) -> u32 {
        match self {
            Self::Fixed { execution_times } => {
                execution_times.len().try_into().expect("bounded by u32")
            }
            Self::Recurring { .. } => 1,
        }
    }
}

#[derive(Clone, Debug, Decode, Encode, PartialEq, TypeInfo)]
pub enum Schedule {
    Fixed {
        execution_times: Vec<UnixTime>,
        executions_left: u32,
    },
    Recurring {
        next_execution_time: UnixTime,
        frequency: Seconds,
    },
}

impl Schedule {
    pub fn new_fixed_schedule<T: Config>(
        mut execution_times: Vec<UnixTime>,
    ) -> Result<Self, DispatchError> {
        Pallet::<T>::clean_execution_times_vector(&mut execution_times);
        let executions_left = execution_times.len() as u32;
        let schedule = Self::Fixed {
            execution_times,
            executions_left,
        };
        schedule.valid::<T>()?;
        Ok(schedule)
    }

    pub fn new_recurring_schedule<T: Config>(
        next_execution_time: UnixTime,
        frequency: Seconds,
    ) -> Result<Self, DispatchError> {
        let schedule = Self::Recurring {
            next_execution_time,
            frequency,
        };
        schedule.valid::<T>()?;
        Ok(schedule)
    }

    pub fn known_executions_left(&self) -> u32 {
        match self {
            Self::Fixed {
                executions_left, ..
            } => *executions_left,
            Self::Recurring { .. } => 1,
        }
    }

    fn valid<T: Config>(&self) -> DispatchResult {
        match self {
            Self::Fixed {
                execution_times, ..
            } => {
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
            }
            Self::Recurring {
                next_execution_time,
                frequency,
            } => {
                Pallet::<T>::is_valid_time(*next_execution_time)?;
                // Validate frequency by ensuring that the next proposed execution is at a valid time
                let next_recurrence = next_execution_time
                    .checked_add(*frequency)
                    .ok_or(Error::<T>::TimeTooFarOut)?;
                if *next_execution_time == next_recurrence {
                    Err(Error::<T>::InvalidTime)?;
                }
                Pallet::<T>::is_valid_time(next_recurrence)?;
            }
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
    pub action: Action<AccountId>,
    pub abort_errors: Vec<Vec<u8>>,
}

impl<AccountId: Ord> PartialEq for Task<AccountId> {
    fn eq(&self, other: &Self) -> bool {
        self.owner_id == other.owner_id
            && self.task_id == other.task_id
            && self.action == other.action
            && self.schedule == other.schedule
    }
}

impl<AccountId: Ord> Eq for Task<AccountId> {}

impl<AccountId: Clone> Task<AccountId> {
    pub fn new(
        owner_id: AccountId,
        task_id: Vec<u8>,
        schedule: Schedule,
        action: Action<AccountId>,
        abort_errors: Vec<Vec<u8>>,
    ) -> Self {
        Self {
            owner_id,
            task_id,
            schedule,
            action,
            abort_errors,
        }
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
        let action = Action::DynamicDispatch {
            encoded_call: call.encode(),
        };
        let schedule = Schedule::new_fixed_schedule::<T>(execution_times)?;
        Ok(Self::new(owner_id, task_id, schedule, action, abort_errors))
    }

    pub fn create_xcmp_task<T: Config>(
        owner_id: AccountId,
        task_id: Vec<u8>,
        execution_times: Vec<UnixTime>,
        destination: Location,
        schedule_fee: Location,
        execution_fee: AssetPayment,
        encoded_call: Vec<u8>,
        encoded_call_weight: Weight,
        overall_weight: Weight,
        schedule_as: Option<AccountId>,
        instruction_sequence: InstructionSequence,
        abort_errors: Vec<Vec<u8>>,
    ) -> Result<Self, DispatchError> {
        let action = Action::XCMP {
            destination,
            schedule_fee,
            execution_fee: Box::new(execution_fee),
            encoded_call,
            encoded_call_weight,
            overall_weight,
            schedule_as,
            instruction_sequence,
        };
        let schedule = Schedule::new_fixed_schedule::<T>(execution_times)?;
        Ok(Self::new(owner_id, task_id, schedule, action, abort_errors))
    }

    pub fn execution_times(&self) -> Vec<UnixTime> {
        match &self.schedule {
            Schedule::Fixed {
                execution_times, ..
            } => execution_times.to_vec(),
            Schedule::Recurring {
                next_execution_time,
                ..
            } => {
                vec![*next_execution_time]
            }
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
        Self {
            owner_id,
            task_id,
            execution_time,
        }
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
        Self {
            tasks: vec![],
            weight: 0,
        }
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
        let weight = self
            .weight
            .checked_add(action_weight as u128)
            .ok_or(Error::<T>::TimeSlotFull)?;
        // A hard limit on tasks/slot prevents unforseen performance consequences
        // that could occur when scheduling a huge number of lightweight tasks.
        // Also allows us to make reasonable assumptions for worst case benchmarks.
        if self.tasks.len() as u32 >= T::MaxTasksPerSlot::get()
            || weight > T::MaxWeightPerSlot::get()
        {
            Err(Error::<T>::TimeSlotFull)?
        }

        self.weight = weight;
        self.tasks.push((task.owner_id.clone(), task_id));
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mock::*,
        tests::{SCHEDULED_TIME, SLOT_SIZE_SECONDS, START_BLOCK_TIME},
    };
    use frame_support::{assert_err, assert_ok};

    mod scheduled_tasks {
        use super::*;
        use crate::{AccountTaskId, ScheduledTasksOf, TaskOf};
        use sp_runtime::AccountId32;

        #[test]
        fn try_push_errors_when_slot_is_full_by_weight() {
            new_test_ext(START_BLOCK_TIME).execute_with(|| {
                let task = TaskOf::<Test>::create_event_task::<Test>(
                    AccountId32::new(ALICE),
                    vec![0],
                    vec![SCHEDULED_TIME],
                    vec![0],
                    vec![],
                )
                .unwrap();
                let task_id = vec![48, 45, 48, 45, 48];
                assert_err!(
                    ScheduledTasksOf::<Test> {
                        tasks: vec![],
                        weight: MaxWeightPerSlot::get()
                    }
                    .try_push::<Test>(task_id, &task),
                    Error::<Test>::TimeSlotFull
                );
            })
        }

        #[test]
        fn try_push_errors_when_slot_is_full_by_task_count() {
            new_test_ext(START_BLOCK_TIME).execute_with(|| {
                let alice = AccountId32::new(ALICE);
                let id = (alice.clone(), vec![49, 45, 48, 45, 42]);

                let task = TaskOf::<Test>::create_event_task::<Test>(
                    alice,
                    vec![0],
                    vec![SCHEDULED_TIME],
                    vec![0],
                    vec![],
                )
                .unwrap();
                let tasks = (0..MaxTasksPerSlot::get()).fold::<Vec<AccountTaskId<Test>>, _>(
                    vec![],
                    |mut tasks, _| {
                        tasks.push(id.clone());
                        tasks
                    },
                );
                let task_id = vec![48, 45, 48, 45, 48];
                assert_err!(
                    ScheduledTasksOf::<Test> { tasks, weight: 0 }.try_push::<Test>(task_id, &task),
                    Error::<Test>::TimeSlotFull
                );
            })
        }

        // verify calling try_push to push the  task into the schedule work when
        // slot is not full, not reaching the max weight and max tasks per slot
        // task will be pushed to the `tasks` field and the `weight` field is
        // increased properly for the weight of the task.
        //
        // the total weight of schedule will be weight of the schedule_* itself
        // plus any to be call extrinsics in case of dynamic dispatch
        //
        #[test]
        fn try_push_works_when_slot_is_not_full() {
            new_test_ext(START_BLOCK_TIME).execute_with(|| {
                let task = TaskOf::<Test>::create_event_task::<Test>(
                    AccountId32::new(ALICE),
                    vec![0],
                    vec![SCHEDULED_TIME],
                    vec![0],
                    vec![],
                )
                .unwrap();
                // When we schedule a test on the first block, on the first extrinsics and no event
                // at all this is the first task id we generate
                // {block-num}-{extrinsics-idx}-{evt-idx}
                let task_id = "0-1-0".as_bytes().to_vec();
                let mut scheduled_tasks = ScheduledTasksOf::<Test>::default();
                scheduled_tasks
                    .try_push::<Test>(task_id.clone(), &task)
                    .expect("slot is not full");

                assert_eq!(scheduled_tasks.tasks, vec![(task.owner_id, task_id)]);

                // this is same call we mock in create_event_tasks
                let call: <Test as frame_system::Config>::RuntimeCall =
                    frame_system::Call::remark_with_event { remark: vec![0] }.into();
                // weight will be equal = weight of the dynamic dispatch + the call itself
                assert_eq!(
                    scheduled_tasks.weight,
                    20_000 + call.get_dispatch_info().weight.ref_time() as u128
                );
            })
        }
    }

    mod schedule_param {
        use super::*;

        #[test]
        fn sets_executions_left() {
            new_test_ext(START_BLOCK_TIME).execute_with(|| {
                let t1 = SCHEDULED_TIME + SLOT_SIZE_SECONDS;
                let t2 = SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2;
                let t3 = SCHEDULED_TIME + SLOT_SIZE_SECONDS * 3;
                let s = ScheduleParam::Fixed {
                    execution_times: vec![t1, t2, t3],
                }
                .validated_into::<Test>()
                .expect("valid");
                if let Schedule::Fixed {
                    executions_left, ..
                } = s
                {
                    assert_eq!(executions_left, 3);
                } else {
                    panic!("Exepected Schedule::Fixed");
                }
            })
        }

        #[test]
        fn validates_fixed_schedule() {
            new_test_ext(START_BLOCK_TIME).execute_with(|| {
                let t1 = SCHEDULED_TIME + SLOT_SIZE_SECONDS / 2;
                let s = ScheduleParam::Fixed {
                    execution_times: vec![t1],
                }
                .validated_into::<Test>();
                assert_err!(s, Error::<Test>::InvalidTime);
            })
        }

        #[test]
        fn validates_recurring_schedule() {
            new_test_ext(START_BLOCK_TIME).execute_with(|| {
                let s = ScheduleParam::Recurring {
                    next_execution_time: SCHEDULED_TIME,
                    frequency: SLOT_SIZE_SECONDS,
                }
                .validated_into::<Test>()
                .expect("valid");
                if let Schedule::Recurring {
                    next_execution_time,
                    ..
                } = s
                {
                    assert_eq!(next_execution_time, SCHEDULED_TIME);
                } else {
                    panic!("Exepected Schedule::Recurring");
                }

                let s = ScheduleParam::Recurring {
                    next_execution_time: SCHEDULED_TIME,
                    frequency: SLOT_SIZE_SECONDS + 1,
                }
                .validated_into::<Test>();
                assert_err!(s, Error::<Test>::InvalidTime);
            })
        }

        #[test]
        fn counts_executions() {
            new_test_ext(START_BLOCK_TIME).execute_with(|| {
                let t1 = SCHEDULED_TIME + SLOT_SIZE_SECONDS;
                let t2 = SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2;
                let t3 = SCHEDULED_TIME + SLOT_SIZE_SECONDS * 3;

                let s = ScheduleParam::Fixed {
                    execution_times: vec![t1, t2, t3],
                };
                assert_eq!(s.number_of_executions(), 3);

                let s = ScheduleParam::Recurring {
                    next_execution_time: SCHEDULED_TIME,
                    frequency: SLOT_SIZE_SECONDS,
                };
                assert_eq!(s.number_of_executions(), 1);
            })
        }
    }

    mod schedule {
        use super::*;

        #[test]
        fn new_fixed_schedule_sets_executions_left() {
            new_test_ext(START_BLOCK_TIME).execute_with(|| {
                let t1 = SCHEDULED_TIME + SLOT_SIZE_SECONDS;
                let t2 = SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2;
                let t3 = SCHEDULED_TIME + SLOT_SIZE_SECONDS * 3;
                let s = Schedule::new_fixed_schedule::<Test>(vec![t1, t2, t3]).unwrap();
                if let Schedule::Fixed {
                    executions_left, ..
                } = s
                {
                    assert_eq!(executions_left, 3);
                } else {
                    panic!("Exepected Schedule::Fixed");
                }
            })
        }

        #[test]
        fn new_fixed_schedule_errors_with_too_many_executions() {
            new_test_ext(0).execute_with(|| {
                let s = Schedule::new_fixed_schedule::<Test>(
                    (0u64..=MaxExecutionTimes::get() as u64).collect(),
                );
                assert_err!(s, Error::<Test>::TooManyExecutionsTimes);
            })
        }

        #[test]
        fn new_fixed_schedule_cleans_execution_times() {
            new_test_ext(START_BLOCK_TIME).execute_with(|| {
                let t1 = SCHEDULED_TIME + SLOT_SIZE_SECONDS;
                let t2 = SCHEDULED_TIME + SLOT_SIZE_SECONDS * 2;
                let t3 = SCHEDULED_TIME + SLOT_SIZE_SECONDS * 3;
                let s = Schedule::new_fixed_schedule::<Test>(vec![t1, t3, t2, t3, t3]);
                if let Schedule::Fixed {
                    execution_times, ..
                } = s.unwrap()
                {
                    assert_eq!(execution_times, vec![t1, t2, t3]);
                } else {
                    panic!("Exepected Schedule::Fixed");
                }
            })
        }

        #[test]
        fn checks_for_fixed_schedule_validity() {
            new_test_ext(START_BLOCK_TIME).execute_with(|| {
                assert_ok!(Schedule::new_fixed_schedule::<Test>(vec![
                    SCHEDULED_TIME + SLOT_SIZE_SECONDS
                ]));
                // Execution time does not end in whole hour
                assert_err!(
                    Schedule::new_fixed_schedule::<Test>(vec![
                        SCHEDULED_TIME + SLOT_SIZE_SECONDS,
                        SCHEDULED_TIME + SLOT_SIZE_SECONDS + SLOT_SIZE_SECONDS / 2
                    ]),
                    Error::<Test>::InvalidTime
                );
                // No execution times
                assert_err!(
                    Schedule::new_fixed_schedule::<Test>(vec![]),
                    Error::<Test>::InvalidTime
                );
            })
        }

        #[test]
        fn checks_for_recurring_schedule_validity() {
            let start_time = 1_663_225_200;
            new_test_ext(start_time * 1_000).execute_with(|| {
                assert_ok!(Schedule::Recurring {
                    next_execution_time: start_time + SLOT_SIZE_SECONDS,
                    frequency: SLOT_SIZE_SECONDS
                }
                .valid::<Test>());
                // Next execution time not at hour granuality
                assert_err!(
                    Schedule::Recurring {
                        next_execution_time: start_time + SLOT_SIZE_SECONDS + SLOT_SIZE_SECONDS / 2,
                        frequency: SLOT_SIZE_SECONDS
                    }
                    .valid::<Test>(),
                    Error::<Test>::InvalidTime
                );
                // Frequency not at hour granularity
                assert_err!(
                    Schedule::Recurring {
                        next_execution_time: start_time + SLOT_SIZE_SECONDS + SLOT_SIZE_SECONDS / 2,
                        frequency: SLOT_SIZE_SECONDS + SLOT_SIZE_SECONDS / 2
                    }
                    .valid::<Test>(),
                    Error::<Test>::InvalidTime
                );
                // Frequency of 0
                assert_err!(
                    Schedule::Recurring {
                        next_execution_time: start_time + SLOT_SIZE_SECONDS,
                        frequency: 0
                    }
                    .valid::<Test>(),
                    Error::<Test>::InvalidTime
                );
                // Frequency too far out
                assert_err!(
                    Schedule::Recurring {
                        next_execution_time: start_time + SLOT_SIZE_SECONDS,
                        frequency: start_time + SLOT_SIZE_SECONDS
                    }
                    .valid::<Test>(),
                    Error::<Test>::TimeTooFarOut
                );
            })
        }

        #[test]
        fn number_of_known_executions_for_fixed() {
            new_test_ext(0).execute_with(|| {
                let s = Schedule::Fixed {
                    execution_times: vec![],
                    executions_left: 5,
                };
                assert_eq!(s.known_executions_left(), 5);
            })
        }

        #[test]
        fn number_of_known_executions_for_recurring() {
            new_test_ext(0).execute_with(|| {
                let s = Schedule::Recurring {
                    next_execution_time: 0,
                    frequency: 0,
                };
                assert_eq!(s.known_executions_left(), 1);
            })
        }
    }
}
