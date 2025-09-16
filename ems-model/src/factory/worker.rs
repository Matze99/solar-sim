use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;
use utoipa::ToSchema;

/// Represents a time of day in 24-hour format
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./worker.ts")]
pub struct Time {
    pub hour: u8,   // 0-23
    pub minute: u8, // 0-59
}

impl Time {
    pub fn new(hour: u8, minute: u8) -> Self {
        Time { hour, minute }
    }
}

/// Represents a work shift with start and end times
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./worker.ts")]
pub struct WorkShift {
    pub start: Time,
    pub end: Time,
}

impl WorkShift {
    pub fn new(start: Time, end: Time) -> Self {
        WorkShift { start, end }
    }
}

/// Days of the week
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./worker.ts")]
pub enum WeekDay {
    #[schema(rename = "monday")]
    #[serde(rename = "monday")]
    Monday,
    #[schema(rename = "tuesday")]
    #[serde(rename = "tuesday")]
    Tuesday,
    #[schema(rename = "wednesday")]
    #[serde(rename = "wednesday")]
    Wednesday,
    #[schema(rename = "thursday")]
    #[serde(rename = "thursday")]
    Thursday,
    #[schema(rename = "friday")]
    #[serde(rename = "friday")]
    Friday,
    #[schema(rename = "saturday")]
    #[serde(rename = "saturday")]
    Saturday,
    #[schema(rename = "sunday")]
    #[serde(rename = "sunday")]
    Sunday,
}

/// Simple weekly schedule
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./worker.ts")]
pub struct Schedule {
    pub weekly_shifts: HashMap<WeekDay, WorkShift>,
}

impl Schedule {
    pub fn new() -> Self {
        Schedule {
            weekly_shifts: HashMap::new(),
        }
    }

    pub fn add_shift(&mut self, day: WeekDay, shift: WorkShift) {
        self.weekly_shifts.insert(day, shift);
    }

    pub fn remove_shift(&mut self, day: WeekDay) {
        self.weekly_shifts.remove(&day);
    }

    pub fn get_shift(&self, day: WeekDay) -> Option<&WorkShift> {
        self.weekly_shifts.get(&day)
    }
}

/// What can a worker do?
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./worker.ts")]
pub enum Specialization {
    Custom(String),
    CncMachineOperator,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./worker.ts")]
pub struct Worker {
    pub id: String,
    pub name: String,
    pub specialization: Vec<Specialization>,
    pub schedule: Schedule,
}

impl Worker {
    pub fn new(
        id: String,
        name: String,
        specialization: Vec<Specialization>,
        schedule: Schedule,
    ) -> Self {
        Worker {
            id,
            name,
            specialization,
            schedule,
        }
    }

    /// Get the worker's schedule
    pub fn get_schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Update the worker's schedule
    pub fn set_schedule(&mut self, schedule: Schedule) {
        self.schedule = schedule;
    }

    /// Add a work shift for a specific day
    pub fn add_work_shift(&mut self, day: WeekDay, start: Time, end: Time) {
        let shift = WorkShift::new(start, end);
        self.schedule.add_shift(day, shift);
    }

    /// Remove a work shift for a specific day
    pub fn remove_work_shift(&mut self, day: WeekDay) {
        self.schedule.remove_shift(day);
    }
}
