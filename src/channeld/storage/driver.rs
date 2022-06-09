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

use std::any::Any;

use lnp::p2p::bolt::ChannelId;

use crate::Error;

pub trait Driver {
    fn init(channel_id: ChannelId, config: Box<dyn Any>) -> Result<Self, Error>
    where
        Self: Sized;

    fn store(&mut self) -> Result<(), Error>;
}
