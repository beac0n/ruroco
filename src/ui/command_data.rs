use crate::ui::colors::GRAY;
use crate::ui::rust_slint_bridge::CommandData;

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
        address: address.into(),
        command: command.into(),
        permissive,
        ip: ip.into(),
        ipv4,
        ipv6,
        name: "".into(),
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
    data.name = name.into();
    data
}
