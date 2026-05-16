use eframe::egui::Color32;

pub(crate) const GREEN: Color32 = Color32::from_rgb(56, 142, 60);
pub(crate) const RED: Color32 = Color32::from_rgb(211, 47, 47);
pub(crate) const GRAY: Color32 = Color32::from_rgb(204, 204, 204);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_constants() {
        assert_eq!(GREEN, Color32::from_rgb(56, 142, 60));
        assert_eq!(RED, Color32::from_rgb(211, 47, 47));
        assert_eq!(GRAY, Color32::from_rgb(204, 204, 204));
    }
}
