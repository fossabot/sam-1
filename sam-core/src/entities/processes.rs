use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ShellCommand<T: Clone> {
    command: T,
}

fn current_shell_or_sh() -> String {
    env::var("SHELL").unwrap_or_else(|_| String::from("/bin/sh"))
}

impl<T> ShellCommand<T>
where
    T: Clone,
{
    pub fn new(command: T) -> Self {
        Self { command }
    }

    pub fn make_command<U>(u: U) -> Command
    where
        U: Into<ShellCommand<T>>,
        T: AsRef<OsStr>,
    {
        let sh_cmd: ShellCommand<T> = u.into();
        sh_cmd.into()
    }
    pub fn value(&self) -> &T {
        &self.command
    }
}

impl ShellCommand<String> {
    pub fn replace_env_vars_in_command(
        &self,
        variables: &HashMap<String, String>,
    ) -> std::io::Result<ShellCommand<String>> {
        let command_escaped = shellwords::escape(self.command.as_str());
        let s = format!("echo \"{}\"|envsubst", command_escaped);
        let shell_cmd = ShellCommand::<String>::new(s);
        let mut cmd: Command = shell_cmd.into();
        cmd.envs(variables);
        let out = cmd.output()?;
        let new_cmd = String::from_utf8_lossy(out.stdout.as_slice()).replace("\n", "");
        Ok(ShellCommand::<String>::new(new_cmd))
    }
}

impl From<&'_ str> for ShellCommand<String> {
    fn from(s: &'_ str) -> Self {
        Self::new(s.to_string())
    }
}

#[allow(clippy::from_over_into)]
impl<T> Into<Command> for ShellCommand<T>
where
    T: AsRef<OsStr> + Clone,
{
    fn into(self) -> Command {
        let mut command = Command::new(current_shell_or_sh());
        command.arg("-c").arg(self.command);
        command.envs(env::vars());
        let curr_dir = std::env::current_dir();
        if let Ok(dir) = curr_dir {
            command.current_dir(dir);
        }
        command
    }
}
