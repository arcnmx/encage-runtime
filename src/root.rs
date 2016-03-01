use std::ffi::CStr;
use std::borrow::Cow;
use into_cow::IntoCow;
use config::Config;
use nix;

#[derive(Clone, Debug)]
pub struct Root<'a> {
    chroot: Cow<'a, CStr>,
    cwd: Cow<'a, CStr>,
}

impl<'a> Root<'a> {
    pub fn new<R: IntoCow<'a, CStr>, C: IntoCow<'a, CStr>>(chroot: R, cwd: C) -> Self {
        Root {
            chroot: chroot.into_cow(),
            cwd: cwd.into_cow(),
        }
    }
}

impl<'a> Config for Root<'a> {
    fn prepare(&self) -> nix::Result<()> {
        nix::unistd::chroot(&*self.chroot).and_then(|_|
            nix::unistd::chdir(&*self.cwd)
        )
    }
}
