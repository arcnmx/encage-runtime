#[cfg(feature = "ocf")]
extern crate ocf;
#[macro_use]
extern crate clap;

extern crate encage_runtime as run;

use std::process::exit;
use clap::{App, AppSettings};

fn main() {
    let app = App::new("encage-run").setting(AppSettings::SubcommandRequiredElseHelp);
    let app = clap_app! { @app (app)
        (author: "arcnmx")
        (about: "Encage Linux container runtime")
    };

    #[cfg(not(feature = "ocf"))]
    fn ocf_args<'a, 'b, 'c, 'd, 'e, 'f>(app: App<'a, 'b, 'c, 'd, 'e, 'f>) -> App<'a, 'b, 'c, 'd, 'e, 'f> { app }
    #[cfg(feature = "ocf")]
    fn ocf_args<'a, 'b, 'c, 'd, 'e, 'f>(app: App<'a, 'b, 'c, 'd, 'e, 'f>) -> App<'a, 'b, 'c, 'd, 'e, 'f> {
        clap_app! { @app (app)
            (@subcommand runc =>
                (about: "Runs an OCF-compliant image")
                (@arg PATH: "A directory containing the OCF image")
            )
        }
    }
    let app = ocf_args(app);
    let matches = app.get_matches();

    match matches.subcommand() {
        #[cfg(feature = "ocf")]
        ("runc", Some(matches)) => {

            let path = matches.value_of("PATH");
            let path = path.map(|s| &s[..]).unwrap_or(".");

            let (spec, runtime) = ocf::load(path).expect("failed to load config");
            let (config, cmd) = run::ocf_config(&spec, &runtime).expect("failed to process config");
            let status = run::run(&config, &cmd).expect("failed to run");
            exit(status);
        },
        _ => unreachable!()
    }
}
