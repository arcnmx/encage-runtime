#[cfg(feature = "ocf")]
extern crate ocf;
extern crate encage_runtime as run;

use std::process::exit;

fn main() {
    #[cfg(feature = "ocf")]
    fn run_ocf() {
        let (spec, runtime) = ocf::load(".").expect("failed to load config");
        let (config, cmd) = run::ocf_config(&spec, &runtime).expect("failed to process config");
        let status = run::run(&config, &cmd).expect("failed to run");
        exit(status);
    }

    #[cfg(not(feature = "ocf"))]
    fn run_ocf() {
        panic!("Not built with OCF support");
    }

    run_ocf();
}
