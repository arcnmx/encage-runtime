use std::borrow::Cow;
use into_cow::IntoCow;
use config::Config;
use nix;

#[derive(Clone, Debug)]
pub struct User<'a> {
    uid: u32,
    gid: u32,
    supplementary_groups: Cow<'a, [u32]>,
}

impl<'a> User<'a> {
    pub fn new<G: IntoCow<'a, [u32]>>(uid: u32, gid: u32, groups: G) -> Self {
        User {
            uid: uid,
            gid: gid,
            supplementary_groups: groups.into_cow(),
        }
    }
}

impl<'a> Config for User<'a> {
    fn prepare(&self) -> nix::Result<()> {
        try!(nix::unistd::setgid(self.gid));
        try!(nix::unistd::setgroups(&self.supplementary_groups));
        nix::unistd::setuid(self.uid)
    }
}
