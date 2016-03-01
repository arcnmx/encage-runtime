#[cfg(feature = "ocf")]
extern crate ocf;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate nix;
extern crate tempdir;

extern crate encage_runtime as run;

use clap::{App, AppSettings};
use tempdir::TempDir;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;
use std::ffi::{CString, NulError};
use std::process::exit;
use std::{env, fs};

#[derive(Debug)]
struct MountArg<'a> {
    mounts: Vec<&'a str>,
    options: Vec<&'a str>,
}

impl<'a> MountArg<'a> {
    fn last_mount(&self) -> &'a str {
        *self.mounts.last().unwrap()
    }

    fn contains_option(&self, opt: &str) -> bool {
        self.options.iter().any(|&o| o == opt)
    }

    fn option_value(&self, opt: &str) -> Option<&'a str> {
        self.options.iter()
            .find(|o| o.starts_with(opt))
            .map(|o| &o[opt.len()..])
    }
}

fn rootless(mut str: &str) -> &str {
    while str.starts_with('/') {
        str = &str[1..];
    }

    str
}

fn mount_bind<'a>(mount: &MountArg<'a>, join: Option<&str>) -> Result<run::Mount<'static>, NulError> {
    let readonly = if mount.contains_option("ro") { nix::mount::MS_RDONLY } else { nix::mount::MsFlags::empty() };
    let propagation = nix::mount::MS_SLAVE;

    let dest = if let Some(join) = join {
        let dest = Path::new(join).join(rootless(mount.last_mount()));
        try!(CString::new(dest.into_os_string().into_vec()))
    } else {
        try!(CString::new(mount.last_mount().as_bytes()))
    };

    let len = mount.mounts.len();
    if len > 2 {
        let (lowers, upper) = if readonly != nix::mount::MsFlags::empty() {
            (&mount.mounts[..len - 1], None)
        } else {
            (&mount.mounts[..len - 1], Some(mount.mounts[len - 2]))
        };

        let mut options = format!("lowerdir={}", lowers[0]);
        for lower in &lowers[1..] {
            options.push(':');
            options.push_str(lower);
        }
        if let Some(upper) = upper {
            options.push_str(",upperdir=");
            options.push_str(upper);
        }
        if let Some(workdir) = mount.option_value("workdir=") {
            fs::create_dir_all(workdir).expect("failed to create workdir");
            options.push_str(",workdir=");
            options.push_str(workdir);
        }
        let options = try!(CString::new(options));

        let overlay = Some(cstr!("overlay"));
        Ok(run::Mount::new(dest, overlay, overlay, readonly, Some(options)))
    } else {
        let source = if len == 2 {
            try!(CString::new(mount.mounts[0]))
        } else {
            dest.clone()
        };

        Ok(run::Mount::from_bind(dest, source, nix::mount::MS_BIND | propagation | readonly))
    }
}

fn parse_mount(value: &str) -> MountArg {
    let mut value = value.splitn(2, ',');
    let mounts = value.next().unwrap();
    let options = value.next();

    let mounts = mounts.split(':').collect();
    let options = options.map(|o| o.split(',').collect()).unwrap_or(Vec::new());

    MountArg {
        mounts: mounts,
        options: options,
    }
}

fn main() {
    let app = App::new("encage-run").setting(AppSettings::SubcommandRequiredElseHelp);
    let app = clap_app! { @app (app)
        (author: "arcnmx")
        (about: "Encage Linux container runtime")
        (@subcommand exec =>
            (about: "Executes a process in a container")
            (@arg ROOT: +required "The image root")
            (@arg BIND: --bind ... +takes_value "A path to bind mount inside the container")
            (@arg CWD: --cwd +takes_value "Set the process working directory")
            (@arg ENV: --env ... +takes_value "Set an environment variable in the container")
            (@arg DIRECT: --direct "Run directly on top of the root directory")
            (@arg CMD: +required "Command to be run inside the container")
            (@arg ARGS: ... "Command arguments")
        )
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
        ("exec", Some(matches)) => {
            fn config<'n, 'a>(matches: &clap::ArgMatches<'n, 'a>) -> Result<i32, NulError> {
                use std::iter;
                use run::Config;

                let tmp = TempDir::new("encage").expect("failed to create temporary dir");
                let mount_dir = tmp.path().join("mounts");

                let root_dir = if matches.is_present("DIRECT") {
                    None
                } else {
                    let root = tmp.path().join("root");
                    fs::create_dir(&root).expect("failed to create root dir");
                    Some(root)
                };

                let root = matches.value_of("ROOT").unwrap();
                let mut root = parse_mount(&root);

                let binds = matches.values_of("BIND").unwrap_or(Vec::new());
                let binds = binds.iter().map(|b| parse_mount(b)).collect::<Vec<_>>();

                let cmd = matches.value_of("CMD").unwrap();
                let args = matches.values_of("ARGS").unwrap_or(Vec::new());

                let cwd = matches.value_of("CWD");

                let mut env = matches.values_of("ENV").unwrap_or(Vec::new());
                if !env.iter().any(|e| e.starts_with("PATH=")) {
                    env.push("PATH=/bin:/usr/bin");
                }

                let mut namespaces = vec![
                    run::Namespace::with_kind(run::NamespaceKind::Mount),
                    run::Namespace::with_kind(run::NamespaceKind::IPC),
                    run::Namespace::with_kind(run::NamespaceKind::UTS),
                ];
                namespaces.push(run::Namespace::with_kind(run::NamespaceKind::PID));
                //namespaces.push(run::Namespace::with_kind(run::NamespaceKind::Network));

                fs::create_dir_all(mount_dir.join("proc")).and_then(|_|
                    fs::create_dir_all(mount_dir.join("sys"))).and_then(|_|
                    fs::create_dir_all(mount_dir.join("dev"))).expect("failed to create mount dir");
                for mount in &binds {
                    fs::create_dir_all(mount_dir.join(rootless(mount.last_mount()))).expect("failed to create mount dir")
                }

                root.mounts.insert(0, mount_dir.to_str().unwrap());

                let mut mounts = Vec::<run::Mount<'static>>::new();
                let cwd = try!(CString::new(cwd.unwrap_or("/")));

                let root_path = root_dir.as_ref().map(|d| d.to_str().unwrap()).unwrap_or(root.last_mount());

                if root_dir.is_some() {
                    root.mounts.push(root_path);
                }
                mounts.push(try!(mount_bind(&root, None)));

                for bind in binds {
                    mounts.push(try!(mount_bind(&bind, Some(root_path))));
                }

                let root_path = try!(CString::new(root_path));
                let root = run::Root::new(root_path, cwd);

                let cmd = try!(CString::new(cmd));
                let args = try!(iter::once(Ok(cmd.clone())).chain(args.iter().map(|s| CString::new(&s[..]))).collect::<Result<Vec<_>, _>>());
                let env = try!(env.into_iter().map(|e| {
                    if e.as_bytes().into_iter().any(|&c| c == b'=') {
                        CString::new(e)
                    } else {
                        let mut var = e.as_bytes().to_owned();
                        var.push(b'=');
                        var.extend_from_slice(env::var_os(e).expect("invalid environment variable").as_bytes());
                        CString::new(var)
                    }
                }).collect::<Result<Vec<_>, _>>());
                let command = run::Command::new(cmd, nix::null_terminated::NullTerminatedVec::map_from(args), nix::null_terminated::NullTerminatedVec::map_from(env));

                let configs = run::Configs::new(namespaces)
                    .chain(run::Configs::new(mounts))
                    .chain(root)
                    .chain(run::Configs::new(run::default_mounts()))
                    .chain(run::Configs::new(run::default_symlinks()))
                    .chain(run::Configs::new(run::default_devices()));

                Ok(run::run(&configs, &command).expect("failed to run"))
            }

            let status = config(matches).expect("failed to run");
            exit(status)
        },
        _ => unreachable!()
    }
}
