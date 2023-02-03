use bitcoin::PrivateKey;
use lnp::channel::bolt::Lifecycle;
use lnp::p2p::bifrost::{SwapId, SwapOutRequestMsg};
use lnp::p2p::bolt::ShortChannelId;
use lnp_rpc::{Failure, FailureCode, ServiceId};
use microservices::esb;

use super::runtime::Runtime;
use crate::automata::{Event, StateMachine};
use crate::bus::{BusMsg, CtlMsg};

/// Errors for swap workflow.
#[derive(Clone, Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// unexpected message from {2} for a swap state {1}. Message details: {0}
    UnexpectedMessage(BusMsg, SwapStateEnum, ServiceId),

    /// error sending RPC request during state transition. Details: {0}
    #[from]
    Esb(esb::Error<ServiceId>),

    /// unable to {operation} during {current_state} channel state
    InvalidState {
        operation: &'static str,
        current_state: Lifecycle,
    },

    /// swap was not persisted on a disk, so unable to reestablish
    NoPersistantData,

    /// failed to save swap state. Details: {0}
    #[from]
    Persistence(strict_encoding::Error),

    Electrum(String),
}

impl From<electrum_client::Error> for Error {
    fn from(err: electrum_client::Error) -> Self { Error::Electrum(err.to_string()) }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SwapInState {
    id: SwapId,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SwapOutState {
    id: SwapId,
    request: SwapOutRequestMsg,
    outgoing_chan_ids: Vec<ShortChannelId>,
    swap_tx_conf_requirement: u32,
    private_key: PrivateKey,
}

pub enum SwapState {
    Out(SwapOutState),
    In(SwapInState),
}

#[derive(Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Debug, Display, From)]
#[display("{0}")]
pub enum SwapStateEnum {
    Init,

    #[from]
    SwapOutSender(SwapOutSenderState),
    #[from]
    SwapOutReceiver(SwapOutReceiverState),
    #[from]
    SwapInSender(SwapInSenderState),
    #[from]
    SwapInReceiver(SwapInReceiverState),

    #[from]
    SwapCommon(SwapCommonState),
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, From)]
#[derive(StrictEncode, StrictDecode)]
pub enum SwapCommonState {
    #[display("SEND_CANCEL")]
    SendCancel,

    #[display("SWAP_CANCELED")]
    SwapCanceled,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, From)]
#[derive(StrictEncode, StrictDecode)]
pub enum SwapOutSenderState {
    #[display("CREATE_SWAP")]
    CreateSwap,

    #[display("SEND_REQUEST")]
    SendRequest,

    #[display("AWAIT_AGREEMENT")]
    AwaitAgreement,

    #[display("PAY_FEE_INVOICE")]
    PayFeeInvoice,

    #[display("AWAIT_TX_BROADCASTED_MSG")]
    AwaitTxBroadcastedMsg,

    #[display("AWAIT_TX_CONFIRMATION")]
    AwaitTxConfirmation,

    #[display("VALIDATE_TX_AND_PAY_CLAIM_INVOICE")]
    ValidateTxAndPayClaimInvoice,

    #[display("CLAIM_SWAP")]
    ClaimSwap,

    #[display("SEND_PRIV_KEY")]
    SendPrivKey,

    #[display("SEND_COOP_CLOSE")]
    SendCoopClose,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, From)]
#[derive(StrictEncode, StrictDecode)]
pub enum SwapOutReceiverState {
    #[display("CREATE_SWAP")]
    CreateSwap,

    #[display("SEND_FEE_INVOICE")]
    SendFeeInvoice,

    #[display("AWAIT_FEE_INVOICE_PAYMENT")]
    AwaitFeeInvoicePayment,

    #[display("BroadcastOpeningTx")]
    BroadcastOpeningTx,
    ///
    #[display("SendTxBroadcastedMessage")]
    SendTxBroadcastedMessage,

    #[display("AWAIT_CLAIM_INVOICE_PAYMENT")]
    AwaitClaimInvoicePayment,

    #[display("ABORTED")]
    Aborted,

    #[display("CLAIM_SWAP_CSV")]
    ClaimSwapCsv,

    #[display("CLAIM_SWAP_COOP")]
    ClaimSwapCoop,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, From)]
#[derive(StrictEncode, StrictDecode)]
pub enum SwapInSenderState {
    #[display("CREATE_SWAP")]
    CreateSwap,

    #[display("SEND_REQUEST")]
    SendRequest,

    #[display("AWAIT_AGREEMENT")]
    AwaitAgreement,

    #[display("BROADCAST_OPENING_TX")]
    BroadcastOpeningTx,

    #[display("SEND_TX_BROADCASTED_MESSAGE")]
    SendTxBroadcastedMessage,

    #[display("AWAIT_CLAIM_PAYMENT")]
    AwaitClaimPayment,

    #[display("CLAIM_SWAP_CSV")]
    ClaimSwapCoop,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, From)]
#[derive(StrictEncode, StrictDecode)]
pub enum SwapInReceiverState {
    #[display("CREATE_SWAP")]
    CreateSwap,

    #[display("SEND_AGREEMENT")]
    SendAgreement,

    #[display("AWAIT_TX_BROADCASTED_MSG")]
    AwaitTxBroadcastedMsg,

    #[display("AWAIT_TX_CONFIRMATION")]
    AwaitTxConfirmation,

    #[display("VALIDATE_TX_AND_PAY_CLAIM_INVOICE")]
    ValidateTxAndPayClaimInvoice,

    #[display("CLAIM_SWAP")]
    ClaimSwap,

    #[display("SEND_PRIV_KEY")]
    SendPrivKey,

    #[display("SEND_COOP_CLOSE")]
    SendCoopClose,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SwapStateMachine {
    id: SwapId,
    state: SwapStateEnum,
}

impl StateMachine<BusMsg, Runtime> for SwapStateMachine {
    type Error = Error;

    fn next(self, event: Event<BusMsg>, runtime: &mut Runtime) -> Result<Option<Self>, Self::Error>
    where
        Self: Sized,
    {
        debug!("SwapStateMachine {:#?} received {} event", self.id, event.message);
        if let BusMsg::Ctl(CtlMsg::Error { error, .. }) = &event.message {
            let failure = Failure { code: FailureCode::Swap, info: error.clone() };
            todo!()
        }

        match (self.state, event.message) {
            (SwapStateEnum::Init, BusMsg::Ctl(CtlMsg::DeriveKeyset(key_set))) => {
                todo!()
            }
            (SwapStateEnum::SwapOutSender(swap_out_sender), BusMsg::Rpc(rpc)) => {
                todo!()
            }
            _ => todo!(),
        }
    }
}
