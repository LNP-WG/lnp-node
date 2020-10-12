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

use amplify::Exec;

use super::Runtime;
use crate::error::BootstrapError;

/// Command-line commands:
#[derive(Clap, Clone, Debug, Display)]
#[display(Debug)]
pub enum Command {}

impl Exec for Command {
    type Runtime = Runtime;
    type Error = BootstrapError;

    fn exec(&self, _runtime: &mut Self::Runtime) -> Result<(), Self::Error> {
        unimplemented!()
    }
}
