use crate::ui::rust_slint_bridge::CommandData;
use slint::Color;

pub(crate) const GREEN: Color = Color::from_rgb_u8(56, 142, 60);
pub(crate) const RED: Color = Color::from_rgb_u8(211, 47, 47);
pub(crate) const GRAY: Color = Color::from_rgb_u8(204, 204, 204);

pub(crate) fn change_color(mut data: CommandData, color: Color) -> CommandData {
    data.color = color;
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_color() {
        let data = CommandData {
            address: "".into(),
            command: "".into(),
            ip: "".into(),
            ipv4: false,
            ipv6: false,
            permissive: false,
            name: "".into(),
            color: GRAY,
        };
        let result = change_color(data, GREEN);
        assert_eq!(result.color, GREEN);
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(GREEN, Color::from_rgb_u8(56, 142, 60));
        assert_eq!(RED, Color::from_rgb_u8(211, 47, 47));
        assert_eq!(GRAY, Color::from_rgb_u8(204, 204, 204));
    }
}
