use log::debug;
use std::{fs, process::Command};

pub fn get_system_timezone() -> Option<String> {
    get_timezone_with_timedatectl().or(get_timezone_with_etc_localtime())
}

fn get_timezone_with_timedatectl() -> Option<String> {
    debug!("Attempting to get timezone with 'timedatectl'");
    get_command_output(Command::new("timedatectl").args(["show", "-P", "Timezone"]))
}

fn get_timezone_with_etc_localtime() -> Option<String> {
    debug!("Attempting to get timezone from '/etc/localtime'");

    let target = fs::read_link("/etc/localtime").ok()?;
    let second = target.file_name()?;
    let first = target.parent()?.file_name()?;

    Some(format!("{}/{}", first.to_str()?, second.to_str()?))
}

fn get_command_output(cmd: &mut Command) -> Option<String> {
    let mut out = String::from_utf8(cmd.output().ok()?.stdout).ok()?;
    out.pop();

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::get_timezone_with_timedatectl;
    use crate::server::tz::get_timezone_with_etc_localtime;

    #[test]
    fn check_timedatectl_method() {
        assert!(get_timezone_with_timedatectl().is_some());
    }

    #[test]
    fn check_etc_localtime() {
        assert!(get_timezone_with_etc_localtime().is_some())
    }
}
