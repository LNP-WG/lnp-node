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

use std::os::raw::{c_char, c_double, c_uchar};

use super::Client;

#[derive(Default)]
#[repr(C)]
pub struct CError {
    code: u32,
    message: *const c_char,
}

#[repr(C)]
pub struct CResult<T> {
    success: bool,
    payload: T,
    error: CError,
}

#[no_mangle]
pub extern "C" fn lnp_client_connect() -> CResult<Client> {}

#[no_mangle]
pub extern "C" fn lnp_client_send() -> CResult {}
