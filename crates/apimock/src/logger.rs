use std::{borrow::Cow, sync::OnceLock};

use console::strip_ansi_codes;
use log::{Level, Metadata, Record, SetLoggerError};
use tokio::sync::mpsc::Sender;

/// logger
static LOGGER: OnceLock<AppLogger> = OnceLock::new();

/// log output
#[derive(Clone)]
enum LogOutput {
    Stdout,
    Sender(Sender<String>),
}

/// app logger
#[derive(Clone)]
struct AppLogger {
    output: LogOutput,
    includes_ansi_codes: bool,
}

impl log::Log for AppLogger {
    /// check if it is enabled
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    /// log print out
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        match &self.output {
            // default feature
            LogOutput::Stdout => {
                println!("{}", record.args());
            }
            // spawn feature
            LogOutput::Sender(tx) => {
                let args = record.args().to_string();
                let msg = if !self.includes_ansi_codes {
                    // omit ansi escape codes for console text color
                    strip_ansi_codes(args.as_ref())
                } else {
                    Cow::from(args)
                };
                // message with log level — log levels are always
                // non-empty ASCII words ("INFO", "WARN", ...), so the
                // first char always exists; the fallback to '?' is just
                // defensive in case a future log crate does something odd.
                let level_initial = record.level().to_string().chars().next().unwrap_or('?');
                let msg_with_log_level = format!("[{}] {}", level_initial, msg);

                let tx = tx.clone();
                tokio::spawn(async move {
                    let _ = tx.send(msg_with_log_level).await;
                });
            }
        }
    }

    /// flush
    fn flush(&self) {}
}

/// Install the application logger as `log`'s global logger.
///
/// # Why this is idempotent-on-failure
///
/// `OnceLock::set` can only succeed once per process. If something else
/// already set our logger (for example, a second `App::new` in tests),
/// we silently swallow the `Err` and just reuse whatever logger is
/// already installed. The only surprise this can cause is that a test
/// that expected a channel-based output might still see stdout output
/// from the prior test, which is acceptable for our setup.
pub fn init_logger(
    tx: Option<Sender<String>>,
    includes_ansi_codes: bool,
) -> Result<(), SetLoggerError> {
    let output = if let Some(tx) = tx {
        LogOutput::Sender(tx)
    } else {
        LogOutput::Stdout
    };

    LOGGER
        .set(AppLogger {
            output,
            includes_ansi_codes,
        })
        .ok();

    // `LOGGER.get()` cannot return `None` here: either our `set` above
    // succeeded (so it's populated), or an earlier call already populated
    // it. The `expect` message exists only for future refactors that might
    // accidentally remove the `set` above.
    let logger = LOGGER
        .get()
        .expect("logger must have been initialized by the set() above");
    log::set_logger(logger)?;
    log::set_max_level(log::LevelFilter::Info);

    Ok(())
}
