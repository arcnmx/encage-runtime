use config::Config;
use nix;
use nix::sys::capability::{self, CapabilityFlags, capset};
use nix::sys::ioctl::libc::{c_int, c_ulong};

#[derive(Clone, Debug)]
pub struct Capabilities {
    caps: CapabilityFlags,
}

fn capbset_drop(cap: usize) -> nix::Result<()> {
    extern {
        fn prctl(opt: c_int, arg2: c_ulong, arg3: c_ulong, arg4: c_ulong, arg5: c_ulong) -> c_int;
    }

    const PR_CAPBSET_DROP: c_int = 24;

    let res = unsafe {
        prctl(PR_CAPBSET_DROP, cap as c_ulong, 0, 0, 0)
    };

    nix::Errno::result(res).map(drop)
}

impl Capabilities {
    pub fn new(caps: CapabilityFlags) -> Self {
        Capabilities {
            caps: caps,
        }
    }
}

impl Config for Capabilities {
    fn prepare(&self) -> nix::Result<()> {
        // Remove all but `self.caps` from the bounding cap set
        for cap in 0..63 {
            if !self.caps.contains(CapabilityFlags::from_bits_truncate(1 << cap)) {
                match capbset_drop(cap) {
                    Err(nix::Errno::EINVAL) | Ok(..) => (),
                    Err(e) => return Err(e),
                }
            }
        }

        let mut caps = capability::Capabilities::empty();
        caps.set_all(self.caps);
        capset(nix::unistd::getpid(), &caps)
    }
}
