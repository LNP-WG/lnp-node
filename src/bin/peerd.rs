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

#![recursion_limit = "256"]
// Coding conventions
#![deny(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    unused_mut,
    unused_imports,
    dead_code,
    missing_docs
)]

//! Main executable for peerd: lightning peer network connection
//! microservice.
//!
//! Program operations
//! ==================
//!
//! Bootstrapping
//! -------------
//!
//! Since this daemon must operate external P2P TCP socket, and TCP socket can
//! be either connected to the remote or accept remote connections; and we need
//! a daemon per connection, while the incoming TCP socket can't be transferred
//! between processes using IPC, the only option here is to have two special
//! cases of the daemon.
//!
//! The first one will open TCP socket in listening mode and wait for incoming
//! connections, forking on each one of them, passing the accepted TCP socket to
//! the child and continuing on listening to the new connections. (In
//! multi-thread mode, differentiated with `--threaded` argument, instead of
//! forking damon will launch a new thread).
//!
//! The second one will be launched by some control process and then commanded
//! (through command API) to connect to a specific remote TCP socket.
//!
//! These two cases are differentiated by a presence command-line option
//! `--listen` followed by a listening address to bind (IPv4, IPv6, Tor and TCP
//! port number) or `--connect` option followed by a remote address in the same
//! format.
//!
//! Runtime
//! -------
//!
//! The overall program logic thus is the following:
//!
//! In the process starting from `main()`:
//! - Parse cli arguments into a config. There is no config file, since the daemon can be started
//!   only from another control process (`lnpd`) or by forking itself.
//! - If `--listen` argument is present, start a listening version as described above and open TCP
//!   port in listening mode; wait for incoming connections
//! - If `--connect` argument is present, connect to the remote TCP peer
//!
//! In forked/spawned version:
//! - Acquire connected TCP socket from the parent
//!
//! From here, all actions must be taken by either forked version or by a daemon
//! launched from the control process:
//! - Split TCP socket and related transcoders into reading and writing parts
//! - Create bridge ZMQ PAIR socket
//! - Put both TCP socket reading ZMQ bridge write PAIR parts into a thread ("bridge")
//! - Open control interface socket
//! - Create run loop in the main thread for polling three ZMQ sockets:
//!   * control interface
//!   * LN P2P messages from intranet
//!   * bridge socket
//!
//! Node key
//! --------
//!
//! Node key, used for node identification and in generation of the encryption
//! keys, is read from the file specified in `--key-file` parameter, or (if the
//! parameter is absent) from `LNP_NODE_KEY_FILE` environment variable.

#[macro_use]
extern crate log;

use std::path::PathBuf;

use clap::Parser;
use internet2::session::noise::FramingProtocol;
use lnp_node::lnpd::read_node_key_file;
use lnp_node::peerd::{self, Opts};
use lnp_node::{Config, P2pProtocol};

/*
mod internal {
    #![allow(unused)]
    include!(concat!(env!("OUT_DIR"), "/configure_me_config.rs"));
}
 */

fn main() {
    println!("peerd: lightning peer network connection microservice");

    let mut opts = Opts::parse();
    trace!("Command-line arguments: {:?}", &opts);
    opts.process();
    trace!("Processed arguments: {:?}", &opts);

    let config: Config<peerd::Config> = opts.clone().into();
    trace!("Daemon configuration: {:?}", &config);
    debug!("MSG RPC socket {}", &config.msg_endpoint);
    debug!("CTL RPC socket {}", &config.ctl_endpoint);

    /*
    use self::internal::ResultExt;
    let (config_from_file, _) =
        internal::Config::custom_args_and_optional_files(std::iter::empty::<
            &str,
        >())
        .unwrap_or_exit();
     */

    let key_file = PathBuf::from(opts.key_opts.key_file.clone());
    let local_node = read_node_key_file(&key_file);
    let peer_socket = opts.peer_socket(local_node.node_id());
    let framing_protocol = match config.ext.protocol {
        P2pProtocol::Bolt => FramingProtocol::Brontide,
        P2pProtocol::Bifrost => FramingProtocol::Brontozaur,
    };

    debug!("Starting runtime ...");
    let threaded = config.threaded;
    microservices::peer::supervisor::run(
        config,
        threaded,
        framing_protocol,
        local_node,
        peer_socket,
        peerd::runtime::run,
    )
    .expect("Error running peerd runtime");

    unreachable!()
}
