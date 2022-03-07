// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use internet2::{zmqsocket, ZmqType};
use microservices::esb;
use microservices::node::TryService;

use crate::bus::{self, BusMsg, CtlMsg, Report, ServiceBus};
use crate::rpc::{Failure, ServiceId};
use crate::{Config, Error};

pub struct Service<Runtime>
where
    Runtime: esb::Handler<ServiceBus, Request = BusMsg>,
    esb::Error<ServiceId>: From<Runtime::Error>,
{
    esb: esb::Controller<ServiceBus, BusMsg, Runtime>,
    broker: bool,
}

impl<Runtime> Service<Runtime>
where
    Runtime: esb::Handler<ServiceBus, Request = BusMsg>,
    esb::Error<ServiceId>: From<Runtime::Error>,
{
    pub fn run(config: Config, runtime: Runtime, broker: bool) -> Result<(), Error> {
        let service = Self::with(config, runtime, broker)?;
        service.run_loop()?;
        unreachable!()
    }

    fn with(config: Config, runtime: Runtime, broker: bool) -> Result<Self, esb::Error<ServiceId>> {
        let router = if !broker { Some(ServiceId::router()) } else { None };
        let services = map! {
            ServiceBus::Msg => esb::BusConfig::with_locator(
                config.msg_endpoint,
                router.clone()
            ),
            ServiceBus::Ctl => esb::BusConfig::with_locator(
                config.ctl_endpoint,
                router.clone()
            ),
            ServiceBus::Rpc => esb::BusConfig::with_locator(config.rpc_endpoint, router)
        };
        let esb = esb::Controller::with(
            services,
            runtime,
            if broker { ZmqType::RouterBind } else { ZmqType::RouterConnect },
        )?;
        Ok(Self { esb, broker })
    }

    pub fn broker(config: Config, runtime: Runtime) -> Result<Self, esb::Error<ServiceId>> {
        Self::with(config, runtime, true)
    }

    #[allow(clippy::self_named_constructors)]
    pub fn service(config: Config, runtime: Runtime) -> Result<Self, esb::Error<ServiceId>> {
        Self::with(config, runtime, false)
    }

    pub fn is_broker(&self) -> bool { self.broker }

    pub fn add_loopback(&mut self, socket: zmq::Socket) -> Result<(), esb::Error<ServiceId>> {
        self.esb.add_service_bus(ServiceBus::Bridge, esb::BusConfig {
            carrier: zmqsocket::Carrier::Socket(socket),
            router: None,
            queued: true,
        })
    }

    pub fn run_loop(mut self) -> Result<(), Error> {
        if !self.is_broker() {
            std::thread::sleep(core::time::Duration::from_secs(1));
            self.esb.send_to(ServiceBus::Ctl, ServiceId::LnpBroker, BusMsg::Ctl(CtlMsg::Hello))?;
            // self.esb.send_to(ServiceBus::Msg, ServiceId::Lnpd, BusMsg::Ctl(CtlMsg::Hello))?;
        }

        let identity = self.esb.handler().identity();
        info!("{} started", identity);

        self.esb.run_or_panic(&identity.to_string());

        unreachable!()
    }
}

pub type Endpoints = esb::EndpointList<ServiceBus>;

pub trait TryToServiceId {
    fn try_to_service_id(&self) -> Option<ServiceId>;
}

impl TryToServiceId for ServiceId {
    fn try_to_service_id(&self) -> Option<ServiceId> { Some(self.clone()) }
}

impl TryToServiceId for &Option<ServiceId> {
    fn try_to_service_id(&self) -> Option<ServiceId> { (*self).clone() }
}

impl TryToServiceId for Option<ServiceId> {
    fn try_to_service_id(&self) -> Option<ServiceId> { self.clone() }
}

pub trait Responder
where
    Self: esb::Handler<ServiceBus>,
    esb::Error<ServiceId>: From<Self::Error>,
{
    /// Returns client which should receive status update reports
    #[inline]
    fn enquirer(&self) -> Option<ClientId> { None }

    fn report_success(
        &mut self,
        endpoints: &mut Endpoints,
        msg: Option<impl ToString>,
    ) -> Result<(), Error> {
        if let Some(ref message) = msg {
            info!("{}", message.to_string());
        }
        if let Some(client) = self.enquirer() {
            let status = bus::Status::Success(msg.map(|m| m.to_string()).into());
            let report = CtlMsg::Report(Report { client, status });
            endpoints.send_to(
                ServiceBus::Ctl,
                self.identity(),
                ServiceId::LnpBroker,
                BusMsg::Ctl(report),
            )?;
        }
        Ok(())
    }

    fn report_progress(
        &mut self,
        endpoints: &mut Endpoints,
        msg: impl ToString,
    ) -> Result<(), Error> {
        let msg = msg.to_string();
        info!("{}", msg);
        if let Some(client) = self.enquirer() {
            let status = bus::Status::Progress(msg);
            let report = CtlMsg::Report(Report { client, status });
            endpoints.send_to(
                ServiceBus::Ctl,
                self.identity(),
                ServiceId::LnpBroker,
                BusMsg::Ctl(report),
            )?;
        }
        Ok(())
    }

    fn report_failure(&mut self, endpoints: &mut Endpoints, failure: impl Into<Failure>) -> Error {
        let failure = failure.into();
        if let Some(client) = self.enquirer() {
            let status = bus::Status::Failure(failure.clone());
            let report = CtlMsg::Report(Report { client, status });
            // Even if we fail, we still have to terminate :)
            let _ = endpoints.send_to(
                ServiceBus::Ctl,
                self.identity(),
                ServiceId::LnpBroker,
                BusMsg::Ctl(report),
            );
        }
        Error::Terminate(failure.to_string())
    }

    fn send_ctl(
        &mut self,
        endpoints: &mut Endpoints,
        dest: impl TryToServiceId,
        request: CtlMsg,
    ) -> Result<(), esb::Error<ServiceId>> {
        if let Some(dest) = dest.try_to_service_id() {
            endpoints.send_to(ServiceBus::Ctl, self.identity(), dest, BusMsg::Ctl(request))?;
        }
        Ok(())
    }

    #[inline]
    fn send_rpc(
        &self,
        endpoints: &mut Endpoints,
        client_id: ClientId,
        message: impl Into<RpcMsg>,
    ) -> Result<(), esb::Error<ServiceId>> {
        endpoints.send_to(
            ServiceBus::Rpc,
            self.identity(),
            ServiceId::Client(client_id),
            BusMsg::Rpc(message.into()),
        )
    }
}

// TODO: Move to LNP/BP Services library
use colored::Colorize;
use lnp_rpc::{ClientId, RpcMsg};

pub trait LogStyle: ToString {
    fn promo(&self) -> colored::ColoredString { self.to_string().bold().bright_blue() }

    fn promoter(&self) -> colored::ColoredString { self.to_string().italic().bright_blue() }

    fn action(&self) -> colored::ColoredString { self.to_string().bold().yellow() }

    fn progress(&self) -> colored::ColoredString { self.to_string().bold().green() }

    fn ended(&self) -> colored::ColoredString { self.to_string().bold().bright_green() }

    fn ender(&self) -> colored::ColoredString { self.to_string().italic().bright_green() }

    fn amount(&self) -> colored::ColoredString { self.to_string().bold().bright_yellow() }

    fn addr(&self) -> colored::ColoredString { self.to_string().bold().bright_yellow() }

    fn err(&self) -> colored::ColoredString { self.to_string().bold().bright_red() }

    fn err_details(&self) -> colored::ColoredString { self.to_string().bold().red() }
}

impl<T> LogStyle for T where T: ToString {}
