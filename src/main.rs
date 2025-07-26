use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    process::Command,
};

use rustyline::{
    completion::{Completer, Pair},
    error::ReadlineError,
    highlight::Highlighter,
    hint::Hinter,
    validate::Validator,
    Helper, Editor, Config as RLConfig, Context,
};

mod help;
use help::show_help;

#[derive(Default)]
struct RshConfig {
    aliases: HashMap<String, String>,
    functions: HashMap<String, String>,
    env_vars: HashMap<String, String>,
    prompt: String,
    on_start: Vec<String>,
    on_exit: Vec<String>,
}

struct RshHelper {
    commands: Vec<String>,
    aliases: Vec<String>,
}

impl Completer for RshHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        // Find start of the current word being completed
        let (start, prefix) = match line[..pos].rfind(' ') {
            Some(idx) => (idx + 1, &line[idx + 1..pos]),
            None => (0, &line[..pos]),
        };

        let mut completions = vec![];

        // Expand ~ in prefix
        let expanded = expand_tilde(prefix);
        let path = Path::new(&expanded);

        // Determine directory to list and prefix to filter by
        let (dir, file_prefix) = if path.is_dir() {
            (path, "")
        } else {
            (path.parent().unwrap_or_else(|| Path::new(".")), path.file_name().and_then(|f| f.to_str()).unwrap_or(""))
        };

        // Read directory entries, collect only directories matching prefix
        if let Ok(entries) = dir.read_dir() {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.starts_with(file_prefix) {
                                let mut display_name = name.to_string();
                                display_name.push('/'); // Add trailing slash for directories
                                completions.push(Pair {
                                    display: display_name.clone(),
                                    replacement: display_name,
                                });
                            }
                        }
                    }
                }
            }
        }

        completions.sort_by(|a, b| a.display.cmp(&b.display));

        Ok((start, completions))
    }
}


impl Hinter for RshHelper {
    type Hint = String;
}
impl Highlighter for RshHelper {}
impl Validator for RshHelper {}
impl Helper for RshHelper {}

fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return path.replacen('~', &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

fn ensure_path() {
     unsafe {std::env::set_var("PATH", "/bin:/usr/bin");}
}

fn load_rshrc(config: &mut RshConfig) {
    let rc_path = expand_tilde("~/.rshrc");
    if let Ok(file) = File::open(rc_path) {
        for line in BufReader::new(file).lines().flatten() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(stripped) = line.strip_prefix("alias ") {
                if let Some((k, v)) = stripped.split_once('=') {
                    config
                        .aliases
                        .insert(k.trim().to_string(), v.trim().trim_matches('"').trim_matches('\'').to_string());
                }
            } else if let Some(stripped) = line.strip_prefix("export ") {
                if let Some((k, v)) = stripped.split_once('=') {
                    config.env_vars.insert(k.trim().to_string(), v.trim().to_string());
                     unsafe {std::env::set_var(k.trim(), v.trim());}
                }
            } else if let Some(prompt) = line.strip_prefix("prompt ") {
                config.prompt = prompt.to_string();
            } else if let Some(on_start) = line.strip_prefix("func on_start ") {
                config.on_start.push(on_start.to_string());
            } else if let Some(on_exit) = line.strip_prefix("func on_exit ") {
                config.on_exit.push(on_exit.to_string());
            } else if let Some(stripped) = line.strip_prefix("func ") {
                if let Some((name, body)) = stripped.split_once(' ') {
                    config.functions.insert(name.trim().to_string(), body.trim().to_string());
                }
            }
        }
    }
}

fn run_function(name: &str, config: &RshConfig) {
    if let Some(body) = config.functions.get(name) {
        let _ = Command::new("sh")
            .env("PATH", "/bin:/usr/bin") // enforce PATH here too
            .arg("-c")
            .arg(body)
            .status();
    }
}

fn expand_alias(line: &str, aliases: &HashMap<String, String>) -> String {
    let mut parts = line.split_whitespace();
    if let Some(cmd) = parts.next() {
        if let Some(replacement) = aliases.get(cmd) {
            let rest = parts.collect::<Vec<_>>().join(" ");
            if rest.is_empty() {
                return replacement.to_string();
            } else {
                return format!("{} {}", replacement, rest);
            }
        }
    }
    line.to_string()
}

fn main() {
    ensure_path();
    println!("Before loading config, PATH: {}", std::env::var("PATH").unwrap());

    let mut config = RshConfig::default();
    load_rshrc(&mut config);

    ensure_path();

    if config.prompt.is_empty() {
        config.prompt = "🦀❓> ".to_string();
    }

    println!("After loading config, PATH: {}", std::env::var("PATH").unwrap());

    let builtins = vec![
        "cd", "exit", "help", "alias", "unalias",
        "export", "set", "source", "func",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();

    let helper = RshHelper {
        commands: builtins.clone(),
        aliases: config.aliases.keys().cloned().collect(),
    };

    let rl_config = RLConfig::builder().auto_add_history(true).build();
    let mut rl = Editor::with_config(rl_config).unwrap();
    rl.set_helper(Some(helper));

// gotta enforce yk
    for cmd in &config.on_start {
        let _ = Command::new("sh")
            .env("PATH", "/bin:/usr/bin")
            .arg("-c")
            .arg(cmd)
            .status();
    }

    loop {
        let line = rl.readline(&config.prompt).unwrap_or_else(|e| {
            if matches!(e, ReadlineError::Eof | ReadlineError::Interrupted) {
                println!();
                std::process::exit(0);
            }
            eprintln!("Readline error: {}", e);
            String::new()
        });

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.split_whitespace();
        if let Some(cmd) = parts.next() {
            match cmd {
                "exit" => {
                    for cmd in &config.on_exit {
                        let _ = Command::new("sh")
                            .env("PATH", "/bin:/usr/bin")
                            .arg("-c")
                            .arg(cmd)
                            .status();
                    }
                    break;
                }
                "cd" => {
                    let target = parts.next().unwrap_or("~");
                    let path = expand_tilde(target);
                    if let Err(e) = env::set_current_dir(path) {
                        eprintln!("cd: {}", e);
                    }
                }
                "help" => show_help(),
                "alias" => {
                    for (k, v) in &config.aliases {
                        println!("alias {}='{}'", k, v);
                    }
                }
                "unalias" => {
                    if let Some(name) = parts.next() {
                        config.aliases.remove(name);
                    } else {
                        eprintln!("unalias: missing argument");
                    }
                }
                "export" => {
                    if let Some(pair) = parts.next() {
                        if let Some((k, v)) = pair.split_once('=') {
                             unsafe {std::env::set_var(k, v);}
                            config.env_vars.insert(k.to_string(), v.to_string());
                        } else {
                            eprintln!("export: invalid syntax");
                        }
                    } else {
                        eprintln!("export: missing argument");
                    }
                }
                "set" => {
                    for (k, v) in &config.env_vars {
                        println!("{}={}", k, v);
                    }
                }
                "source" => {
                    if let Some(file) = parts.next() {
                        let path = expand_tilde(file);
                        if Path::new(&path).exists() {
                            let mut source_config = RshConfig::default();
                            load_rshrc(&mut source_config);
                            config.aliases.extend(source_config.aliases);
                            config.functions.extend(source_config.functions);
                            config.env_vars.extend(source_config.env_vars);
                        } else {
                            eprintln!("source: file not found");
                        }
                    } else {
                        eprintln!("source: missing file");
                    }
                }
                "func" => {
                    if let Some(fname) = parts.next() {
                        run_function(fname, &config);
                    } else {
                        eprintln!("func: missing function name");
                    }
                }
                _ => {

                    let expanded_line = expand_alias(line, &config.aliases);
                    let status = Command::new("sh")
                        .env("PATH", "/bin:/usr/bin")
                        .arg("-c")
                        .arg(expanded_line)
                        .status();

                    if let Err(e) = status {
                        eprintln!("command failed: {}", e);
                    }
                }
            }
        }
    }
}
