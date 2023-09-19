#![no_std]

use codec::{Decode, Encode};
use gmeta::{In, InOut, Metadata};
use gstd::prelude::*;
use scale_info::TypeInfo;

#[derive(Default, Debug, Encode, Decode, TypeInfo)]
pub struct IOOnchainQuant {
    // Regular Investment Ratio
    pub r_invest_ration: u64,
    pub block_step: u32,
    pub block_next: u32,
    pub action_id: u64,
}

#[derive(Encode, Decode, TypeInfo)]
pub enum OcqAction {
    Start,
    Stop,
    Act,
    GasReserve,
    Terminate,
}

#[derive(Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
#[codec(crate = gstd::codec)]
#[scale_info(crate = gstd::scale_info)]
pub enum OcqEvent {
    Start,
    Stop,
    Act,
    GasReserve { amount: u64, time: u32 },
    Terminate,
}

#[derive(Debug, Decode, Encode, TypeInfo)]
#[codec(crate = gstd::codec)]
#[scale_info(crate = gstd::scale_info)]
pub struct InitConfig {
    // Regular Investment Ratio, in 0.000001
    pub r_invest_ration: u64,
    pub block_step: u32,
}

pub struct ProgramMetadata;

impl Metadata for ProgramMetadata {
    type Init = In<InitConfig>;
    type Handle = InOut<OcqAction, OcqEvent>;
    type State = IOOnchainQuant;
    type Reply = ();
    type Others = ();
    type Signal = ();
}
