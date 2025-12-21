use crate::ui::rust_slint_bridge::CommandData;
use slint::Color;

pub(crate) const GREEN: Color = Color::from_rgb_u8(56, 142, 60);
pub(crate) const RED: Color = Color::from_rgb_u8(211, 47, 47);
pub(crate) const GRAY: Color = Color::from_rgb_u8(204, 204, 204);

pub(crate) fn change_color(mut data: CommandData, color: Color) -> CommandData {
    data.color = color;
    data
}
