use nix;

pub trait Config {
    fn clone_flags(&self) -> nix::sched::CloneFlags {
        0
    }

    fn prepare(&self) -> nix::Result<()>;

    fn chain<C: Config>(self, other: C) -> ConfigChain<Self, C> where Self: Sized {
        ConfigChain(self, other)
    }
}

pub struct ConfigChain<A, B>(A, B);

impl<A: Config, B: Config> Config for ConfigChain<A, B> {
    fn clone_flags(&self) -> nix::sched::CloneFlags {
        self.0.clone_flags() | self.1.clone_flags()
    }

    fn prepare(&self) -> nix::Result<()> {
        self.0.prepare().and_then(|_|
            self.1.prepare()
        )
    }
}


impl<'a, C: Config> Config for &'a C {
    fn clone_flags(&self) -> nix::sched::CloneFlags {
        (**self).clone_flags()
    }

    fn prepare(&self) -> nix::Result<()> {
        (**self).prepare()
    }
}

impl Config for Box<Config> {
    fn clone_flags(&self) -> nix::sched::CloneFlags {
        (**self).clone_flags()
    }

    fn prepare(&self) -> nix::Result<()> {
        (**self).prepare()
    }
}

impl<C: Config> Config for Option<C> {
    fn clone_flags(&self) -> nix::sched::CloneFlags {
        self.as_ref().map(Config::clone_flags).unwrap_or(0)
    }

    fn prepare(&self) -> nix::Result<()> {
        self.as_ref().map(Config::prepare).unwrap_or(Ok(()))
    }
}

pub struct Configs<C>(Box<[C]>);

impl<C> Configs<C> {
    pub fn new<I: Into<Vec<C>>>(configs: I) -> Self {
        Configs(configs.into().into_boxed_slice())
    }
}

impl<C: Config> Config for Configs<C> {
    fn clone_flags(&self) -> nix::sched::CloneFlags {
        self.0.iter().fold(0, |flags, c| flags | c.clone_flags())
    }

    fn prepare(&self) -> nix::Result<()> {
        self.0.iter().fold(Ok(()), |res, c| res.and_then(|_| c.prepare()))
    }
}
