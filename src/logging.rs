pub use tracing::{debug, error, info, warn, Level};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, prelude::*, util::SubscriberInitExt, EnvFilter,
};

use crate::{
    print_to_terminal,
    vfs::{create_drive, open_file, File},
    Address, Request,
};

pub struct RemoteLogSettings {
    pub target: Address,
    pub level: Level,
}

pub struct RemoteWriter {
    pub target: Address,
}

pub struct RemoteWriterMaker {
    pub target: Address,
}

pub struct FileWriter {
    pub file: File,
}

pub struct FileWriterMaker {
    pub file: File,
}

pub struct TerminalWriter {
    pub level: u8,
}

pub struct TerminalWriterMaker {
    pub level: u8,
}

impl std::io::Write for RemoteWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Request::to(&self.target).body(buf).send().unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for RemoteWriterMaker {
    type Writer = RemoteWriter;

    fn make_writer(&'a self) -> Self::Writer {
        RemoteWriter {
            target: self.target.clone(),
        }
    }
}

impl std::io::Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // TODO: use non-blocking call instead? (.append() `send_and_await()`s)
        self.file
            .append(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for FileWriterMaker {
    type Writer = FileWriter;

    fn make_writer(&'a self) -> Self::Writer {
        FileWriter {
            file: File::new(self.file.path.clone(), self.file.timeout),
        }
    }
}

impl std::io::Write for TerminalWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let string = String::from_utf8(buf.to_vec())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        print_to_terminal(self.level, &format!("{string}"));
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TerminalWriterMaker {
    type Writer = TerminalWriter;

    fn make_writer(&'a self) -> Self::Writer {
        TerminalWriter { level: self.level }
    }
}

/// Initialize `tracing`-based logging for the given process at the given level.
///
/// To write to logs, import the re-exported `debug!`, `info!`, `warn!`, `error!`
/// macros and use as usual. Logs will be printed to terminal as appropriate depending
/// on given level. Logs will be logged into the logging file as appropriate depending
/// on the given level.
///
/// The logging file lives in the node's `vfs/` directory, specifically at
/// `node/vfs/package:publisher.os/log/process.log`, where `node` is your node's home
/// directory, `package` is the package name, `publisher.os` is the publisher of the
/// package, and `process` is the process name of the process doing the logging.
pub fn init_logging(
    our: &Address,
    file_level: Level,
    terminal_level: Level,
    remote: Option<RemoteLogSettings>,
) -> anyhow::Result<()> {
    let log_dir_path = create_drive(our.package_id(), "log", None)?;
    let log_file_path = format!("{log_dir_path}/{}.log", our.process());
    let log_file = open_file(&log_file_path, true, None)?;

    let file_filter = EnvFilter::new(file_level.as_str());
    let error_filter = tracing_subscriber::filter::filter_fn(|metadata: &tracing::Metadata<'_>| {
        metadata.level() == &Level::ERROR
    });
    let warn_filter = tracing_subscriber::filter::filter_fn(|metadata: &tracing::Metadata<'_>| {
        metadata.level() == &Level::WARN
    });
    let info_filter = tracing_subscriber::filter::filter_fn(|metadata: &tracing::Metadata<'_>| {
        metadata.level() == &Level::INFO
    });
    let debug_filter = tracing_subscriber::filter::filter_fn(|metadata: &tracing::Metadata<'_>| {
        metadata.level() == &Level::DEBUG
    });
    let file_writer_maker = FileWriterMaker { file: log_file };
    let error_terminal_writer_maker = TerminalWriterMaker { level: 0 };
    let warn_terminal_writer_maker = TerminalWriterMaker { level: 1 };
    let info_terminal_writer_maker = TerminalWriterMaker { level: 2 };
    let debug_terminal_writer_maker = TerminalWriterMaker { level: 3 };

    let sub = tracing_subscriber::registry()
        .with(ErrorLayer::default())
        .with(
            fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_writer(file_writer_maker)
                .with_ansi(false)
                .with_target(false)
                .json()
                .with_filter(file_filter),
        )
        .with(
            fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .without_time()
                .with_writer(error_terminal_writer_maker)
                .with_ansi(true)
                .with_level(true)
                .with_target(true)
                .fmt_fields(fmt::format::PrettyFields::new())
                .with_filter(error_filter),
        );

    // TODO: can we DRY?
    let Some(remote) = remote else {
        if terminal_level >= Level::DEBUG {
            sub.with(
                fmt::layer()
                    .without_time()
                    .with_writer(warn_terminal_writer_maker)
                    .with_ansi(true)
                    .with_level(true)
                    .with_target(true)
                    .fmt_fields(fmt::format::PrettyFields::new())
                    .with_filter(warn_filter),
            )
            .with(
                fmt::layer()
                    .without_time()
                    .with_writer(info_terminal_writer_maker)
                    .with_ansi(true)
                    .with_level(true)
                    .with_target(true)
                    .fmt_fields(fmt::format::PrettyFields::new())
                    .with_filter(info_filter),
            )
            .with(
                fmt::layer()
                    .without_time()
                    .with_writer(debug_terminal_writer_maker)
                    .with_ansi(true)
                    .with_level(true)
                    .with_target(true)
                    .fmt_fields(fmt::format::PrettyFields::new())
                    .with_filter(debug_filter),
            )
            .init();
        } else if terminal_level >= Level::INFO {
            sub.with(
                fmt::layer()
                    .without_time()
                    .with_writer(warn_terminal_writer_maker)
                    .with_ansi(true)
                    .with_level(true)
                    .with_target(true)
                    .fmt_fields(fmt::format::PrettyFields::new())
                    .with_filter(warn_filter),
            )
            .with(
                fmt::layer()
                    .without_time()
                    .with_writer(info_terminal_writer_maker)
                    .with_ansi(true)
                    .with_level(true)
                    .with_target(true)
                    .fmt_fields(fmt::format::PrettyFields::new())
                    .with_filter(info_filter),
            )
            .init();
        } else if terminal_level >= Level::WARN {
            sub.with(
                fmt::layer()
                    .without_time()
                    .with_writer(warn_terminal_writer_maker)
                    .with_ansi(true)
                    .with_level(true)
                    .with_target(true)
                    .fmt_fields(fmt::format::PrettyFields::new())
                    .with_filter(warn_filter),
            )
            .init();
        }

        return Ok(());
    };

    let remote_filter = EnvFilter::new(remote.level.as_str());
    let remote_writer_maker = RemoteWriterMaker {
        target: remote.target,
    };
    let sub = sub.with(
        fmt::layer()
            .with_file(true)
            .with_line_number(true)
            .with_writer(remote_writer_maker)
            .with_ansi(false)
            .with_target(false)
            .json()
            .with_filter(remote_filter),
    );
    if terminal_level >= Level::DEBUG {
        sub.with(
            fmt::layer()
                .without_time()
                .with_writer(warn_terminal_writer_maker)
                .with_ansi(true)
                .with_level(true)
                .with_target(true)
                .fmt_fields(fmt::format::PrettyFields::new())
                .with_filter(warn_filter),
        )
        .with(
            fmt::layer()
                .without_time()
                .with_writer(info_terminal_writer_maker)
                .with_ansi(true)
                .with_level(true)
                .with_target(true)
                .fmt_fields(fmt::format::PrettyFields::new())
                .with_filter(info_filter),
        )
        .with(
            fmt::layer()
                .without_time()
                .with_writer(debug_terminal_writer_maker)
                .with_ansi(true)
                .with_level(true)
                .with_target(true)
                .fmt_fields(fmt::format::PrettyFields::new())
                .with_filter(debug_filter),
        )
        .init();
    } else if terminal_level >= Level::INFO {
        sub.with(
            fmt::layer()
                .without_time()
                .with_writer(warn_terminal_writer_maker)
                .with_ansi(true)
                .with_level(true)
                .with_target(true)
                .fmt_fields(fmt::format::PrettyFields::new())
                .with_filter(warn_filter),
        )
        .with(
            fmt::layer()
                .without_time()
                .with_writer(info_terminal_writer_maker)
                .with_ansi(true)
                .with_level(true)
                .with_target(true)
                .fmt_fields(fmt::format::PrettyFields::new())
                .with_filter(info_filter),
        )
        .init();
    } else if terminal_level >= Level::WARN {
        sub.with(
            fmt::layer()
                .without_time()
                .with_writer(warn_terminal_writer_maker)
                .with_ansi(true)
                .with_level(true)
                .with_target(true)
                .fmt_fields(fmt::format::PrettyFields::new())
                .with_filter(warn_filter),
        )
        .init();
    }

    Ok(())
}
