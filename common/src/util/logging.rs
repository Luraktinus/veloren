use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;
use std::path::PathBuf;

pub struct VelorenLogger {
    cur: Vec<fern::Dispatch>,
}

impl VelorenLogger {
    pub fn new() -> Self {
        VelorenLogger { cur: vec![] }
    }

    pub fn with_term(mut self, level: &LevelFilter) -> Self {
        let colors = ColoredLevelConfig::new()
            .error(Color::Red)
            .warn(Color::Yellow)
            .info(Color::Blue)
            .debug(Color::Green)
            .trace(Color::BrightBlack);

        let term = fern::Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "[{}] {}",
                    colors.color(record.level()),
                    message
                ))
            })
            .level(*level)
            .chain(std::io::stdout());

        self.cur.push(term);
        self
    }

    pub fn with_file(mut self, path: &PathBuf) -> Self {
        let file = fern::Dispatch::new()
            .format(|out, message, record| {
                if let (Some(file), Some(line)) = (record.file(), record.line()) {
                    out.finish(format_args!(
                        "{}[{}][{}] {}",
                        chrono::Local::now().format("[%Y-%m-%d/%H:%M:%S]"),
                        record.level(),
                        format!("{}:{}", file, line),
                        message
                    ))
                } else {
                    out.finish(format_args!(
                        "{}[{}][{}] {}",
                        chrono::Local::now().format("[%Y-%m-%d/%H:%M:%S]"),
                        record.level(),
                        record.target(),
                        message
                    ))
                }
            })
            .level(LevelFilter::Debug)
            .level_for("gfx_device_gl::factory", log::LevelFilter::Warn)
            .level_for("dot_vox::parser", log::LevelFilter::Info)
            .level_for("uvth", log::LevelFilter::Info)
            .chain(fern::log_file(path).expect("Failed to set log file"));

        self.cur.push(file);
        self
    }

    pub fn apply(self) {
        let mut base = fern::Dispatch::new();

        for dispatch in self.cur {
            base = base.chain(dispatch);
        }

        match base.apply() {
            Ok(()) => {}
            Err(e) => panic!("Failed to set logging! {:?}", e),
        }
    }
}
