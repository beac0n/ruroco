use crate::ui::rust_slint_bridge::CommandData;
use crate::ui::saved_command_list::CommandsList;
use slint::{Color, SharedString};

pub const GREEN: Color = Color::from_rgb_u8(56, 142, 60);
pub const RED: Color = Color::from_rgb_u8(211, 47, 47);
pub const GRAY: Color = Color::from_rgb_u8(204, 204, 204);

pub fn create_command_tuple(command: &str) -> CommandData {
    CommandData {
        command: SharedString::from(command),
        name: SharedString::from(CommandsList::command_to_name(command)),
        color: Color::from_rgb_u8(204, 204, 204),
    }
}
