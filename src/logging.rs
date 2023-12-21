use clap::{Args, ValueEnum};
use env_logger::{Builder, WriteStyle};
use log::LevelFilter;
use std::io::{self, IsTerminal};

#[derive(Debug, Copy, Clone, Args)]
pub struct Logging {
    #[clap(long, short, value_enum, default_value_t = Verbosity::Info)]
    verbosity: Verbosity,

    #[clap(long, value_enum, default_value_t = Color::Auto)]
    color: Color,
}

impl Logging {
    pub fn init(self) {
        let mut builder = Builder::new();

        let write_style = match self.color {
            Color::Auto => {
                if io::stderr().is_terminal() {
                    WriteStyle::Always
                } else {
                    WriteStyle::Never
                }
            }

            Color::Always => WriteStyle::Always,
            Color::Never => WriteStyle::Never,
        };

        builder
            .filter(None, self.verbosity.into())
            .write_style(write_style);

        builder.init();
    }
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum Verbosity {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<Verbosity> for LevelFilter {
    fn from(verbosity: Verbosity) -> Self {
        match verbosity {
            Verbosity::Off => Self::Off,
            Verbosity::Error => Self::Error,
            Verbosity::Warn => Self::Warn,
            Verbosity::Info => Self::Info,
            Verbosity::Debug => Self::Debug,
            Verbosity::Trace => Self::Trace,
        }
    }
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum Color {
    Auto,
    Always,
    Never,
}
