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

#[cfg(feature = "server")]
mod opts;
mod runtime;

#[cfg(feature = "server")]
pub use opts::Opts;
pub use runtime::run;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Display, Error)]
#[display(doc_comments)]
pub enum PaymentError {
    /// the invoice does not have amount specified; please add amount information
    AmountUnknown,

    /// there is no known route to the payee
    RouteNotFound,
}
