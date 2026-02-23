use std::{
    fmt,
    fs::OpenOptions,
    io::Write,
    sync::{Mutex, OnceLock},
};

static LOG_FILE: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();

fn with_log_file(mut f: impl FnMut(&mut std::fs::File)) {
    let mutex = LOG_FILE.get_or_init(|| Mutex::new(None));
    let Ok(mut guard) = mutex.lock() else {
        return;
    };

    if guard.is_none() {
        *guard = OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs.txt")
            .ok();
    }

    if let Some(file) = guard.as_mut() {
        f(file);
    }
}

pub fn log_fmt(args: fmt::Arguments) {
    with_log_file(|file| {
        let _ = file.write_fmt(args);
        let _ = file.write_all(b"\n");
        let _ = file.flush();
    });
}

/// Like `println!`, but writes to `logs.txt` in the current working directory.
#[macro_export]
macro_rules! log {
    () => {
        {
            $crate::logging::log_newline()
        }
    };
    ($($arg:tt)*) => {
        {
            $crate::logging::log_fmt(format_args!($($arg)*))
        }
    };
}
