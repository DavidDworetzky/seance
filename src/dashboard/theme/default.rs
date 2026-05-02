use ratatui::style::Color;

pub const HEADER_TEXT: Color = Color::Cyan;
pub const MUTED_TEXT: Color = Color::DarkGray;
pub const TABLE_HEADER_TEXT: Color = Color::Yellow;
pub const TABLE_ROW_TEXT: Color = Color::Gray;
pub const TABLE_SELECTED_ROW_TEXT: Color = Color::Rgb(50, 205, 50);
pub const PREVIEW_TEXT: Color = Color::White;
pub const INPUT_MODE_BORDER: Color = Color::Green;
pub const ERROR_TEXT: Color = Color::Red;
pub const REPO_PICKER_SELECTED_TEXT: Color = TABLE_SELECTED_ROW_TEXT;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_worktree_row_uses_lime_green() {
        assert_eq!(TABLE_SELECTED_ROW_TEXT, Color::Rgb(50, 205, 50));
    }

    #[test]
    fn repo_picker_selection_matches_worktree_selection() {
        assert_eq!(REPO_PICKER_SELECTED_TEXT, TABLE_SELECTED_ROW_TEXT);
    }
}
