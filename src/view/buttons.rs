//! Console buttons (BTN-01, BTN-02): one look for every button in the top
//! bar and the inset. A button is a node with a `ConsoleButton` and one
//! text child; `apply_button_looks` paints both from the button's style,
//! so callers change a button by changing its `ConsoleButton` alone.

use bevy::prelude::*;

use super::colors;
use super::fonts::Fonts;
use super::tunables::Tunables;

/// BTN-01: the three button styles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonStyle {
    /// Filled in the neutral colour, with ground-colour text.
    Primary,
    /// Outlined in the secondary colour, with neutral text.
    Secondary,
    /// Text only, in the secondary colour.
    Quiet,
}

/// A button's style and state, read by `apply_button_looks`.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ConsoleButton {
    pub style: ButtonStyle,
    /// Drawn dimmed; the caller ignores its clicks.
    pub disabled: bool,
    /// Its action can't be undone, so its outline is orange (BTN-02).
    pub irreversible: bool,
    /// Drawn as the selected option of a choice (PLNR-14), as a preset
    /// the planner's sliders sit on is (PLNR-29).
    pub selected: bool,
}

impl ConsoleButton {
    pub fn new(style: ButtonStyle) -> Self {
        Self {
            style,
            disabled: false,
            irreversible: false,
            selected: false,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// The colours a button is painted in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ButtonLook {
    pub background: Color,
    pub border: Color,
    pub text: Color,
}

/// A disabled button's parts are drawn at this opacity.
const DISABLED_ALPHA: f32 = 0.4;
const PADDING_X_PX: f32 = 12.0;
const PADDING_Y_PX: f32 = 5.0;

/// BTN-01 and BTN-02: a button's colours. Only the warning orange of an
/// irreversible outline ever leaves the neutral and secondary colours.
pub fn button_look(button: ConsoleButton) -> ButtonLook {
    let outline = if button.irreversible {
        colors::WARNING
    } else {
        colors::SECONDARY
    };
    let look = match button.style {
        ButtonStyle::Primary => ButtonLook {
            background: colors::NEUTRAL,
            border: if button.irreversible {
                outline
            } else {
                colors::NEUTRAL
            },
            text: colors::GROUND,
        },
        ButtonStyle::Secondary => ButtonLook {
            background: Color::NONE,
            border: outline,
            text: colors::NEUTRAL,
        },
        ButtonStyle::Quiet => ButtonLook {
            background: Color::NONE,
            border: if button.irreversible {
                outline
            } else {
                Color::NONE
            },
            text: colors::SECONDARY,
        },
    };
    if !button.disabled {
        return look;
    }
    let dim = |c: Color| c.with_alpha(c.alpha() * DISABLED_ALPHA);
    ButtonLook {
        background: dim(look.background),
        border: dim(look.border),
        text: dim(look.text),
    }
}

/// PLNR-14 / PLNR-29: the selected option of a choice: filled with the
/// neutral colour mixed into the ground at `fill_mix`, with neutral text.
pub fn selected_look(fill_mix: f32) -> ButtonLook {
    let fill = colors::mix(colors::GROUND, colors::NEUTRAL, fill_mix);
    ButtonLook {
        background: fill,
        border: fill,
        text: colors::NEUTRAL,
    }
}

/// Spawns a button with its label. Every style keeps the same border width,
/// so switching style never shifts the layout.
pub fn spawn_button<'a>(
    parent: &'a mut ChildSpawnerCommands,
    fonts: &Fonts,
    label: &str,
    font_px: f32,
    button: ConsoleButton,
) -> EntityCommands<'a> {
    let look = button_look(button);
    let mut entity = parent.spawn((
        button,
        Node {
            padding: UiRect::axes(px(PADDING_X_PX), px(PADDING_Y_PX)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BackgroundColor(look.background),
        BorderColor::all(look.border),
    ));
    entity.with_children(|b| {
        b.spawn((Text::new(label), fonts.label(font_px), TextColor(look.text)));
    });
    entity
}

pub struct ButtonsPlugin;

impl Plugin for ButtonsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            apply_button_looks.before(bevy::ui::UiSystems::Prepare),
        );
    }
}

/// Paints every button from its `ConsoleButton`, with the outline
/// `button_stroke_px` wide (BTN-01); a selected one as the selected
/// option of a choice (PLNR-14).
pub fn apply_button_looks(
    tunables: Res<Tunables>,
    mut buttons: Query<(
        &ConsoleButton,
        &Children,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut texts: Query<&mut TextColor>,
) {
    let border = UiRect::all(px(tunables.button_stroke_px));
    for (button, children, mut node, mut background, mut border_color) in &mut buttons {
        let look = if button.selected && !button.disabled {
            selected_look(tunables.segment_fill_mix)
        } else {
            button_look(*button)
        };
        if node.border != border {
            node.border = border;
        }
        background.set_if_neq(BackgroundColor(look.background));
        border_color.set_if_neq(BorderColor::all(look.border));
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child) {
                text.set_if_neq(TextColor(look.text));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_affiliation_colour(c: Color) -> bool {
        let opaque = c.with_alpha(1.0);
        [
            colors::OWN,
            colors::CIVILIAN,
            colors::NEUTRAL_GREEN,
            colors::HOSTILE,
        ]
        .contains(&opaque)
    }

    /// BTN-01: primary is filled neutral with ground text, secondary is
    /// outlined in the secondary colour with neutral text, quiet is text
    /// alone in the secondary colour.
    #[test]
    fn each_style_has_its_fill_outline_and_text() {
        let primary = button_look(ConsoleButton::new(ButtonStyle::Primary));
        assert_eq!(primary.background, colors::NEUTRAL);
        assert_eq!(primary.text, colors::GROUND);

        let secondary = button_look(ConsoleButton::new(ButtonStyle::Secondary));
        assert_eq!(secondary.background, Color::NONE);
        assert_eq!(secondary.border, colors::SECONDARY);
        assert_eq!(secondary.text, colors::NEUTRAL);

        let quiet = button_look(ConsoleButton::new(ButtonStyle::Quiet));
        assert_eq!(quiet.background, Color::NONE);
        assert_eq!(quiet.border, Color::NONE);
        assert_eq!(quiet.text, colors::SECONDARY);
    }

    /// BTN-02: no style uses an affiliation colour or orange, unless the
    /// action can't be undone, which outlines it in orange.
    #[test]
    fn only_irreversible_buttons_use_orange_and_none_use_affiliation_colours() {
        for style in [
            ButtonStyle::Primary,
            ButtonStyle::Secondary,
            ButtonStyle::Quiet,
        ] {
            for disabled in [false, true] {
                let mut button = ConsoleButton::new(style);
                button.disabled = disabled;
                let look = button_look(button);
                for c in [look.background, look.border, look.text] {
                    assert!(!is_affiliation_colour(c), "{style:?}");
                    assert_ne!(c.with_alpha(1.0), colors::WARNING, "{style:?}");
                }
                button.irreversible = true;
                let look = button_look(button);
                assert_eq!(look.border.with_alpha(1.0), colors::WARNING, "{style:?}");
            }
        }
    }

    /// BTN-01: the outline is `button_stroke_px` wide.
    #[test]
    fn the_outline_width_follows_the_tunable() {
        let mut app = App::new();
        let tunables = Tunables {
            button_stroke_px: 2.0,
            ..Tunables::default()
        };
        app.insert_resource(tunables);
        app.add_systems(Update, apply_button_looks);
        let text = app.world_mut().spawn(TextColor(Color::WHITE)).id();
        let button = app
            .world_mut()
            .spawn((
                ConsoleButton::new(ButtonStyle::Secondary),
                Node::default(),
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
            ))
            .add_children(&[text])
            .id();
        app.update();
        let node = app.world().get::<Node>(button).unwrap();
        assert_eq!(node.border, UiRect::all(px(2.0)));
        let border = app.world().get::<BorderColor>(button).unwrap();
        assert_eq!(border.top, colors::SECONDARY);
        let text = app.world().get::<TextColor>(text).unwrap();
        assert_eq!(text.0, colors::NEUTRAL);
    }
}
