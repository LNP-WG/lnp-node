// Keyring: private/public key managing service
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the AGPL License
// along with this software.
// If not, see <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

use ::std::io;
#[cfg(feature = "shell")]
use settings::ConfigError;
#[cfg(feature = "daemon")]
use tokio::task::JoinError;

use lnpbp::lnp;

#[cfg(feature = "shell")]
#[derive(Debug, Display, Error, From)]
#[display(Debug)]
pub enum ConfigInitError {
    #[from]
    IoError(io::Error),

    #[from]
    Toml(toml::ser::Error),
}

#[derive(Debug, Display, Error, From)]
#[display(Debug)]
pub enum BootstrapError {
    #[cfg(feature = "shell")]
    #[from]
    ConfigError(ConfigError),

    TorNotYetSupported,

    #[from]
    IoError(io::Error),

    #[from]
    ArgParseError(String),

    #[from]
    ZmqSocketError(zmq::Error),

    #[cfg(feature = "daemon")]
    #[from]
    MultithreadError(JoinError),

    #[cfg(feature = "monitoring")]
    MonitorSocketError(Box<dyn std::error::Error + Send>),

    #[from]
    TransportError(lnp::transport::Error),

    #[cfg(feature = "daemon")]
    ConfigInitError,

    Other,
}

impl From<BootstrapError> for String {
    fn from(err: BootstrapError) -> Self {
        format!("{}", err)
    }
}

impl From<&str> for BootstrapError {
    fn from(err: &str) -> Self {
        BootstrapError::ArgParseError(err.to_string())
    }
}

#[derive(Debug, Display, Error, From)]
#[display(Debug)]
pub enum RuntimeError {
    #[from(lnp::transport::Error)]
    Transport,

    #[from(lnp::presentation::Error)]
    Message,
}
