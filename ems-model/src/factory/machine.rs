use crate::factory::worker::Specialization;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

/// How can a machine be controlled?
/// does it need to be controlled by a human or can it be controlled by a computer?
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./machine.ts")]
pub enum MachineControl {
    /// The machine is controlled by a human.
    Human,
    /// The machine is controlled by a computer.
    Computer,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./machine.ts")]
pub enum StepType {
    /// A machine that can be turned on and off.
    Machine,
    /// A task that can be done by a worker.
    Task,
}

/// A machine in the factory.
///
/// for the ems it is important how flexible the machine is in terms of how it can be turned on and off,
/// for how long a run is, and how much electricity it consumes.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./machine.ts")]
pub struct Step {
    pub id: String,
    /// The type of the step.
    pub step_type: StepType,
    /// The name of the machine/task.
    pub name: String,
    /// The power consumption of the machine.
    pub power_consumption: f64,
    /// The runtime of the machine in minutes.
    pub runtime_minutes: f64,
    /// the control of the machine
    pub control: MachineControl,
    /// required specialization of the worker
    pub required_specialization: Option<Specialization>,
}
