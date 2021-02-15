use env_logger::{Builder, Color, Env};
use log::{Level, LevelFilter};
use std::io::Write;

pub fn init_logger(level: u8) {
    let env = Env::default();

    let level_filter = match level {
        0 => LevelFilter::Off,
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        5 => LevelFilter::Trace,
        _ => LevelFilter::Off,
    };

    let mut builder = Builder::from_env(env);

    builder.format(|buf, record| {
        let level = record.level();
        let mut style = buf.style();

        let color = match level {
            Level::Trace => Color::White,
            Level::Debug => Color::Cyan,
            Level::Info => Color::Green,
            Level::Warn => Color::Yellow,
            Level::Error => Color::Red,
        };

        style.set_color(color);

        if level == Level::Error {
            style.set_bold(true);
        }

        write!(buf, "[{:>5}] ", style.value(level)).unwrap();

        let ts = buf.timestamp();
        write!(buf, "[{}] ", style.value(ts)).unwrap();

        let module_path = record.module_path().unwrap().to_string();

        write!(buf, "[{:^20}] ", style.value(module_path)).unwrap();

        writeln!(buf, "{}", style.value(record.args())).unwrap();

        Ok(())
    });

    builder.filter_level(LevelFilter::Warn);
    builder.filter_module("fock", level_filter);

    builder.init();
}
