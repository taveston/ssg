//! Label tooltips (PLNR-35, PLNR-91, PLNR-94, PLNR-95): a label that
//! carries a `Tooltip` shows its text in a small box by the pointer once
//! the pointer has rested on it for `tooltip_delay_ms`. A node that carries
//! `Stretches` (a slider's track) shows the text of the stretch under
//! the pointer, and nothing between its stretches. These are labels'
//! tooltips, not markers': nothing on the map has one (HOV-01).

use bevy::ecs::system::SystemParam;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;

use super::slider::Stretch;
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::layers;
use crate::view::tunables::Tunables;

/// On a label: the text its tooltip shows.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tooltip(pub &'static str);

/// PLNR-35: on a node: stretches of its width, each with the text its
/// tooltip shows while the pointer is over it. Written as what the node
/// shows changes; empty, it shows none.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct Stretches(pub Vec<Stretch>);

impl Stretches {
    /// The text of the stretch at `fraction` of the node's width (0 at
    /// its left edge, 1 at its right), if any.
    pub fn text_at(&self, fraction: f32) -> Option<&str> {
        self.0
            .iter()
            .find(|s| (s.from..=s.to).contains(&fraction))
            .map(|s| s.text.as_str())
    }
}

/// The one tooltip box, shown by `show_tooltips`.
#[derive(Component)]
pub struct TooltipBox;

/// Which label the pointer rests on, the text it shows there, and since
/// when (real seconds).
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct TooltipHover {
    pub label: Option<(Entity, String, f32)>,
}

/// The tooltip box's width, and its offset below the pointer.
const BOX_WIDTH_PX: f32 = 240.0;
const BELOW_POINTER_PX: f32 = 18.0;
const EDGE_PX: f32 = 4.0;
const PADDING_PX: f32 = 6.0;

/// PLNR-91 / PLNR-94: a tooltip shows once the pointer has rested on its
/// label for the delay.
pub fn tooltip_due(rested_secs: f32, delay_ms: f32) -> bool {
    rested_secs * 1000.0 >= delay_ms
}

/// The box, hidden until a tooltip is due.
pub fn spawn_tooltip_box(mut commands: Commands, fonts: Res<Fonts>, tunables: Res<Tunables>) {
    commands
        .spawn((
            TooltipBox,
            GlobalZIndex(layers::PANELS + 1),
            Node {
                position_type: PositionType::Absolute,
                width: px(BOX_WIDTH_PX),
                padding: UiRect::all(px(PADDING_PX)),
                border: UiRect::all(px(1)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(colors::PANEL),
            BorderColor::all(colors::PANEL_BORDER),
            Pickable::IGNORE,
        ))
        .with_children(|tip| {
            tip.spawn((
                Text::new(""),
                fonts.label(tunables.sidebar_text_px),
                TextColor(colors::NEUTRAL),
                Pickable::IGNORE,
            ));
        });
}

/// The labels that carry a tooltip, and what the mouse is over.
#[derive(SystemParam)]
pub struct TooltipLabels<'w, 's> {
    hover: Option<Res<'w, HoverMap>>,
    labels: Query<'w, 's, &'static Tooltip>,
    stretches: Query<'w, 's, &'static Stretches>,
    parents: Query<'w, 's, &'static ChildOf>,
}

impl TooltipLabels<'_, '_> {
    /// The label with a tooltip that the mouse is over, if any, and its
    /// text: the hit itself or one of its ancestors with a `Tooltip`, or
    /// the hit itself with a stretch under the pointer.
    fn hovered(&self) -> Option<(Entity, String)> {
        let hits = self.hover.as_deref()?.0.get(&PointerId::Mouse)?;
        hits.iter().find_map(|(&hit, data)| {
            // The hit's position is relative to its node's centre, in
            // node widths.
            let fraction = data.position.map(|at| at.x + 0.5);
            if let (Ok(stretches), Some(fraction)) = (self.stretches.get(hit), fraction)
                && let Some(text) = stretches.text_at(fraction)
            {
                return Some((hit, text.to_string()));
            }
            std::iter::successors(Some(hit), |&node| {
                self.parents.get(node).ok().map(ChildOf::parent)
            })
            .find_map(|node| Some((node, self.labels.get(node).ok()?.0.to_string())))
        })
    }
}

/// The tooltip box and its text.
#[derive(SystemParam)]
pub struct TooltipBoxes<'w, 's> {
    boxes: Query<'w, 's, (&'static mut Node, &'static Children), With<TooltipBox>>,
    texts: Query<'w, 's, &'static mut Text>,
}

/// PLNR-35 / PLNR-91 / PLNR-94: times how long the pointer rests on a
/// label, or on one stretch of a track, and shows its tooltip by the
/// pointer once it is due.
pub fn show_tooltips(
    time: Res<Time<Real>>,
    tunables: Res<Tunables>,
    labels: TooltipLabels,
    windows: Query<&Window>,
    mut rest: ResMut<TooltipHover>,
    mut boxes: TooltipBoxes,
) {
    let now = time.elapsed_secs();
    let label = labels.hovered();
    let resting_on = rest.label.as_ref().map(|(e, text, _)| (*e, text.clone()));
    if resting_on != label {
        rest.label = label.map(|(entity, text)| (entity, text, now));
    }
    let due = rest
        .label
        .as_ref()
        .filter(|(.., since)| tooltip_due(now - since, tunables.tooltip_delay_ms));
    // With no window (or no cursor in it) the box sits at the corner.
    let at = windows
        .single()
        .ok()
        .and_then(Window::cursor_position)
        .unwrap_or_default();
    let TooltipBoxes { boxes, texts } = &mut boxes;
    for (mut node, children) in boxes.iter_mut() {
        let Some((_, tooltip, _)) = due else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px((at.x - BOX_WIDTH_PX).max(EDGE_PX));
        node.top = px(at.y + BELOW_POINTER_PX);
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child)
                && text.0 != *tooltip
            {
                text.0.clone_from(tooltip);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PLNR-35: a track's stretch shows its text over its own width
    /// only.
    #[test]
    fn a_stretch_shows_its_text_over_its_own_width() {
        let stretches = Stretches(vec![Stretch {
            from: 0.2,
            to: 0.4,
            text: "Path passes too close to Halden IV".to_string(),
        }]);
        assert_eq!(stretches.text_at(0.1), None);
        assert_eq!(
            stretches.text_at(0.3),
            Some("Path passes too close to Halden IV")
        );
        assert_eq!(stretches.text_at(0.5), None);
    }

    /// PLNR-91 / PLNR-94: a tooltip is due only once the pointer has
    /// rested for `tooltip_delay_ms`.
    #[test]
    fn a_tooltip_waits_for_the_delay() {
        let delay = Tunables::default().tooltip_delay_ms;
        assert_eq!(delay, 400.0);
        assert!(!tooltip_due(0.39, delay));
        assert!(tooltip_due(0.4, delay));
        assert!(tooltip_due(2.0, delay));
    }
}
