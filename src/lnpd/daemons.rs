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

use std::fmt::{Debug, Display};
use std::process::ExitStatus;
use std::{process, thread};

use amplify::IoError;

use crate::Error;

// TODO: Move `DaemonHandle` to microservices crate
/// Handle for a daemon launched by LNPd
#[derive(Debug)]
pub enum DaemonHandle<DaemonName: Display + Clone> {
    /// Daemon launched as a separate process
    Process(DaemonName, process::Child),

    /// Daemon launched as a thread
    Thread(DaemonName, thread::JoinHandle<Result<(), Error>>),
}

/// Errors during daemon launching
#[derive(Debug, Error, Display, From)]
#[display(doc_comments)]
pub enum DaemonError<DaemonName: Debug + Display + Clone> {
    /// thread `{0}` has exited with an error.
    ///
    /// Error details: {1}
    ThreadAborted(DaemonName, Error),

    /// thread `{0}` failed to launch
    ThreadLaunch(DaemonName),

    /// process `{0}` has existed with a non-zero exit status {1}
    ProcessAborted(DaemonName, ExitStatus),

    /// I/O error {1} during process `{0}` execution
    ProcessLaunch(DaemonName, IoError),
}

impl<DaemonName: Debug + Display + Clone> DaemonHandle<DaemonName> {
    /// Waits for daemon execution completion on the handler.
    ///
    /// # Returns
    ///
    /// On error or upon thread/process successul completion. For process this means that the
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
                .map_err(|_| DaemonError::ThreadLaunch(name.clone()))?
                .map_err(|err| DaemonError::ThreadAborted(name, err)),
        }
    }
}

/// Daemons that can be launched by lnpd
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
pub(super) enum Daemon {
    #[display("signd")]
    Signd,

    #[display("peerd")]
    Peerd,

    #[display("channeld")]
    Channeld,

    #[display("routed")]
    Routed,

    #[display("gossipd")]
    Gossipd,
}

impl Daemon {
    pub fn bin_name(self) -> &'static str {
        match self {
            Daemon::Signd => "signd",
            Daemon::Peerd => "peerd",
            Daemon::Channeld => "channeld",
            Daemon::Routed => "routed",
            Daemon::Gossipd => "gossipd",
        }
    }
}
