//! The breadcrumb in the top bar (CLK-02, CAM-25): the path to the
//! reference frame, "Sector / *system* / ancestors… / frame body". The
//! system entry stands for the star's frame. Clicking an ancestor's entry
//! makes it the frame; clicking the frame body's own entry re-centres it.
//! Both go to the camera through `camera/requests.rs`. In the sector view
//! the breadcrumb reads "Sector" alone, as the entry for the view shown
//! (SECT-34).
//!
//! The entries are worked out by a plain function from the frame body's
//! path, so what the breadcrumb says can be tested apart from its nodes.

use bevy::prelude::*;

use crate::sim::body::DisplayName;
use crate::view::camera::requests::{FrameRequest, RecentreRequest};
use crate::view::colors;
use crate::view::fonts::{Face, Fonts};
use crate::view::frame::{Frame, ancestors};
use crate::view::map_layer::MapLayer;
use crate::view::text_size::TextSize;
use crate::view::tunables::Tunables;

/// The sector entry's text. In the system view it does nothing yet
/// (SECT-32, SECT-33).
const SECTOR: &str = "Sector";
const SEPARATOR: &str = " / ";

/// What clicking a breadcrumb entry does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrumbAction {
    /// The sector entry in the system view: nothing yet.
    None,
    /// The sector entry in the sector view, the view shown: nothing to do
    /// (SECT-34).
    Here,
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
        label: SECTOR.to_string(),
        action: CrumbAction::None,
    })
    .chain(entries)
    .collect()
}

/// SECT-34: in the sector view the breadcrumb is the sector entry alone,
/// standing for the view shown.
pub fn sector_crumbs() -> Vec<Crumb> {
    vec![Crumb {
        label: SECTOR.to_string(),
        action: CrumbAction::Here,
    }]
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
        app.init_resource::<MapLayer>()
            .add_systems(Update, rebuild_breadcrumb);
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

/// CAM-25 / SECT-34: rebuild the entries whenever the frame body or the
/// layer shown changes.
fn rebuild_breadcrumb(
    mut commands: Commands,
    (frame, layer): (Res<Frame>, Res<MapLayer>),
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
    parents: Query<&ChildOf>,
    names: Query<&DisplayName>,
    roots: Query<Entity, With<BreadcrumbRoot>>,
    mut built_for: Local<Option<(Entity, MapLayer)>>,
) {
    let Ok(root) = roots.single() else {
        return;
    };
    // The star's name is only known once the system is spawned.
    let built = *built_for == Some((frame.body, *layer));
    if built && (layer.is_sector() || names.contains(frame.star)) {
        return;
    }
    *built_for = Some((frame.body, *layer));
    let entries = if layer.is_sector() {
        sector_crumbs()
    } else {
        crumbs(&frame_path(&frame, &parents, &names))
    };
    commands
        .entity(root)
        .despawn_related::<Children>()
        .with_children(|row| {
            for (index, crumb) in entries.into_iter().enumerate() {
                if index > 0 {
                    row.spawn(crumb_text(
                        SEPARATOR,
                        (&fonts, &tunables),
                        colors::PANEL_BORDER,
                    ));
                }
                spawn_crumb(row, (&fonts, &tunables), crumb);
            }
        });
}

fn crumb_text(text: &str, (fonts, tunables): (&Fonts, &Tunables), color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextSize::Bar.font(fonts, Face::Label, tunables),
        TextColor(color),
    )
}

/// CAM-25 / SECT-34: an entry's colour: the sector's in the secondary
/// colour; the entry that stands for the current frame (the frame body's
/// own, the system's in the star's frame, or the sector's in the sector
/// view) in the frame violet, as the frame cue is; the ancestors in the
/// neutral colour.
pub fn crumb_color(action: CrumbAction) -> Color {
    match action {
        CrumbAction::None => colors::PANEL_BORDER,
        CrumbAction::Recentre | CrumbAction::Here => colors::REFERENCE,
        CrumbAction::Frame(_) => colors::NEUTRAL,
    }
}

/// One entry: the sector in the secondary colour, the rest clickable.
fn spawn_crumb(row: &mut ChildSpawnerCommands, draw: (&Fonts, &Tunables), crumb: Crumb) {
    let color = crumb_color(crumb.action);
    row.spawn((
        CrumbEntry(crumb.action),
        crumb_text(&crumb.label, draw, color),
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

    /// CAM-25: in a station's frame the path reads from the sector down
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
            "Sector / Corvane / Halden IV / Arden Highport"
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
        assert_eq!(breadcrumb_text(&entries), "Sector / Corvane");
        assert_eq!(entries[1].action, CrumbAction::Recentre);
    }

    /// SECT-34: in the sector view the breadcrumb reads "Sector" alone, in
    /// the frame colour, as the entry for the view shown.
    #[test]
    fn in_the_sector_view_the_breadcrumb_is_the_sector_alone() {
        let entries = sector_crumbs();
        assert_eq!(breadcrumb_text(&entries), "Sector");
        assert_eq!(crumb_color(entries[0].action), colors::REFERENCE);
        assert_eq!(crumb_color(CrumbAction::None), colors::PANEL_BORDER);
    }
}
