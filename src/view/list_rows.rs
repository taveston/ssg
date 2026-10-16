//! The rows of a list panel, shared by the sidebar's outline (INS-05) and
//! a multi-marker's tray (MUL-09): a row node indented one step per
//! level, highlighted while selected, and the toggle slot at its start
//! that shows whether its members are open. Each panel keeps its own
//! sizes in a `RowLayout`; what follows the toggle is the panel's own.

use bevy::prelude::*;

use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::symbols::draw::{Tint, TintSlot};

/// The toggle glyphs: collapsed and expanded.
pub const COLLAPSED_GLYPH: &str = ">";
pub const EXPANDED_GLYPH: &str = "v";

/// How one panel lays out its rows, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowLayout {
    /// A fixed height for every row, which then never shrinks; `None`
    /// fits a row to what it holds.
    pub height: Option<f32>,
    /// Space above and below what a row holds.
    pub pad_y: f32,
    /// Space inside a row's left and right ends.
    pub side: f32,
    /// One level of indent.
    pub indent: f32,
    /// Whether the indent is inside the row, so its highlight spans it,
    /// or before the row, so its highlight starts after it.
    pub indent_inside: bool,
    /// The space between the toggle and the parts after it.
    pub gap: f32,
    /// The toggle's slot, so names line up whether or not a row has one.
    pub toggle: f32,
    /// The toggle glyph's size.
    pub toggle_font: f32,
}

/// The background of a row: the list highlight while it is selected.
pub fn row_highlight(selected: bool) -> BackgroundColor {
    BackgroundColor(if selected {
        colors::LIST_HIGHLIGHT
    } else {
        Color::NONE
    })
}

/// The toggle glyph for a row that can open (`Some(open)`), or none.
pub fn toggle_glyph(open: Option<bool>) -> &'static str {
    match open {
        Some(true) => EXPANDED_GLYPH,
        Some(false) => COLLAPSED_GLYPH,
        None => "",
    }
}

impl RowLayout {
    /// A row `depth` levels in, laid out left to right and centred
    /// across, highlighted while `selected`.
    pub fn row(&self, depth: usize, selected: bool) -> (Node, BackgroundColor) {
        let indent = depth as f32 * self.indent;
        let (inside, before) = if self.indent_inside {
            (indent, 0.0)
        } else {
            (0.0, indent)
        };
        let node = Node {
            height: self.height.map_or(Val::Auto, px),
            flex_shrink: if self.height.is_some() { 0.0 } else { 1.0 },
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(self.gap),
            padding: UiRect {
                left: px(self.side + inside),
                right: px(self.side),
                top: px(self.pad_y),
                bottom: px(self.pad_y),
            },
            margin: UiRect::left(px(before)),
            ..default()
        };
        (node, row_highlight(selected))
    }

    /// The toggle slot at a row's start, with its glyph for a row that
    /// can open (`Some(open)`) and empty otherwise.
    pub fn spawn_toggle(
        &self,
        row: &mut ChildSpawnerCommands,
        fonts: &Fonts,
        open: Option<bool>,
    ) -> Entity {
        row.spawn((
            Node {
                width: px(self.toggle),
                ..default()
            },
            Text::new(toggle_glyph(open)),
            fonts.data(self.toggle_font),
            TextColor(colors::PANEL_BORDER),
            Tint::new(colors::PANEL_BORDER, TintSlot::Text),
        ))
        .id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAYOUT: RowLayout = RowLayout {
        height: None,
        pad_y: 2.0,
        side: 10.0,
        indent: 14.0,
        indent_inside: true,
        gap: 6.0,
        toggle: 14.0,
        toggle_font: 11.0,
    };

    /// INS-05 / MUL-09: each level indents a row one step, inside the row
    /// so the highlight spans it, or before it so the highlight starts
    /// after it; a selected row is highlighted.
    #[test]
    fn rows_indent_one_step_per_level() {
        let (node, background) = LAYOUT.row(2, true);
        assert_eq!(node.padding.left, px(10.0 + 28.0));
        assert_eq!(node.margin.left, px(0.0));
        assert_eq!(background.0, colors::LIST_HIGHLIGHT);
        let before = RowLayout {
            indent_inside: false,
            ..LAYOUT
        };
        let (node, background) = before.row(2, false);
        assert_eq!((node.padding.left, node.margin.left), (px(10.0), px(28.0)));
        assert_eq!(background.0, Color::NONE);
    }

    /// MUL-09 / INS-05: a fixed-height row never shrinks; one fitted to
    /// what it holds does as a node does.
    #[test]
    fn a_fixed_height_row_never_shrinks() {
        let fixed = RowLayout {
            height: Some(24.0),
            ..LAYOUT
        };
        let (node, _) = fixed.row(0, false);
        assert_eq!((node.height, node.flex_shrink), (px(24.0), 0.0));
        let (node, _) = LAYOUT.row(0, false);
        assert_eq!((node.height, node.flex_shrink), (Val::Auto, 1.0));
    }
}
