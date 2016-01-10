use config::Config;
use command::Command;
use void::ResultVoidErrExt;
use nix;

pub fn run<'a, C: Config>(config: C, command: &'a Command<'a>) -> nix::Result<i32> {
    use nix::sys::wait::{self, WaitStatus};

    let flags = config.clone_flags() | nix::sys::signal::SIGCHLD as nix::sched::CloneFlags;
    let mut stack = [0; 0x200];
    let pid = try!(nix::sched::clone(Box::new(move || {
        let res = config.prepare().and_then(|_| {
            command.exec()
        });
        println!("{:?}", res.void_unwrap_err());
        1
    }), &mut stack, flags));

    loop {
        match wait::waitpid(pid, Some(wait::__WALL)) {
            Ok(WaitStatus::StillAlive) |
            Ok(WaitStatus::Stopped(..)) | Ok(WaitStatus::Continued(..)) |
            Err(nix::Errno::EINTR) =>
                (),
            Ok(WaitStatus::Exited(_, status)) =>
                return Ok(status as i32),
            Ok(WaitStatus::Signaled(..)) | Err(_) =>
                panic!("signaled"),
        }
    }
}
