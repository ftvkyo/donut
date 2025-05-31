fn init_logging() {
    use env_logger::{Builder, Env};

    let filter = if cfg!(debug_assertions) {
        "info"
    } else {
        "warn"
    };

    let env = Env::default().default_filter_or(filter);

    Builder::from_env(env).init();
}

fn main() {
    init_logging();

    log::info!("Hello, world!");
}
