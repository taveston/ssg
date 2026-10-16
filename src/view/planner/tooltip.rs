//! Label tooltips (PLNR-91, PLNR-94): a label that carries a `Tooltip`
//! shows its text in a small box by the pointer once the pointer has
//! rested on it for `tooltip_delay_ms`. These are labels' tooltips, not
//! markers': nothing on the map has one (HOV-01).

use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;

use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::layers;
use crate::view::tunables::Tunables;

/// On a label: the text its tooltip shows.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tooltip(pub &'static str);

/// The one tooltip box, shown by `show_tooltips`.
#[derive(Component)]
pub struct TooltipBox;

/// Which label the pointer rests on, and since when (real seconds).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct TooltipHover {
    pub label: Option<(Entity, f32)>,
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

/// The label with a tooltip that the mouse is over, if any: the hit
/// itself or one of its ancestors.
fn hovered_label(
    hover: Option<&HoverMap>,
    labels: &Query<&Tooltip>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let hits = hover?.0.get(&PointerId::Mouse)?;
    hits.keys().find_map(|&hit| {
        std::iter::successors(Some(hit), |&node| {
            parents.get(node).ok().map(ChildOf::parent)
        })
        .find(|&node| labels.contains(node))
    })
}

/// PLNR-91 / PLNR-94: times how long the pointer rests on a label, and
/// shows that label's tooltip by the pointer once it is due.
#[allow(clippy::too_many_arguments)]
pub fn show_tooltips(
    time: Res<Time<Real>>,
    tunables: Res<Tunables>,
    hover: Option<Res<HoverMap>>,
    labels: Query<&Tooltip>,
    parents: Query<&ChildOf>,
    windows: Query<&Window>,
    mut rest: ResMut<TooltipHover>,
    mut boxes: Query<(&mut Node, &Children), With<TooltipBox>>,
    mut texts: Query<&mut Text>,
) {
    let now = time.elapsed_secs();
    let label = hovered_label(hover.as_deref(), &labels, &parents);
    if rest.label.map(|(entity, _)| entity) != label {
        rest.label = label.map(|entity| (entity, now));
    }
    let due = rest
        .label
        .filter(|(_, since)| tooltip_due(now - since, tunables.tooltip_delay_ms));
    let cursor = windows.single().ok().and_then(Window::cursor_position);
    for (mut node, children) in &mut boxes {
        let Some(((entity, _), at)) = due.zip(cursor) else {
            node.display = Display::None;
            continue;
        };
        let Ok(tooltip) = labels.get(entity) else {
            continue;
        };
        node.display = Display::Flex;
        node.left = px((at.x - BOX_WIDTH_PX).max(EDGE_PX));
        node.top = px(at.y + BELOW_POINTER_PX);
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child)
                && text.0 != tooltip.0
            {
                text.0 = tooltip.0.to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
