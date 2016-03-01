use std::ffi::CStr;
use std::borrow::Cow;
use into_cow::IntoCow;
use config::Config;
use nix;

#[derive(Clone, Debug)]
pub struct Symlink<'a> {
    target: Cow<'a, CStr>,
    linkpath: Cow<'a, CStr>,
}

impl<'a> Symlink<'a> {
    pub fn new<T: IntoCow<'a, CStr>, L: IntoCow<'a, CStr>>(target: T, linkpath: L) -> Self {
        Symlink {
            target: target.into_cow(),
            linkpath: linkpath.into_cow(),
        }
    }
}

impl<'a> Config for Symlink<'a> {
    fn prepare(&self) -> nix::Result<()> {
        mod ffi {
            use libc::{c_char, c_int};

            extern {
                pub fn symlink(target: *const c_char, linkpath: *const c_char) -> c_int;
            }
        }

        let res = unsafe {
            ffi::symlink(self.target.as_ref().as_ptr(), self.linkpath.as_ref().as_ptr())
        };

        nix::Errno::result(res).map(drop)
    }
}

pub fn default_symlinks() -> Vec<Symlink<'static>> {
    vec![
        Symlink::new(cstr!("/proc/self/fd"), cstr!("/dev/fd")),
        Symlink::new(cstr!("/proc/self/fd/0"), cstr!("/dev/stdin")),
        Symlink::new(cstr!("/proc/self/fd/1"), cstr!("/dev/stdout")),
        Symlink::new(cstr!("/proc/self/fd/2"), cstr!("/dev/stderr")),
    ]
}
