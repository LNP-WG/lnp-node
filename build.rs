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

use clap::IntoApp;
use clap_generate::{generate_to, generators::*};

pub mod opts {
    include!("src/opts.rs");
}

include!("src/connectiond/opts.rs");

fn main() -> Result<(), configure_me_codegen::Error> {
    let outdir = "./shell";

    let mut app = Opts::into_app();
    let name = app.get_name().to_string();
    generate_to::<Bash, _, _>(&mut app, &name, &outdir);
    generate_to::<PowerShell, _, _>(&mut app, &name, &outdir);
    generate_to::<Zsh, _, _>(&mut app, &name, &outdir);

    configure_me_codegen::build_script_auto()
}
