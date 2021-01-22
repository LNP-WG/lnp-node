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

use std::any::Any;
use std::path::PathBuf;

use lnp::ChannelId;

use super::Driver;
use crate::Error;

pub struct DiskConfig {
    pub path: PathBuf,
}

pub struct DiskDriver {
    channel_id: ChannelId,
    config: DiskConfig,
}

impl Driver for DiskDriver {
    fn init(
        channel_id: ChannelId,
        config: Box<dyn Any>,
    ) -> Result<Self, Error> {
        let config = *config.downcast().map_err(|_| Error::Other(s!("")))?;
        Ok(Self { channel_id, config })
    }

    fn store(&mut self) -> Result<(), Error> {
        unimplemented!()
    }
}
