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
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use std::time::Duration;
use std::{fs, thread};

use bitcoin::secp256k1::PublicKey;
use internet2::addr::InetSocketAddr;
use internet2::{session, LocalNode, LocalSocketAddr, NodeAddr, RemoteNodeAddr, RemoteSocketAddr};
use microservices::peer::PeerConnection;
use nix::unistd::{fork, ForkResult, Pid};
use strict_encoding::StrictDecode;

use super::runtime;
use crate::peerd::PeerSocket;
use crate::{Config, Error, LogStyle};

#[derive(Clone, Debug)]
pub(super) struct RuntimeParams {
    pub config: Config,
    pub id: NodeAddr,
    pub local_id: PublicKey,
    pub remote_id: Option<PublicKey>,
    pub local_socket: Option<InetSocketAddr>,
    pub remote_socket: InetSocketAddr,
    pub connect: bool,
}

impl RuntimeParams {
    fn with(config: Config, local_id: PublicKey) -> RuntimeParams {
        RuntimeParams {
            config,
            id: NodeAddr::Local(LocalSocketAddr::Posix(s!(""))),
            local_id,
            remote_id: None,
            local_socket: None,
            remote_socket: Default::default(),
            connect: false,
        }
    }
}

pub fn read_node_key_file(key_file: &Path) -> LocalNode {
    let local_node = LocalNode::strict_decode(fs::File::open(key_file).expect(&format!(
        "Unable to open key file '{}';\nplease check that the file exists and the daemon has \
         access rights to it",
        key_file.display()
    )))
    .expect(&format!("Unable understand format of node key file '{}'", key_file.display()));

    let local_id = local_node.node_id();
    info!("{}: {}", "Local node id".ended(), local_id.addr());

    local_node
}

pub fn run(config: Config, key_file: &Path, peer_socket: PeerSocket) -> Result<(), Error> {
    debug!("Peer socket parameter interpreted as {}", peer_socket);

    let local_node = read_node_key_file(key_file);

    let threaded = config.threaded;
    let mut params = RuntimeParams::with(config, local_node.node_id());
    match peer_socket {
        PeerSocket::Listen(RemoteSocketAddr::Ftcp(inet_addr)) => {
            debug!("Running in LISTEN mode");

            params.connect = false;
            params.local_socket = Some(inet_addr);
            params.id = NodeAddr::Remote(RemoteNodeAddr {
                node_id: local_node.node_id(),
                remote_addr: RemoteSocketAddr::Ftcp(inet_addr),
            });

            spawner(params, inet_addr, threaded)?;
        }
        PeerSocket::Connect(remote_node_addr) => {
            debug!("Running in CONNECT mode");

            params.connect = true;
            params.id = NodeAddr::Remote(remote_node_addr.clone());
            params.remote_id = Some(remote_node_addr.node_id);
            params.remote_socket = remote_node_addr.remote_addr.into();

            info!("Connecting to {}", &remote_node_addr);
            let connection = PeerConnection::connect(remote_node_addr, &local_node)
                .expect("Unable to connect to the remote peer");
            runtime::run(connection, params)?;
        }
        PeerSocket::Listen(_) => {
            unimplemented!("we do not support non-TCP connections for the legacy lightning network")
        }
    }

    unreachable!()
}

pub enum Handler {
    Thread(JoinHandle<Result<(), Error>>),
    Process(Pid),
}

fn spawner(
    mut params: RuntimeParams,
    inet_addr: InetSocketAddr,
    threaded_daemons: bool,
) -> Result<(), Error> {
    // Handlers for all of our spawned processes and threads
    let mut handlers = vec![];

    debug!("Binding TCP socket {}", inet_addr);
    let listener =
        TcpListener::bind(SocketAddr::try_from(inet_addr).expect("Tor is not yet supported"))
            .expect("Unable to bind to Lightning network peer socket");

    debug!("Running TCP listener event loop");
    let stream = loop {
        debug!("Awaiting for incoming connections...");
        let (stream, remote_socket_addr) =
            listener.accept().expect("Error accepting incpming peer connection");
        debug!("New connection from {}", remote_socket_addr);

        params.remote_socket = remote_socket_addr.into();

        if threaded_daemons {
            debug!("Spawning child thread");
            let child_params = params.clone();
            let handler = thread::spawn(move || {
                debug!("Establishing session with the remote");
                let session = session::Raw::with_ftcp_unencrypted(stream, inet_addr)
                    .expect("Unable to establish session with the remote peer");
                let connection = PeerConnection::with(session);
                runtime::run(connection, child_params)
            });
            handlers.push(Handler::Thread(handler));
            // We have started the thread so awaiting for the next incoming connection
        } else {
            debug!("Forking child process");
            if let ForkResult::Parent { child } =
                unsafe { fork().expect("Unable to fork child process") }
            {
                handlers.push(Handler::Process(child));
                debug!("Child forked with pid {}; returning into main listener event loop", child);
            } else {
                break stream; // We are in the child process and need to proceed with incoming
                              // connection
            }
        }
        trace!("Total {} peerd are spawned for the incoming connections", handlers.len());
    };

    // Here we get only in the child process forked from the parent
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("Unable to set up timeout for TCP connection");

    debug!("Establishing session with the remote");
    let session = session::Raw::with_ftcp_unencrypted(stream, inet_addr)
        .expect("Unable to establish session with the remote peer");

    debug!("Session successfully established");
    let connection = PeerConnection::with(session);
    runtime::run(connection, params)?;

    unreachable!()
}
