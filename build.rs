use clap::IntoApp;
use clap_generate::{generate_to, generators::*};

pub mod opts {
    include!("src/opts.rs");
}

include!("src/connectiond/opts.rs");

fn main() {
    let outdir = "./shell";

    let mut app = Opts::into_app();
    let name = app.get_name().to_string();
    generate_to::<Bash, _, _>(&mut app, &name, &outdir);
    generate_to::<PowerShell, _, _>(&mut app, &name, &outdir);
    generate_to::<Zsh, _, _>(&mut app, &name, &outdir);
}
