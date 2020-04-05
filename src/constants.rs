// Lightning network protocol (LNP) daemon
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

pub const STATE_DB_PATH: &str = "state.sqlite";
pub const MONITOR_ADDR: &str = "0.0.0.0:9666";
pub const RES_ADDR: &str = "0.0.0.0:9667";
pub const PUB_ADDR: &str = "0.0.0.0:9668";

pub const INPUT_PARSER_SOCKET: &str = "inproc://input-parser";
pub const PARSER_PUB_SOCKET: &str = "inproc://parser-input";