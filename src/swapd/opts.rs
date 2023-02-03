use crate::opts::Options;

#[derive(Parser, Clone, PartialEq, Eq, Debug)]
#[clap(name = "watchd", bin_name = "watchd", author, version)]
pub struct Opts {
    /// These params can be read also from the configuration file, not just
    /// command-line args or environment variables
    #[clap(flatten)]
    pub shared: crate::opts::Opts,
}

impl Options for Opts {
    type Conf = ();

    fn shared(&self) -> &crate::opts::Opts { &self.shared }

    fn config(&self) -> Self::Conf { () }
}

impl Opts {
    pub fn process(&mut self) { self.shared.process() }
}
