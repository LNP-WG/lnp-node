use lnp_rpc::{RpcMsg, ServiceId};
use microservices::esb::ClientId;
use microservices::util::OptionDetails;

use crate::bus::{BusMsg, ServiceBus};
use crate::rpc::Failure;
use crate::Endpoints;

pub fn report_failure<E>(client_id: ClientId, endpoints: &mut Endpoints, err: E) -> Result<(), E>
where
    for<'a> &'a E: Into<Failure>,
{
    let enquirer = ServiceId::Client(client_id);
    let report = RpcMsg::Failure((&err).into());
    // Swallowing error since we do not want to break channel creation workflow just because of
    // not able to report back to the client
    let _ = endpoints
        .send_to(ServiceBus::Rpc, ServiceId::LnpBroker, enquirer, BusMsg::Rpc(report))
        .map_err(|err| error!("Can't report back to client #{}: {}", client_id, err));
    Err(err)
}

pub fn report_progress<T>(client_id: ClientId, endpoints: &mut Endpoints, msg: T)
where
    T: ToString,
{
    let enquirer = ServiceId::Client(client_id);
    let report = RpcMsg::Progress(msg.to_string());
    // Swallowing error since we do not want to break channel creation workflow just because of
    // not able to report back to the client
    let _ = endpoints
        .send_to(ServiceBus::Rpc, ServiceId::LnpBroker, enquirer, BusMsg::Rpc(report))
        .map_err(|err| error!("Can't report back to client #{}: {}", client_id, err));
}

pub fn report_success<T>(client_id: ClientId, endpoints: &mut Endpoints, msg: T)
where
    T: Into<OptionDetails>,
{
    let enquirer = ServiceId::Client(client_id);
    let report = RpcMsg::Success(msg.into());
    // Swallowing error since we do not want to break channel creation workflow just because of
    // not able to report back to the client
    let _ = endpoints
        .send_to(ServiceBus::Rpc, ServiceId::LnpBroker, enquirer, BusMsg::Rpc(report))
        .map_err(|err| error!("Can't report back to client #{}: {}", client_id, err));
}

pub fn report_progress_or_failure<T, E>(
    client_id: ClientId,
    endpoints: &mut Endpoints,
    result: Result<T, E>,
) -> Result<(), E>
where
    for<'a> &'a E: Into<Failure>,
    T: ToString,
{
    let enquirer = ServiceId::Client(client_id);
    let report = match result {
        Ok(ref val) => RpcMsg::Progress(val.to_string()),
        Err(ref err) => RpcMsg::Failure(err.into()),
    };
    // Swallowing error since we do not want to break channel creation workflow just because of
    // not able to report back to the client
    let _ = endpoints
        .send_to(ServiceBus::Rpc, ServiceId::LnpBroker, enquirer, BusMsg::Rpc(report))
        .map_err(|err| error!("Can't report back to client #{}: {}", client_id, err));
    result.map(|_| ())
}
