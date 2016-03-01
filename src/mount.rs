use std::ffi::CStr;
use std::borrow::{Cow, Borrow};
use into_cow::IntoCow;
use config::Config;
use nix;

#[derive(Clone, Debug)]
pub struct Mount<'a> {
    path: Cow<'a, CStr>,
    source: Option<Cow<'a, CStr>>,
    filesystem: Option<Cow<'a, CStr>>,
    flags: nix::mount::MsFlags,
    opts: Option<Cow<'a, CStr>>,
}

impl<'a> Mount<'a> {
    pub fn new<P: IntoCow<'a, CStr>, S: IntoCow<'a, CStr>, FS: IntoCow<'a, CStr>, O: IntoCow<'a, CStr>>(path: P, source: Option<S>, filesystem: Option<FS>, flags: nix::mount::MsFlags, opts: Option<O>) -> Self {
        Mount {
            path: path.into_cow(),
            source: source.map(IntoCow::into_cow),
            filesystem: filesystem.map(IntoCow::into_cow),
            flags: flags,
            opts: opts.map(IntoCow::into_cow),
        }
    }

    pub fn from_bind<P: IntoCow<'a, CStr>, S: IntoCow<'a, CStr>>(path: P, source: S, flags: nix::mount::MsFlags) -> Self {
        Mount {
            path: path.into_cow(),
            source: Some(source.into_cow()),
            filesystem: None,
            flags: flags,
            opts: None,
        }
    }
}

impl<'a> Config for Mount<'a> {
    fn prepare(&self) -> nix::Result<()> {
        // TODO: cgroup mounts require a ton of extra work
        if self.filesystem == Some(Cow::Borrowed(cstr!("cgroup"))) {
            println!("TODO: cgroup mount");
            return Ok(())
        }

        // TODO: mkdir properly
        unsafe { nix::sys::ioctl::libc::mkdir(self.path.as_ptr(), 0o755); }

        let source = self.source.as_ref().map(Borrow::<CStr>::borrow);
        let filesystem = self.filesystem.as_ref().map(Borrow::<CStr>::borrow);
        let opts = self.opts.as_ref().map(Borrow::<CStr>::borrow);
        try!(nix::mount::mount(
            source,
            &self.path,
            filesystem,
            self.flags,
            opts
        ));

        // bind mount options require a special song and dance
        // TODO: only bother with this if additional flags are being requested
        if self.flags.contains(nix::mount::MS_BIND) {
            try!(nix::mount::mount(
                source,
                &self.path,
                filesystem,
                nix::mount::MS_REMOUNT | self.flags,
                opts
            ));
        }

        Ok(())
    }
}

pub fn default_mounts() -> Vec<Mount<'static>> {
    vec![
        Mount::new(
            cstr!("/proc"), Some(cstr!("proc")), Some(cstr!("proc")),
            nix::mount::MsFlags::empty(), None::<&CStr>
        ),
        Mount::new(
            cstr!("/dev"), Some(cstr!("tmpfs")), Some(cstr!("tmpfs")),
            nix::mount::MS_STRICTATIME | nix::mount::MS_NOSUID,
            Some(cstr!("mode=755,size=65536k"))
        ),
        Mount::new(
            cstr!("/dev/pts"), Some(cstr!("devpts")), Some(cstr!("devpts")),
            nix::mount::MS_NOEXEC | nix::mount::MS_NOSUID,
            Some(cstr!("newinstance,ptmxmode=0666,mode=0620,gid=5"))
        ),
        Mount::new(
            cstr!("/dev/mqueue"), Some(cstr!("mqueue")), Some(cstr!("mqueue")),
            nix::mount::MS_NOEXEC | nix::mount::MS_NOSUID | nix::mount::MS_NODEV,
            None::<&CStr>
        ),
        Mount::new(
            cstr!("/dev/shm"), Some(cstr!("shm")), Some(cstr!("tmpfs")),
            nix::mount::MS_NOEXEC | nix::mount::MS_NOSUID | nix::mount::MS_NODEV,
            Some(cstr!("mode=1777,size=65536k"))
        ),
        Mount::new(
            cstr!("/sys"), Some(cstr!("sysfs")), Some(cstr!("sysfs")),
            nix::mount::MS_NOEXEC | nix::mount::MS_NOSUID | nix::mount::MS_NODEV,
            None::<&CStr>
        ),
        Mount::new(
            cstr!("/tmp"), Some(cstr!("tmpfs")), Some(cstr!("tmpfs")),
            nix::mount::MsFlags::empty(), None::<&CStr>
        ),
    ]
}
