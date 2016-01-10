extern crate ocf;

use std::borrow::Cow;
use std::ffi::CString;
use nix;
use super::*;

impl From<ocf::linux::NamespaceKind> for NamespaceKind {
    fn from(kind: ocf::linux::NamespaceKind) -> Self {
        match kind {
            ocf::linux::NamespaceKind::PID => NamespaceKind::PID,
            ocf::linux::NamespaceKind::Network => NamespaceKind::Network,
            ocf::linux::NamespaceKind::Mount => NamespaceKind::Mount,
            ocf::linux::NamespaceKind::IPC => NamespaceKind::IPC,
            ocf::linux::NamespaceKind::UTS => NamespaceKind::UTS,
            ocf::linux::NamespaceKind::User => NamespaceKind::User,
        }
    }
}

quick_error! {
    #[derive(Clone, Debug)]
    pub enum OcfError {
        UnsupportedPlatform {
            description("unsupported platform")
        }
        UnsupportedArchitecture {
            description("unsupported architecture")
        }
        InvalidConfiguration(err: &'static str) {
            description("invalid configuration")
            display("{}", err)
        }
        MissingMount(mount: String) {
            description("runtime configuration missing mountpoint")
            display("missing mountpoint: {}", mount)
        }
        CStringError {
            description("internal nul error")
            from(::std::ffi::NulError)
        }
    }
}

pub fn ocf_config<'s, 'r>(config: &'s ocf::Spec, runtime: &'r ocf::RuntimeSpec) -> Result<(Box<Config>, Command<'static>), OcfError> {
    use std::path::Path;
    use std::ffi::NulError;
    use std::iter;

    let (arch, caps, linux) = if let (&ocf::Platform::Linux { ref arch, ref capabilities }, Some(linux)) = (config.platform(), runtime.linux()) {
        (arch, capabilities, linux)
    } else {
        return Err(OcfError::UnsupportedPlatform)
    };

    #[cfg(target_arch = "x86")]
    const ARCH: [ocf::Arch; 1] = [ocf::Arch::X86];
    #[cfg(target_arch = "x86_64")]
    const ARCH: [ocf::Arch; 2] = [ocf::Arch::X86, ocf::Arch::X86_64];

    if !ARCH.contains(arch) {
        return Err(OcfError::UnsupportedArchitecture)
    }

    if config.hostname().is_some() && !linux.namespaces().iter().any(|ns| ns.path.is_none() && ns.kind == ocf::linux::NamespaceKind::UTS) {
        return Err(OcfError::InvalidConfiguration("hostname requires new uts namespace"))
    }

    if !linux.namespaces().iter().any(|ns| ns.kind == ocf::linux::NamespaceKind::Mount) {
        return Err(OcfError::InvalidConfiguration("mount namespace required"))
    }

    let mounts: Result<Vec<_>, _> = config.mounts().iter().map(|mount|
        runtime.mounts().get(&mount.name).map(|rm| (mount, rm)).ok_or(&mount.name)
    ).collect();

    let mounts = try!(mounts.map_err(|mount|
        OcfError::MissingMount(mount.to_owned())
    ));

    fn opres<T, E>(str: Option<Result<T, E>>) -> Result<Option<T>, E> {
        match str {
            Some(Ok(t)) => Ok(Some(t)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    let namespaces = try!(linux.namespaces().iter().map(|ns| -> Result<_, NulError> {
        Ok(Namespace::new(ns.kind.into(), try!(opres(ns.path().map(CString::new)))))
    }).collect::<Result<Vec<_>, _>>());

    let root = try!(CString::new(config.root().path()));
    let root_readonly = if config.root().readonly() { nix::mount::MS_RDONLY } else { nix::mount::MsFlags::empty() };
    let root_propagation = match linux.rootfs_propagation() {
        ocf::linux::MountPropagation::Private => nix::mount::MS_PRIVATE,
        ocf::linux::MountPropagation::Shared => nix::mount::MS_SHARED,
        ocf::linux::MountPropagation::Slave => nix::mount::MS_SLAVE,
    };
    let mounts = try!(
        iter::once(Ok(Mount::from_bind(root.clone(), root, nix::mount::MS_BIND | root_propagation | root_readonly)))
        .chain(mounts.into_iter().map(|(m, mount)| -> Result<_, OcfError> {
            let path = Path::new(m.path().trim_left_matches('/'));
            if path.has_root() {
                return Err(OcfError::InvalidConfiguration("mount path issue"))
            }

            let path = Path::new(config.root().path()).join(path).to_string_lossy().into_owned();
            let path = try!(CString::new(path));

            let (mut flags, opts) = mount.options().iter().map(|opt| (Some(match &opt[..] {
                "bind" => nix::mount::MS_BIND,
                "move" => nix::mount::MS_MOVE,

                "private" => nix::mount::MS_PRIVATE,
                "slave" => nix::mount::MS_SLAVE,
                "shared" => nix::mount::MS_SHARED,

                "acl" => nix::mount::MS_POSIXACL,
                "noatime" => nix::mount::MS_NOATIME,
                "nodev" => nix::mount::MS_NODEV,
                "nodiratime" => nix::mount::MS_NODIRATIME,
                "dirsync" => nix::mount::MS_DIRSYNC,
                "noexec" => nix::mount::MS_NOEXEC,
                "mand" => nix::mount::MS_MANDLOCK,
                "relatime" => nix::mount::MS_RELATIME,
                "strictatime" => nix::mount::MS_STRICTATIME,
                "lazytime" => nix::mount::MS_LAZYTIME,
                "nosuid" => nix::mount::MS_NOSUID,
                "silent" => nix::mount::MS_SILENT,
                "remount" => nix::mount::MS_REMOUNT,
                "ro" => nix::mount::MS_RDONLY,
                "sync" => nix::mount::MS_SYNCHRONOUS,
                "nouser" => nix::mount::MS_NOUSER,
                "x-mount-mkdir" => unimplemented!(),
                _ => return (None, Some(opt)),
            }), None)).fold((nix::mount::MsFlags::empty(), String::new()), |(flags, mut opts), (flag, opt)| (
                if let Some(flag) = flag { flags | flag } else { flags },
                {
                    if let Some(opt) = opt {
                        if !opts.is_empty() {
                            opts.push(',');
                        }
                        opts.push_str(opt);
                    }
                    opts
                },
            ));

            let kind = match mount.kind() {
                "" => None,
                "bind" => {
                    flags = flags | nix::mount::MS_BIND;
                    None
                },
                kind => Some(try!(CString::new(kind))),
            };

            Ok(Mount::new(
                path,
                Some(try!(CString::new(mount.source()))),
                kind,
                flags,
                if opts.is_empty() { None } else { Some(try!(CString::new(opts))) }
            ))
        }))
        .collect::<Result<Vec<_>, _>>()
    );

    let devices = try!(linux.devices().iter().map(|device| -> Result<_, OcfError> {
        let mode = try!(nix::sys::stat::Mode::from_bits(device.mode())
            .ok_or(OcfError::InvalidConfiguration("invalid device mode")));

        let kind = match device.kind() {
            b'b' => nix::sys::stat::S_IFBLK,
            b'c' | b'u' => nix::sys::stat::S_IFCHR,
            b'p' => nix::sys::stat::S_IFIFO,
            _ => return Err(OcfError::InvalidConfiguration("invalid device type"))
        };

        Ok(Device::new(
            try!(CString::new(device.path())),
            kind,
            mode,
            (device.major(), device.minor())
        ))
    }).collect::<Result<Vec<_>, _>>());

    let hostname = config.hostname().map(ToOwned::to_owned).map(Hostname::new);

    if let Some(cwd) = config.process().cwd() {
        use std::path::Path;

        if !Path::new(cwd).is_absolute() {
            return Err(OcfError::InvalidConfiguration("process cwd must be an absolute path"))
        }
    }

    let root = Root::new(
        try!(CString::new(config.root().path())),
        try!(
            config.process().cwd().map(|cwd| CString::new(cwd).map(Cow::Owned))
            .unwrap_or(Ok(Cow::Borrowed(cstr!("/"))))
        )
    );

    let user = config.process().linux_user().map(|user| User::new(user.uid(), user.gid(), user.additional_gids().to_owned()));

    let capabilities = Capabilities::new(caps.iter().fold(
        nix::sys::capability::CapabilityFlags::empty(),
        |flags, cap| flags | nix::sys::capability::CapabilityFlags::from_bits_truncate(1 << cap.cap_flag())
    ));

    let command = {
        let cmd = try!(CString::new(config.process().cmd()));
        let args = try!(iter::once(Ok(cmd.clone())).chain(config.process().args().iter().map(|s| CString::new(&s[..]))).collect::<Result<Vec<_>, _>>());
        let env = try!(config.process().env().iter().map(ToString::to_string).map(CString::new).collect::<Result<Vec<_>, _>>());
        Command::new(cmd, nix::null_terminated::NullTerminatedVec::map_from(args), nix::null_terminated::NullTerminatedVec::map_from(env))
    };

    Ok((Box::new(
        Configs::new(namespaces)
        .chain(Configs::new(mounts))
        .chain(root)
        .chain(Configs::new(devices))
        .chain(hostname)
        .chain(user)
        .chain(capabilities)
    ),
    command))
}
