use gstd::{
    debug, errors::Error as GstdError, errors::Result as GstdResult, exec, msg, prelude::*,
    ActorId, MessageId, ReservationId,
};

use onchainquant_io::*;

use crate::price;
use hex_literal::hex;

#[derive(Debug, Clone, Default)]
pub struct OnchainQuant {
    // Regular Investment Ratio, in 0.000001
    pub r_invest_ration: u64,
    pub reservation_ids: HashMap<ActorId, ReservationId>,
    pub token_info: HashMap<String, TokenInfo>,
    pub block_step: u32,
    pub block_next: u32,
    pub action_id: u64,
    pub owner: ActorId,
}
static mut ONCHAIN_QUANT: Option<OnchainQuant> = None;

static RESERVATION_AMOUNT: u64 = 50_000_000_000;
static REPLY_GAS_AMOUNT: u64 = 20_000_000_000;
// 30 days
static RESERVATION_TIME: u32 = 30 * 24 * 60 * 60 / 2;

#[derive(Debug)]
enum QuantError {
    ContractError(GstdError),
}

impl From<GstdError> for QuantError {
    fn from(value: GstdError) -> Self {
        Self::ContractError(value)
    }
}

impl OnchainQuant {
    async fn start(&mut self) {
        let source = msg::source();
        if self.owner != source {
            debug!("only owner can start, {:?} is not owner", source);
            return;
        }
        let block = exec::block_height();
        if self.block_next >= block {
            debug!(
                "already start, schedule in {}, should stop before start",
                self.block_next
            );
        }
        // not start, this will triger a start
        self.block_next = exec::block_height();
        self.action().await;
    }

    fn stop(&mut self) {
        let source = msg::source();
        if self.owner != source {
            debug!("only owner can stop, {:?} is not owner", source);
            return;
        }
        self.block_next = 0;
    }

    async fn transfer(&mut self) -> Result<(), QuantError> {
        debug!("transfer 0");
        for (account_id, reservation_id) in &self.reservation_ids {
            if account_id == &self.owner {
                continue;
            }
            for token_inf in self.token_info.values() {
                let payload = ft_io::FTAction::BalanceOf(account_id.clone());

                if let Ok(future) = msg::send_from_reservation_for_reply_as(
                    *reservation_id,
                    token_inf.program_id,
                    payload,
                    0,
                    REPLY_GAS_AMOUNT,
                ) {
                    match future.await {
                        Ok(event) => {
                            if let ft_io::FTEvent::Balance(balance) = event {
                                debug!(
                                    "the balance of {} for account {:?} is {}",
                                    token_inf.name, account_id, balance
                                );
                            }
                        }
                        Err(e) => {
                            debug!("error {}", e);
                        }
                    }
                } else {
                    debug!("error");
                }
            }
        }
        debug!("transfer 1");
        Ok(())
    }

    async fn action(&mut self) {
        let block = exec::block_height();
        if self.block_next != block {
            debug!("scheduled in {0} instead of {block}", self.block_next);
            return;
        }
        debug!("run action {} in block {}", self.action_id, block);

        let price = price::get_price("ocqBTC").unwrap_or_else(|e| {
            debug!("fail to get price {:?}", e);
            0
        });
        debug!("get price {price}");
        debug!("action 0");
        self.transfer().await.expect("transfer");
        debug!("action 1");
        let reservation_id = self
            .reservation_ids
            .get(&self.owner)
            .expect("can't find reservation");
        let _msg_id = msg::send_delayed_from_reservation(
            reservation_id.clone(),
            exec::program_id(),
            OcqAction::Act,
            0,
            self.block_step,
        )
        .expect("msg_send");
        self.action_id += 1;
        self.block_next = block + self.block_step;
    }

    fn reserve(&mut self) -> OcqEvent {
        let reservation_id = ReservationId::reserve(RESERVATION_AMOUNT, RESERVATION_TIME)
            .expect("reservation across executions");
        self.reservation_ids.insert(msg::source(), reservation_id);
        debug!("reserve {RESERVATION_AMOUNT} gas for {RESERVATION_TIME} blocks");
        OcqEvent::GasReserve {
            amount: RESERVATION_AMOUNT,
            time: RESERVATION_TIME,
        }
    }

    fn register_token(&mut self, info: TokenInfo) {
        self.token_info.insert(info.name.clone(), info);
    }
}

#[gstd::async_main]
async fn main() {
    let action: OcqAction = msg::load().expect("can not decode a handle action!");
    let quant: &mut OnchainQuant = unsafe { ONCHAIN_QUANT.get_or_insert(Default::default()) };
    let rply = match action {
        OcqAction::Start => {
            quant.start().await;
            OcqEvent::Start
        }
        OcqAction::Stop => {
            quant.stop();
            OcqEvent::Stop
        }
        OcqAction::Act => {
            quant.action().await;
            OcqEvent::Act
        }
        OcqAction::GasReserve => quant.reserve(),
        OcqAction::RegisterToken(toke_info) => {
            quant.register_token(toke_info);
            OcqEvent::None
        }
        OcqAction::Terminate => {
            exec::exit(quant.owner);
        }
    };
    msg::reply(rply, 0).expect("error in sending reply");
}

#[gstd::async_init]
async fn init() {
    let config: InitConfig = msg::load().expect("Unable to decode InitConfig");
    let mut token_info = HashMap::new();
    token_info.insert(
        "ocqBTC".to_owned(),
        TokenInfo {
            name: "ocqBTC".to_string(),
            // decimals: 8,
            program_id: hex!("bace93dd595a97c66e4548d88bfc96595f8b4bc8f5899b5d272e72af921e4ea9")
                .into(),
        },
    );
    token_info.insert(
        "ocqUSDT".to_owned(),
        TokenInfo {
            name: "ocqUSDT".to_string(),
            // decimals: 8,
            program_id: hex!("89c16b98b528c97d11f06f4f34871666c87634bd001d0cb9d66adea817f0a5a3")
                .into(),
        },
    );
    let quant = OnchainQuant {
        r_invest_ration: config.r_invest_ration,
        reservation_ids: HashMap::new(),
        block_step: config.block_step,
        block_next: 0,
        action_id: 0,
        owner: msg::source(),
        token_info: token_info,
    };
    unsafe { ONCHAIN_QUANT = Some(quant) };
    price::init();
}

#[no_mangle]
extern "C" fn state() {
    reply(common_state())
        .expect("Failed to encode or reply with `<AppMetadata as Metadata>::State` from `state()`");
}

fn reply(payload: impl Encode) -> GstdResult<MessageId> {
    msg::reply(payload, 0)
}

fn common_state() -> IOOnchainQuant {
    let state = static_mut_state();
    let r_invest_ration = state.r_invest_ration;
    IOOnchainQuant {
        r_invest_ration,
        block_step: state.block_step,
        block_next: state.block_next,
        action_id: state.action_id,
    }
}

fn static_mut_state() -> &'static mut OnchainQuant {
    unsafe { ONCHAIN_QUANT.get_or_insert(Default::default()) }
}
