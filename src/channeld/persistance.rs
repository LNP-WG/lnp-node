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

// TODO: Move to LNP/BP Core Library

use std::collections::HashMap;

use lnpbp::bitcoin::util::psbt::PartiallySignedTransaction as Psbt;

pub enum Error {}

/// Marker trait defining transaction role in a graph
pub trait TxRole {}

/// Marker trait defining public key role in a graph
pub trait PkRole {}

pub trait History {
    type State;
    type Error: std::error::Error;

    fn height(&self) -> usize;
    fn get(&self, height: usize) -> Result<Self::State, Self::Error>;
    fn top(&self) -> Result<Self::State, Self::Error>;
    fn bottom(&self) -> Result<Self::State, Self::Error>;
    fn dig(&self) -> Result<Self::State, Self::Error>;
    fn push(&mut self, state: Self::State) -> Result<&mut Self, Self::Error>;
}

/// API for working with persistent transaction graphs
pub trait TxGraph {
    type Roles: TxRole;

    fn get<'a>(
        &'a self,
        role: impl TxRole,
    ) -> Result<Vec<TxNode<'a, Roles>>, Error>;
}

pub struct TxNode<'a, R>
where
    R: TxRole,
{
    psbt: Psbt,
    edges: HashMap<R, &'a Vec<Psbt>>,
}

impl<'a, R> TxNode<'a, R>
where
    R: TxRole,
{
    #[inline]
    pub fn as_psbt(&'a self) -> &'a Psbt {
        &self.psbt
    }

    #[inline]
    pub fn into_psbt(self) -> Psbt {
        self.psbt
    }

    #[inline]
    pub fn next(&'a self, role: &R, index: usize) -> Option<TxNode<'a, R>> {
        self.edges.get(role).and_then(|vec| (*vec).get(index))
    }
}
