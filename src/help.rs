pub fn show_help() {
    println!(
"rsh: the Rust Shell
These shell commands are defined internally. Type 'help' to see this list.

Built‑in commands:
  cd [dir]         Change current directory
  exit             Exit the shell
  help             Show this help message
  alias            List defined aliases
  unalias NAME     Remove an alias
  export VAR=VAL   Set environment variable
  set              List all environment variables
  source FILE      Execute commands from FILE
  func NAME ARGS   Invoke a shell function

Shell customization via ~/.rshrc:
  # alias name='command'     Define an alias
  # unalias name             Remove an alias
  # export VAR=VAL           Set environment variable
  # func name command        Define a shell function
  # plugin name              Load a plugin (stub)
  # theme NAME               Select prompt theme
  # prompt STRING            Customize the prompt
  # func on_start CMD        Run CMD on startup
  # func on_exit CMD         Run CMD on exit

Use `source ~/.rshrc` to reload configuration.

rsh is an aggressively customizable shell.
Report bugs or submit features: (you know where)"
    );
}
