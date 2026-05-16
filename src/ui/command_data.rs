use eframe::egui::Color32;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CommandData {
    pub(crate) address: String,
    pub(crate) command: String,
    pub(crate) permissive: bool,
    pub(crate) ip: String,
    pub(crate) ipv4: bool,
    pub(crate) ipv6: bool,
    pub(crate) name: String,
    pub(crate) color: Color32,
}

pub(crate) fn data_to_command(data: &CommandData, key: Option<String>) -> String {
    let mut command = String::new();

    command.push_str("send ");
    if !data.address.trim().is_empty() {
        command.push_str(&format!("--address {} ", data.address));
    }
    if !data.command.trim().is_empty() {
        command.push_str(&format!("--command {} ", data.command));
    }

    if !data.ip.trim().is_empty() {
        command.push_str(&format!("--ip {} ", data.ip));
    }

    if data.ipv4 {
        command.push_str("--ipv4 ");
    }
    if data.ipv6 {
        command.push_str("--ipv6 ");
    }
    if data.permissive {
        command.push_str("--permissive ");
    }

    if let Some(k) = key {
        command.push_str("--key ");
        command.push_str(&k);
    }

    command.trim_end().to_string()
}

pub(crate) fn command_to_data(input: &str) -> CommandData {
    use crate::ui::colors::GRAY;

    let mut address = "";
    let mut command = "";
    let mut ip = "";
    let mut ipv4 = false;
    let mut ipv6 = false;
    let mut permissive = false;

    let parts: Vec<&str> = input.split_whitespace().collect();
    let parts_len = parts.len();
    let mut i = 0;
    while i < parts_len {
        match parts[i] {
            "--address" if i + 1 < parts_len => {
                i += 1;
                address = parts[i];
            }
            "--command" if i + 1 < parts_len => {
                i += 1;
                command = parts[i];
            }
            "--ip" if i + 1 < parts_len => {
                i += 1;
                ip = parts[i];
            }
            "--ipv4" => ipv4 = true,
            "--ipv6" => ipv6 = true,
            "--permissive" => permissive = true,
            _ => {}
        }
        i += 1;
    }

    add_command_name(CommandData {
        address: address.to_string(),
        command: command.to_string(),
        permissive,
        ip: ip.to_string(),
        ipv4,
        ipv6,
        name: String::new(),
        color: GRAY,
    })
}

pub(crate) fn add_command_name(mut data: CommandData) -> CommandData {
    let name = format!(
        "{}@{}{}{}{}",
        data.command,
        data.address,
        if data.permissive { " permissive" } else { "" },
        if data.ipv4 { " ipv4" } else { "" },
        if data.ipv6 { " ipv6" } else { "" }
    );
    data.name = name;
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::colors::GRAY;

    fn make_cmd(
        address: &str,
        command: &str,
        ip: &str,
        ipv4: bool,
        ipv6: bool,
        permissive: bool,
    ) -> CommandData {
        CommandData {
            address: address.to_string(),
            command: command.to_string(),
            ip: ip.to_string(),
            ipv4,
            ipv6,
            permissive,
            name: String::new(),
            color: GRAY,
        }
    }

    #[test]
    fn test_data_to_command_full() {
        let data = make_cmd("127.0.0.1:80", "restart", "10.0.0.1", true, true, true);
        let result = data_to_command(&data, Some("mykey123".to_string()));
        assert_eq!(
            result,
            "send --address 127.0.0.1:80 --command restart --ip 10.0.0.1 --ipv4 --ipv6 --permissive --key mykey123"
        );
    }

    #[test]
    fn test_data_to_command_minimal() {
        let data = make_cmd("", "", "", false, false, false);
        let result = data_to_command(&data, None);
        assert_eq!(result, "send");
    }

    #[test]
    fn test_data_to_command_no_key() {
        let data = make_cmd("host:80", "default", "", false, false, false);
        let result = data_to_command(&data, None);
        assert_eq!(result, "send --address host:80 --command default");
    }

    #[test]
    fn test_data_to_command_ipv4_only() {
        let data = make_cmd("host:80", "cmd", "", true, false, false);
        let result = data_to_command(&data, None);
        assert!(result.contains("--ipv4"));
        assert!(!result.contains("--ipv6"));
    }

    #[test]
    fn test_data_to_command_ipv6_only() {
        let data = make_cmd("host:80", "cmd", "", false, true, false);
        let result = data_to_command(&data, None);
        assert!(!result.contains("--ipv4"));
        assert!(result.contains("--ipv6"));
    }

    #[test]
    fn test_command_to_data_full() {
        let input = "send --address 127.0.0.1:80 --command restart --ip 10.0.0.1 --ipv4 --ipv6 --permissive";
        let data = command_to_data(input);
        assert_eq!(data.address, "127.0.0.1:80");
        assert_eq!(data.command, "restart");
        assert_eq!(data.ip, "10.0.0.1");
        assert!(data.ipv4);
        assert!(data.ipv6);
        assert!(data.permissive);
    }

    #[test]
    fn test_command_to_data_minimal() {
        let input = "send";
        let data = command_to_data(input);
        assert_eq!(data.address, "");
        assert_eq!(data.command, "");
        assert_eq!(data.ip, "");
        assert!(!data.ipv4);
        assert!(!data.ipv6);
        assert!(!data.permissive);
    }

    #[test]
    fn test_command_to_data_unknown_flags() {
        let input = "send --unknown flag --address host:80";
        let data = command_to_data(input);
        assert_eq!(data.address, "host:80");
    }

    #[test]
    fn test_command_to_data_address_at_end() {
        let input = "--command cmd --address host:80";
        let data = command_to_data(input);
        assert_eq!(data.address, "host:80");
        assert_eq!(data.command, "cmd");
    }

    #[test]
    fn test_roundtrip_data_to_command_to_data() {
        let original = make_cmd("host:8080", "deploy", "192.168.1.1", true, false, true);
        let cmd_str = data_to_command(&original, None);
        let parsed = command_to_data(&cmd_str);
        assert_eq!(parsed.address, "host:8080");
        assert_eq!(parsed.command, "deploy");
        assert_eq!(parsed.ip, "192.168.1.1");
        assert!(parsed.ipv4);
        assert!(!parsed.ipv6);
        assert!(parsed.permissive);
    }

    #[test]
    fn test_add_command_name_basic() {
        let data = make_cmd("host:80", "restart", "", false, false, false);
        let result = add_command_name(data);
        assert_eq!(result.name, "restart@host:80");
    }

    #[test]
    fn test_add_command_name_with_flags() {
        let data = make_cmd("host:80", "cmd", "", true, true, true);
        let result = add_command_name(data);
        assert_eq!(result.name, "cmd@host:80 permissive ipv4 ipv6");
    }

    #[test]
    fn test_add_command_name_permissive_only() {
        let data = make_cmd("h:80", "c", "", false, false, true);
        let result = add_command_name(data);
        assert_eq!(result.name, "c@h:80 permissive");
    }
}
