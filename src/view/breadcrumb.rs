//! The breadcrumb in the top bar (CLK-02, CAM-25): the path to the
//! reference frame, "Cluster / *system* / ancestors… / frame body". The
//! system entry stands for the star's frame. Clicking an ancestor's entry
//! makes it the frame; clicking the frame body's own entry re-centres it.
//! Both go to the camera through `camera/requests.rs`.
//!
//! The entries are worked out by a plain function from the frame body's
//! path, so what the breadcrumb says can be tested apart from its nodes.

use bevy::prelude::*;

use super::camera::requests::{FrameRequest, RecentreRequest};
use super::colors;
use super::fonts::Fonts;
use super::frame::{Frame, ancestors};
use crate::sim::body::DisplayName;

/// The cluster entry's text. It does nothing until the cluster view exists.
const CLUSTER: &str = "Cluster";
const SEPARATOR: &str = " / ";
const FONT_PX: f32 = 14.0;

/// What clicking a breadcrumb entry does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrumbAction {
    /// The cluster entry: nothing yet.
    None,
    /// An ancestor of the frame body (or the star): make it the frame.
    Frame(Entity),
    /// The frame body's own entry: re-centre it.
    Recentre,
}

/// One entry of the breadcrumb.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Crumb {
    pub label: String,
    pub action: CrumbAction,
}

/// CAM-25: the entries for a frame. `path` runs from the star down to the
/// frame body, each with its name; the last one is the frame body. The
/// star's entry is named for the system.
pub fn crumbs(path: &[(Entity, String)]) -> Vec<Crumb> {
    let last = path.len().saturating_sub(1);
    let entries = path.iter().enumerate().map(|(index, (body, name))| Crumb {
        label: name.clone(),
        action: if index == last {
            CrumbAction::Recentre
        } else {
            CrumbAction::Frame(*body)
        },
    });
    std::iter::once(Crumb {
        label: CLUSTER.to_string(),
        action: CrumbAction::None,
    })
    .chain(entries)
    .collect()
}

/// The node in the top bar the entries are spawned into.
#[derive(Component)]
pub struct BreadcrumbRoot;

/// One clickable entry, and what it does.
#[derive(Component)]
struct CrumbEntry(CrumbAction);

pub struct BreadcrumbPlugin;

impl Plugin for BreadcrumbPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, rebuild_breadcrumb);
    }
}

/// The top bar's slot for the breadcrumb (CLK-02).
pub fn spawn_breadcrumb_root(bar: &mut ChildSpawnerCommands) {
    bar.spawn((
        BreadcrumbRoot,
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        },
    ));
}

/// The frame body's path from the star, with names.
fn frame_path(
    frame: &Frame,
    parents: &Query<&ChildOf>,
    names: &Query<&DisplayName>,
) -> Vec<(Entity, String)> {
    let name = |body: Entity| names.get(body).map(|n| n.0.clone()).unwrap_or_default();
    let mut path = vec![frame.star];
    if !frame.in_star_frame() {
        let chain = ancestors(frame.body, |e| parents.get(e).ok().map(ChildOf::parent));
        path.extend(chain.into_iter().rev());
        path.push(frame.body);
    }
    path.into_iter().map(|body| (body, name(body))).collect()
}

/// CAM-25: rebuild the entries whenever the frame body changes.
fn rebuild_breadcrumb(
    mut commands: Commands,
    frame: Res<Frame>,
    fonts: Res<Fonts>,
    parents: Query<&ChildOf>,
    names: Query<&DisplayName>,
    roots: Query<Entity, With<BreadcrumbRoot>>,
    mut built_for: Local<Option<Entity>>,
) {
    let Ok(root) = roots.single() else {
        return;
    };
    // The star's name is only known once the system is spawned.
    if *built_for == Some(frame.body) && names.contains(frame.star) {
        return;
    }
    *built_for = Some(frame.body);
    let entries = crumbs(&frame_path(&frame, &parents, &names));
    commands
        .entity(root)
        .despawn_related::<Children>()
        .with_children(|row| {
            for (index, crumb) in entries.into_iter().enumerate() {
                if index > 0 {
                    row.spawn(crumb_text(SEPARATOR, &fonts, colors::PANEL_BORDER));
                }
                spawn_crumb(row, &fonts, crumb);
            }
        });
}

fn crumb_text(text: &str, fonts: &Fonts, color: Color) -> impl Bundle {
    (Text::new(text), fonts.label(FONT_PX), TextColor(color))
}

/// CAM-25: an entry's colour: the cluster's in the secondary colour; the
/// entry that stands for the current frame (the frame body's own, or the
/// system's in the star's frame) in the frame violet, as the frame cue is;
/// the ancestors in the neutral colour.
pub fn crumb_color(action: CrumbAction) -> Color {
    match action {
        CrumbAction::None => colors::PANEL_BORDER,
        CrumbAction::Recentre => colors::REFERENCE,
        CrumbAction::Frame(_) => colors::NEUTRAL,
    }
}

/// One entry: the cluster in the secondary colour, the rest clickable.
fn spawn_crumb(row: &mut ChildSpawnerCommands, fonts: &Fonts, crumb: Crumb) {
    let color = crumb_color(crumb.action);
    row.spawn((
        CrumbEntry(crumb.action),
        crumb_text(&crumb.label, fonts, color),
    ))
    .observe(on_crumb_click);
}

/// CAM-25: an ancestor's entry asks for its frame; the frame body's own
/// entry asks for a re-centre.
fn on_crumb_click(
    click: On<PointerClick>,
    entries: Query<&CrumbEntry>,
    mut frame_request: ResMut<FrameRequest>,
    mut recentre: ResMut<RecentreRequest>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    match entries.get(click.entity).map(|entry| entry.0) {
        Ok(CrumbAction::Frame(body)) => frame_request.0 = Some(body),
        Ok(CrumbAction::Recentre) => recentre.0 = true,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The breadcrumb's text, entries joined by " / ".
    fn breadcrumb_text(crumbs: &[Crumb]) -> String {
        crumbs
            .iter()
            .map(|crumb| crumb.label.as_str())
            .collect::<Vec<_>>()
            .join(SEPARATOR)
    }

    /// CAM-25: in a station's frame the path reads from the cluster down
    /// to the station; ancestors reframe and the station re-centres.
    #[test]
    fn the_breadcrumb_reads_the_path_to_the_frame_body() {
        let mut world = World::new();
        let [star, planet, station] = [(); 3].map(|_| world.spawn_empty().id());
        let path = [
            (star, "Corvane".to_string()),
            (planet, "Halden IV".to_string()),
            (station, "Arden Highport".to_string()),
        ];
        let entries = crumbs(&path);
        assert_eq!(
            breadcrumb_text(&entries),
            "Cluster / Corvane / Halden IV / Arden Highport"
        );
        let actions: Vec<CrumbAction> = entries.iter().map(|c| c.action).collect();
        assert_eq!(
            actions,
            [
                CrumbAction::None,
                CrumbAction::Frame(star),
                CrumbAction::Frame(planet),
                CrumbAction::Recentre
            ]
        );
    }

    /// CAM-25: in the star's frame the system entry is the frame body's own
    /// entry, so it re-centres.
    #[test]
    fn in_the_star_frame_the_system_entry_is_the_frame_body() {
        let mut world = World::new();
        let star = world.spawn_empty().id();
        let entries = crumbs(&[(star, "Corvane".to_string())]);
        assert_eq!(breadcrumb_text(&entries), "Cluster / Corvane");
        assert_eq!(entries[1].action, CrumbAction::Recentre);
    }
}
