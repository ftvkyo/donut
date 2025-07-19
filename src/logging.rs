fn init_backtrace() {
    use color_backtrace::{BacktracePrinter, Frame};

    let filter = |frames: &mut Vec<&Frame>| {
        frames.retain(
            |frame| matches!(&frame.name, Some(name) if name.starts_with(super::CRATE_NAME)),
        )
    };

    BacktracePrinter::new()
        .strip_function_hash(true)
        .add_frame_filter(Box::new(filter))
        .install(color_backtrace::default_output_stream());
}

#[cfg(not(test))]
pub fn init_logging() {
    init_backtrace();

    let level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::builder()
        .filter_module(module_path!().trim_end_matches("::logging"), level)
        .parse_default_env()
        .init();
}

#[cfg(test)]
pub fn init_logging() {
    use anstyle::{RgbColor as Rgb, Style};
    use std::{
        hash::{DefaultHasher, Hash, Hasher},
        sync::{Arc, RwLock},
    };

    init_backtrace();

    let crate_prefix = format!("{}::", super::CRATE_NAME);

    let module_fg = [
        Rgb(249, 65, 68),
        Rgb(243, 114, 44),
        Rgb(248, 150, 30),
        Rgb(249, 199, 79),
        Rgb(144, 190, 109),
        Rgb(67, 170, 139),
        Rgb(87, 117, 144),
        Rgb(255, 173, 173),
        Rgb(255, 214, 165),
        Rgb(253, 255, 182),
        Rgb(202, 255, 191),
        Rgb(155, 246, 255),
        Rgb(160, 196, 255),
        Rgb(189, 178, 255),
        Rgb(255, 198, 255),
        Rgb(255, 255, 252),
    ];

    let module_length_max = Arc::new(RwLock::new(0));

    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .format(move |buf, record| {
            use std::io::Write;

            let level = record.level();
            let level_style = buf.default_level_style(level);

            write!(buf, "[{level_style}{level}{level_style:#}")?;

            if let Some(module) = record.module_path() {
                let module = module.trim_start_matches(&crate_prefix);
                let module_length = module.len();

                let module_length_current_max = *module_length_max.read().unwrap();
                let module_length = if module_length > module_length_current_max {
                    *module_length_max.write().unwrap() = module_length;
                    module_length
                } else {
                    module_length_current_max
                };

                let mut hasher = DefaultHasher::new();
                module.hash(&mut hasher);
                let module_hash = hasher.finish();

                let module_fg_i = module_hash % module_fg.len() as u64;
                let module_fg = module_fg[module_fg_i as usize];

                let module_style = Style::new().fg_color(Some(module_fg.into()));

                write!(
                    buf,
                    " {module_style}{module:module_length$}{module_style:#}",
                )?;
            }

            writeln!(buf, "] {}", record.args())
        })
        .is_test(true)
        .try_init();
}
