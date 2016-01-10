use std::ffi::CStr;
use std::borrow::Cow;
use into_cow::IntoCow;
use config::Config;
use nix;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamespaceKind {
    PID,
    Network,
    Mount,
    IPC,
    UTS,
    User,
}

#[derive(Clone, Debug)]
pub struct Namespace<'a> {
    kind: NamespaceKind,
    path: Option<Cow<'a, CStr>>,
}

impl<'a> Namespace<'a> {
    pub fn new<P: IntoCow<'a, CStr>>(kind: NamespaceKind, path: Option<P>) -> Self {
        Namespace {
            kind: kind,
            path: path.map(IntoCow::into_cow),
        }
    }

    pub fn with_path<P: IntoCow<'a, CStr>>(kind: NamespaceKind, path: P) -> Self {
        Namespace {
            kind: kind,
            path: Some(path.into_cow()),
        }
    }

    pub fn with_kind(kind: NamespaceKind) -> Self {
        Namespace {
            kind: kind,
            path: None,
        }
    }
}

impl NamespaceKind {
    pub fn clone_flag(&self) -> nix::sched::CloneFlags {
        match *self {
            NamespaceKind::PID => nix::sched::CLONE_NEWPID,
            NamespaceKind::Network => nix::sched::CLONE_NEWNET,
            NamespaceKind::Mount => nix::sched::CLONE_NEWNS,
            NamespaceKind::IPC => nix::sched::CLONE_NEWIPC,
            NamespaceKind::UTS => nix::sched::CLONE_NEWUTS,
            NamespaceKind::User => nix::sched::CLONE_NEWUSER,
        }
    }
}

impl<'a> Config for Namespace<'a> {
    fn clone_flags(&self) -> nix::sched::CloneFlags {
        if self.path.is_none() {
            self.kind.clone_flag()
        } else {
            0
        }
    }

    fn prepare(&self) -> nix::Result<()> {
        if let Some(ref path) = self.path {
            let fd = try!(nix::fcntl::open(&**path, nix::fcntl::O_RDONLY, nix::sys::stat::Mode::empty()));
            let ret = nix::sched::setns(fd, self.clone_flags());
            let _ = nix::unistd::close(fd);

            if self.kind == NamespaceKind::PID {
                match try!(nix::unistd::fork()) {
                    nix::unistd::Fork::Parent(pid) => {
                        use nix::sys::wait::{self, WaitStatus};
                        use nix::sys::ioctl::libc::_exit;
                        loop {
                            match wait::waitpid(pid, Some(wait::__WALL)) {
                                Ok(WaitStatus::StillAlive) |
                                Ok(WaitStatus::Stopped(..)) | Ok(WaitStatus::Continued(..)) |
                                Err(nix::Errno::EINTR) =>
                                    (),
                                Ok(WaitStatus::Exited(_, status)) =>
                                   unsafe { _exit(status as i32) },
                                Ok(WaitStatus::Signaled(..)) | Err(_) =>
                                    unsafe { _exit(1) },
                            }
                        }
                    },
                    nix::unistd::Fork::Child => (),
                }
            }

            ret
        } else {
            if self.kind == NamespaceKind::Mount {
                let empty = Some(cstr!(""));
                try!(nix::mount::mount(empty, cstr!("/"), empty, nix::mount::MS_SLAVE | nix::mount::MS_REC, empty));
            }

            Ok(())
        }
    }
}
