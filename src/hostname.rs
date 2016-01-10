use std::borrow::Cow;
use into_cow::IntoCow;
use config::Config;
use nix;

pub struct Hostname<'a> {
    hostname: Cow<'a, str>,
}

impl<'a> Hostname<'a> {
    pub fn new<H: IntoCow<'a, str>>(hostname: H) -> Self {
        Hostname {
            hostname: hostname.into_cow(),
        }
    }
}

impl<'a> Config for Hostname<'a> {
    fn prepare(&self) -> nix::Result<()> {
        nix::unistd::sethostname(self.hostname.as_bytes())
    }
}
