// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020 by
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

use std::convert::TryFrom;
use std::fmt::{self, Debug, Display, Formatter};
use std::net::SocketAddr;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Child, ExitStatus};
use std::{process, thread};

use amplify::hex::ToHex;
use amplify::IoError;
use internet2::RemoteSocketAddr;
use lnp::p2p::legacy::ActiveChannelId;

use crate::lnpd::runtime::Runtime;
use crate::peerd::PeerSocket;
use crate::{channeld, peerd, signd, Config, Error};

// TODO: Move `DaemonHandle` to microservices crate
/// Handle for a daemon launched by LNPd
#[derive(Debug)]
pub enum DaemonHandle<DaemonName: Debug + Display + Clone> {
    /// Daemon launched as a separate process
    Process(DaemonName, process::Child),

    /// Daemon launched as a thread
    Thread(DaemonName, thread::JoinHandle<Result<(), Error>>),
}

impl<DaemonName: Debug + Display + Clone> Display for DaemonHandle<DaemonName> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DaemonHandle::Process(name, child) => write!(f, "{} PID #{}", name, child.id()),
            DaemonHandle::Thread(name, handle) => {
                write!(f, "{} {:?}", name, handle.thread().id())
            }
        }
    }
}

/// Errors during daemon launching
#[derive(Debug, Error, Display, From)]
#[display(doc_comments)]
pub enum DaemonError<DaemonName: Debug + Display + Clone> {
    /// thread `{0}` has exited with an error.
    ///
    /// Error details: {1}
    ThreadAborted(DaemonName, Error),

    /// thread `{0}` failed to launch due to I/O error {1}
    ThreadLaunch(DaemonName, IoError),

    /// thread `{0}` failed to launch
    ThreadJoin(DaemonName),

    /// process `{0}` has existed with a non-zero exit status {1}
    ProcessAborted(DaemonName, ExitStatus),

    /// process `{0}` failed to launch due to I/O error {1}
    ProcessLaunch(DaemonName, IoError),
}

impl<DaemonName: Debug + Display + Clone> DaemonHandle<DaemonName> {
    /// Waits for daemon execution completion on the handler.
    ///
    /// # Returns
    ///
    /// On error or upon thread/process successful completion. For process this means that the
    /// process has exited with status 0.
    ///
    /// # Errors
    /// - if the thread failed to start;
    /// - if it failed to join the thread;
    /// - if the process exit status was not 0
    fn join(self) -> Result<(), DaemonError<DaemonName>> {
        match self {
            DaemonHandle::Process(name, mut proc) => proc
                .wait()
                .map_err(|io| DaemonError::ProcessLaunch(name.clone(), io.into()))
                .and_then(|status| {
                    if status.success() {
                        Ok(())
                    } else {
                        Err(DaemonError::ProcessAborted(name, status))
                    }
                }),
            DaemonHandle::Thread(name, thread) => thread
                .join()
                .map_err(|_| DaemonError::ThreadJoin(name.clone()))?
                .map_err(|err| DaemonError::ThreadAborted(name, err)),
        }
    }
}

/// Daemons that can be launched by lnpd
#[derive(Clone, Eq, PartialEq, Debug, Display)]
pub enum Daemon {
    #[display("signd")]
    Signd,

    #[display("peerd")]
    Peerd(PeerSocket, PathBuf),

    #[display("channeld")]
    Channeld(ActiveChannelId),

    #[display("routed")]
    Routed,

    #[display("gossipd")]
    Gossipd,
}

impl Daemon {
    pub fn bin_name(&self) -> &'static str {
        match self {
            Daemon::Signd => "signd",
            Daemon::Peerd(..) => "peerd",
            Daemon::Channeld(..) => "channeld",
            Daemon::Routed => "routed",
            Daemon::Gossipd => "gossipd",
        }
    }
}

impl Runtime {
    fn thread_daemon(
        &self,
        daemon: Daemon,
        config: Config,
    ) -> Result<thread::JoinHandle<Result<(), Error>>, DaemonError<Daemon>> {
        debug!("Spawning {} as a new thread", daemon);

        let builder = thread::Builder::new().name(daemon.to_string());
        Ok(match daemon.clone() {
            Daemon::Signd => thread::spawn(move || signd::run(config)),
            Daemon::Peerd(socket, key_file) => builder
                .spawn(move || peerd::supervisor::run(config, &key_file, socket))
                .map_err(|io| DaemonError::ThreadLaunch(daemon, io.into()))?,
            Daemon::Channeld(channel_id) => builder
                .spawn(move || channeld::run(config, channel_id))
                .map_err(|io| DaemonError::ThreadLaunch(daemon, io.into()))?,
            Daemon::Routed => todo!(),
            Daemon::Gossipd => todo!(),
        })
    }

    fn exec_daemon(&self, daemon: Daemon) -> Result<Child, DaemonError<Daemon>> {
        let mut bin_path = std::env::current_exe().map_err(|err| {
            error!("Unable to detect binary directory: {}", err);
            DaemonError::ProcessLaunch(daemon.clone(), err.into())
        })?;
        bin_path.pop();
        bin_path.push(daemon.bin_name());
        #[cfg(target_os = "windows")]
        bin_path.set_extension("exe");

        debug!(
            "Launching {} as a separate process using `{}` as binary",
            daemon.clone(),
            bin_path.display()
        );

        let mut cmd = process::Command::new(bin_path);
        cmd.args(std::env::args().skip(1).filter(|arg| !arg.starts_with("--listen")));

        match &daemon {
            Daemon::Peerd(PeerSocket::Listen(RemoteSocketAddr::Ftcp(inet)), _) => {
                let socket_addr =
                    SocketAddr::try_from(inet.clone()).expect("invalid connection address");
                let ip = socket_addr.ip();
                let port = socket_addr.port();
                cmd.args(&["--listen", &ip.to_string(), "--port", &port.to_string()]);
            }
            Daemon::Peerd(PeerSocket::Connect(node_addr), _) => {
                cmd.args(&["--connect", &node_addr.to_string()]);
            }
            Daemon::Peerd(PeerSocket::Listen(_), _) => {
                // Lightning do not support non-TCP sockets
                DaemonError::ProcessAborted(daemon.clone(), ExitStatus::from_raw(101));
            }
            Daemon::Channeld(channel_id, ..) => {
                cmd.args(&[channel_id.as_slice32().to_hex()]);
                if channel_id.channel_id().is_some() {
                    cmd.args(&["--reestablish"]);
                }
            }
            _ => { /* No additional configuration is required here */ }
        }

        trace!("Executing `{:?}`", cmd);
        cmd.spawn().map_err(|err| {
            error!("Error launching {}: {}", daemon.clone(), err);
            DaemonError::ProcessLaunch(daemon, err.into())
        })
    }

    pub(super) fn launch_daemon(
        &self,
        daemon: Daemon,
        config: Config,
    ) -> Result<DaemonHandle<Daemon>, DaemonError<Daemon>> {
        if self.config.threaded {
            Ok(DaemonHandle::Thread(daemon.clone(), self.thread_daemon(daemon, config)?))
        } else {
            Ok(DaemonHandle::Process(daemon.clone(), self.exec_daemon(daemon)?))
        }
    }
}
