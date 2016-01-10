use std::ffi::CStr;
use std::borrow::Cow;
use into_cow::IntoCow;
use config::Config;
use nix;

pub struct Device<'a> {
    path: Cow<'a, CStr>,
    kind: nix::sys::stat::SFlag,
    mode: nix::sys::stat::Mode,
    device: (u64, u64),
}

impl<'a> Device<'a> {
    pub fn new<P: IntoCow<'a, CStr>>(path: P, kind: nix::sys::stat::SFlag, mode: nix::sys::stat::Mode, device: (u64, u64)) -> Self {
        Device {
            path: path.into_cow(),
            kind: kind,
            mode: mode,
            device: device,
        }
    }
}

impl<'a> Config for Device<'a> {
    fn prepare(&self) -> nix::Result<()> {
        fn gnu_makedev(major: u64, minor: u64) -> u64 {
            (minor & 0xff) | ((major & 0xfff) << 8) |
                ((minor & !0xff) << 12) | ((major & !0xfff) << 32)
        }

        let device = gnu_makedev(self.device.0, self.device.1);

        nix::sys::stat::mknod(&*self.path, self.kind, self.mode, device)
    }
}

