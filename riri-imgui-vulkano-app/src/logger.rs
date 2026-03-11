use riri_mod_tools_rt::logger::{ LogColor, LogLevel };
use std::{
    sync::OnceLock,
    time::SystemTime
};
use std::sync::mpsc::Sender;
#[derive(Debug)]
pub(crate) struct Logger;

static LOGGER: OnceLock<Logger> = OnceLock::new();
static LOGGER_TASK_SEND: OnceLock<Sender<String>> = OnceLock::new();

impl Logger {
    pub(crate) fn set_native_callbacks() {
        unsafe { riri_mod_tools_rt::logger::set_reloaded_logger(invoke_reloaded_logger) };
        unsafe { riri_mod_tools_rt::logger::set_reloaded_logger_newline(invoke_reloaded_logger) };
    }

    pub(crate) fn init_task() {
        let (tx, rx)  = std::sync::mpsc::channel::<String>();
        LOGGER_TASK_SEND.set(tx).unwrap();
        LOGGER.set(Self::new()).unwrap();
        std::thread::spawn(move || {
            while let Ok(v) = rx.recv() {
                println!("{}", v);
            }
        });
        Self::set_native_callbacks();
    }

    pub(crate) fn new() -> Self { Self }
}

impl Drop for Logger {
    fn drop(&mut self) {
        println!("Dropped!");
        // self.handle.sync_all().unwrap();
    }
}

// https://en.wikipedia.org/wiki/ANSI_escape_code
trait TerminalEscape {
    fn get_escape() -> &'static str;
    fn get_return() -> &'static str;
}
struct Escape5B;
impl TerminalEscape for Escape5B {
    fn get_escape() -> &'static str { "\x1b[" }
    fn get_return() -> &'static str { "m" }
}

trait TerminalFormat {
    fn set_foreground_color_code<E>(color: LogColor) -> String where E: TerminalEscape;
    fn reset<E>() -> String where E: TerminalEscape;
}
struct TrueColor;
impl TerminalFormat for TrueColor {
    fn set_foreground_color_code<E>(color: LogColor) -> String
    where E: TerminalEscape
    {
        format!(
            "{}38;2;{};{};{}{}",
            E::get_escape(), color.get_red(), color.get_green(), color.get_blue(), E::get_return()
        )
    }
    fn reset<E>() -> String where E : TerminalEscape {
        format!("{}0{}", E::get_escape(), E::get_return())
    }
}

fn multiplayer_get_color(level: LogLevel) -> LogColor {
    match level {
        LogLevel::Verbose => riri_mod_tools_rt::logger::builtin_colors::PINK,
        LogLevel::Debug => riri_mod_tools_rt::logger::builtin_colors::VIOLET,
        LogLevel::Information => riri_mod_tools_rt::logger::builtin_colors::HOTPINK,
        LogLevel::Warning => riri_mod_tools_rt::logger::builtin_colors::SANDYBROWN,
        LogLevel::Error => riri_mod_tools_rt::logger::builtin_colors::RED
    }
}

fn fmt_log_level(level: LogLevel) -> String {
    let mut level_text = format!("{}", level);
    level_text.push_str(&" ".repeat(5-level_text.len()));
    format!("[{}{}{}]",
            TrueColor::set_foreground_color_code::<Escape5B>(multiplayer_get_color(level)),
            level_text, TrueColor::reset::<Escape5B>()
    )
}

#[allow(dead_code)]
pub(crate) fn fmt_game_module(game: &str) -> String {
    let mut game = game.to_owned();
    game.make_ascii_uppercase();
    match game.as_ref() {
        // Metaphor: Refantazio
        "XRD759" => format!("{}{}{}",
                            TrueColor::set_foreground_color_code::<Escape5B>(LogColor::from_rgb_u8(78, 207, 147)),
                            game, TrueColor::reset::<Escape5B>()),
        // Persona 6
        "XRD768" => format!("{}{}{}",
                            TrueColor::set_foreground_color_code::<Escape5B>(LogColor::from_rgb_u8(78, 207, 83)),
                            game, TrueColor::reset::<Escape5B>()),
        // Persona 3 Reload
        "XRD777" => format!("{}{}{}",
                            TrueColor::set_foreground_color_code::<Escape5B>(LogColor::from_rgb_u8(66, 135, 245)),
                            game, TrueColor::reset::<Escape5B>()),
        // Unknown, passthrough
        _ => game
    }
}

#[allow(dead_code)]
unsafe extern "C" fn invoke_reloaded_logger(p: *const u8, len: usize, _c: LogColor, level: LogLevel, _: bool) {
    let text = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(p, len)) };
    let time = humantime::format_rfc3339_millis(SystemTime::now());
    let text_fmt = format!("[{}] {} {}", time, fmt_log_level(level), text);
    match LOGGER_TASK_SEND.get() {
        Some(v) => if let Err(_) = v.send(text_fmt) {
            println!("ERROR! Logger task is full! Try allocating a larger channel or changing it to unbounded!");
        },
        // fallback to blocking method (this won't be written to log file)
        None => println!("[BLOCK] {}", text_fmt)
    }
}

#[allow(dead_code)]
pub(crate) fn get_game_code(mod_path: &'static str) -> &'static str {
    let path_sep_indices: Vec<_> = mod_path.match_indices("::").collect();
    if path_sep_indices.len() < 2 { // crate root
        return "";
    }
    let start_index = path_sep_indices[0].0 + path_sep_indices[0].1.len();
    let end_index = path_sep_indices[1].0;
    &mod_path[start_index..end_index]
}