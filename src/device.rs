use std::ffi::CStr;
use std::borrow::Cow;
use into_cow::IntoCow;
use config::Config;
use nix;

#[derive(Clone, Debug)]
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

pub fn default_devices() -> Vec<Device<'static>> {
    let world = nix::sys::stat::Mode::from_bits_truncate(0o666);
    let chr = nix::sys::stat::S_IFCHR;

    vec![
        Device::new(cstr!("/dev/null"), chr, world, (1, 3)),
        Device::new(cstr!("/dev/zero"), chr, world, (1, 5)),
        Device::new(cstr!("/dev/full"), chr, world, (1, 7)),
        Device::new(cstr!("/dev/random"), chr, world, (1, 8)),
        Device::new(cstr!("/dev/urandom"), chr, world, (1, 9)),
        Device::new(cstr!("/dev/tty"), chr, world, (5, 0)),
    ]
}
