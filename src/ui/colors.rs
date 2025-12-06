use crate::ui::rust_slint_bridge::CommandData;
use slint::Color;

pub const GREEN: Color = Color::from_rgb_u8(56, 142, 60);
pub const RED: Color = Color::from_rgb_u8(211, 47, 47);
pub const GRAY: Color = Color::from_rgb_u8(204, 204, 204);

pub fn change_color(mut data: CommandData, color: Color) -> CommandData {
    data.color = color;
    data
}
