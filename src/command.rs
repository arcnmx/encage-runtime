use std::borrow::{Cow, Borrow};
use std::ffi::CStr;
use into_cow::IntoCow;
use CStringArgs;
use void;
use nix;

#[derive(Clone, Debug)]
pub struct Command<'a> {
    cmd: Cow<'a, CStr>,
    args: CStringArgs,
    env: CStringArgs,
}

impl<'a> Command<'a> {
    pub fn new<C: IntoCow<'a, CStr>>(cmd: C, args: CStringArgs, env: CStringArgs) -> Self {
        Command {
            cmd: cmd.into_cow(),
            args: args,
            env: env,
        }
    }

    pub fn exec(&self) -> nix::Result<void::Void> {
        let path = self.env.iter().find(|env| env.as_bytes().starts_with(b"PATH=")).map(Borrow::borrow)
            .unwrap_or(cstr!("PATH=/bin:/usr/bin"));
        unsafe { nix::sys::ioctl::libc::putenv(path.as_ptr() as *mut _) };

        nix::unistd::execvpe(&self.cmd, &self.args, &self.env)
    }
}
