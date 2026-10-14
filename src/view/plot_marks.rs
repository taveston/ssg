//! The map's plot furniture (MRK-16, MRK-17): a scale bar at the bottom
//! left of the map area and a north dial at its top right. Both are read
//! off this frame's camera at the pivot, so they always agree with the
//! map, and both are placed in the map area (`MapArea`), so they move with
//! anything that narrows it.

use bevy::prelude::*;

use super::camera::requests::NorthUpRequest;
use super::camera::rules::NORTH;
use super::camera::state::CameraRig;
use super::colors;
use super::fonts::Fonts;
use super::layers;
use super::readout::format_distance;
use super::scale::{km_to_render, render_to_km};
use super::screen::MapArea;
use super::targets::MapInputBlocker;
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::sim::units::KM_PER_AU;
use crate::{Boot, Stage};

/// The scale bar's root, its line and its length label.
#[derive(Component)]
struct ScaleBar;
#[derive(Component)]
pub struct ScaleBarLine;
#[derive(Component)]
struct ScaleBarLength;

/// The north dial and the holder its arrow turns with.
#[derive(Component)]
pub struct NorthDial;
#[derive(Component)]
pub struct NorthArrow;

const LABEL_FONT_PX: f32 = 10.0;
/// The scale bar's end and middle ticks.
const TICK_PX: f32 = 5.0;
const LINE_PX: f32 = 1.0;
/// The arrow's length and its head's arms, as fractions of the dial.
const ARROW_LENGTH: f32 = 0.6;
const ARROW_HEAD: f32 = 0.18;
/// The head's arms open this far either side of the shaft.
const ARROW_HEAD_ANGLE_DEG: f32 = 35.0;

/// The largest 1, 2 or 5 times a power of ten that is at most `x`.
pub fn round_floor(x: f64) -> f64 {
    if !(x > 0.0 && x.is_finite()) {
        return 0.0;
    }
    let power = 10f64.powf(x.log10().floor());
    [5.0, 2.0, 1.0]
        .into_iter()
        .map(|step| step * power)
        .find(|value| *value <= x * (1.0 + 1e-9))
        .unwrap_or(power)
}

/// MRK-16: the scale bar's length in km: the longest round distance (1, 2
/// or 5 times a power of ten) no longer than `max_km`, round in AU at or
/// above `au_threshold_au` and in km below it (UNIT-01).
pub fn scale_bar_km(max_km: f64, au_threshold_au: f64) -> f64 {
    let au = round_floor(max_km / KM_PER_AU);
    if au >= au_threshold_au {
        return au * KM_PER_AU;
    }
    let mut km = round_floor(max_km);
    // A round km figure at or past the threshold would read in AU; step
    // down until it reads in km.
    while km / KM_PER_AU >= au_threshold_au {
        km = round_floor(km * (1.0 - 1e-6));
    }
    km
}

/// MRK-17: the clockwise angle from screen up to the direction from
/// `from` to `to` (screen y grows downwards).
pub fn screen_angle(from: Vec2, to: Vec2) -> f32 {
    let d = to - from;
    d.x.atan2(-d.y)
}

pub struct PlotMarksPlugin;

impl Plugin for PlotMarksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (spawn_scale_bar, spawn_north_dial).in_set(Boot::Overlay),
        )
        .add_systems(
            Update,
            (update_scale_bar, update_north_dial).in_set(Stage::Overlay),
        );
    }
}

/// The map area this frame, from the primary window.
fn map_area(windows: &Query<&Window>) -> Option<(Vec2, MapArea)> {
    let window = windows.single().ok()?;
    let viewport = Vec2::new(window.width(), window.height());
    Some((viewport, MapArea::of(viewport)))
}

fn tick(left: Val) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left,
            top: px(0),
            width: px(LINE_PX),
            height: px(TICK_PX),
            margin: UiRect::left(px(-LINE_PX / 2.0)),
            ..default()
        },
        BackgroundColor(colors::NEUTRAL),
    )
}

/// MRK-16: a neutral line with ticks at its ends and middle, "0" under its
/// left end and its length under its right.
fn spawn_scale_bar(mut commands: Commands, fonts: Res<Fonts>) {
    let label = |text: &str| {
        (
            Text::new(text),
            TextLayout::linebreak(LineBreak::NoWrap),
            fonts.mono(LABEL_FONT_PX),
            TextColor(colors::SECONDARY),
        )
    };
    commands
        .spawn((
            ScaleBar,
            GlobalZIndex(layers::EDGE),
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                display: Display::None,
                ..default()
            },
        ))
        .with_children(|bar| {
            bar.spawn((
                ScaleBarLine,
                Node {
                    height: px(TICK_PX),
                    ..default()
                },
            ))
            .with_children(|line| {
                line.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px((TICK_PX - LINE_PX) / 2.0),
                        width: percent(100),
                        height: px(LINE_PX),
                        ..default()
                    },
                    BackgroundColor(colors::NEUTRAL),
                ));
                for at in [0.0, 50.0, 100.0] {
                    line.spawn(tick(percent(at)));
                }
            });
            bar.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|labels| {
                labels.spawn(label("0"));
                labels.spawn((ScaleBarLength, label("")));
            });
        });
}

/// MRK-16: the bar's length on the plane at the pivot, measured along the
/// screen's horizontal, kept to the longest round distance that fits.
#[allow(clippy::type_complexity)]
fn update_scale_bar(
    view: ViewNow,
    rig: Res<CameraRig>,
    tunables: Res<Tunables>,
    windows: Query<&Window>,
    mut bars: Query<&mut Node, (With<ScaleBar>, Without<ScaleBarLine>)>,
    mut lines: Query<&mut Node, (With<ScaleBarLine>, Without<ScaleBar>)>,
    mut lengths: Query<&mut Text, With<ScaleBarLength>>,
) {
    let (Some((viewport, map)), Some((_, camera))) = (map_area(&windows), view.camera()) else {
        return;
    };
    // A short step along the camera's right, which lies in the plane.
    let step = rig.distance * 0.01;
    let ends = (
        view.project(rig.pivot),
        view.project(rig.pivot + camera.right() * step),
    );
    let (Some(a), Some(b)) = ends else {
        return;
    };
    let px_per_render = a.distance(b) / step;
    if px_per_render <= 0.0 {
        return;
    }
    let max_km = render_to_km(tunables.scale_bar_max_px / px_per_render);
    let km = scale_bar_km(max_km, tunables.distance_au_threshold_au);
    let length_px = (km_to_render(km) * px_per_render).round();

    for mut node in &mut bars {
        node.display = Display::Flex;
        node.left = px(map.offset.x + tunables.plot_margin_px);
        node.bottom = px(viewport.y - (map.offset.y + map.size.y) + tunables.plot_margin_px);
    }
    for mut node in &mut lines {
        node.width = px(length_px);
    }
    let text = format_distance(km, tunables.distance_au_threshold_au);
    for mut label in &mut lengths {
        if label.0 != text {
            label.0 = text.clone();
        }
    }
}

/// MRK-17: a flat, upright circle with a neutral arrow; clicking it does
/// what N does (CAM-03).
fn spawn_north_dial(mut commands: Commands, tunables: Res<Tunables>) {
    let size = tunables.north_dial_px;
    let shaft = size * ARROW_LENGTH;
    let head = size * ARROW_HEAD;
    let full = || Node {
        position_type: PositionType::Absolute,
        left: px(0),
        top: px(0),
        width: percent(100),
        height: percent(100),
        ..default()
    };
    commands
        .spawn((
            NorthDial,
            GlobalZIndex(layers::EDGE),
            // It wins over the map beneath it: a click on it never selects
            // or deselects, and a press on it never pans (CAM-10).
            MapInputBlocker,
            Node {
                position_type: PositionType::Absolute,
                width: px(size),
                height: px(size),
                border: UiRect::all(px(LINE_PX)),
                border_radius: BorderRadius::MAX,
                display: Display::None,
                ..default()
            },
            BackgroundColor(colors::GROUND),
            BorderColor::all(colors::SECONDARY),
        ))
        .observe(
            |_click: On<PointerClick>, mut request: ResMut<NorthUpRequest>| {
                request.0 = true;
            },
        )
        .with_children(|dial| {
            dial.spawn((NorthArrow, full(), Pickable::IGNORE))
                .with_children(|arrow| {
                    // The shaft is centred on the dial; each arm of the
                    // head hangs down from the shaft's tip.
                    arrow.spawn(segment(
                        Vec2::new(0.0, shaft / 2.0),
                        Vec2::new(0.0, -shaft / 2.0),
                    ));
                    let tip = Vec2::new(0.0, -shaft / 2.0);
                    for side in [-1.0, 1.0] {
                        let angle = (ARROW_HEAD_ANGLE_DEG * side).to_radians();
                        let arm = Vec2::new(angle.sin(), angle.cos()) * head;
                        arrow.spawn(segment(tip, tip + arm));
                    }
                });
        });
}

/// A line from `a` to `b`, in pixels from the holder's centre (y down),
/// drawn as a thin node turned about its own centre.
fn segment(a: Vec2, b: Vec2) -> impl Bundle {
    let middle = (a + b) / 2.0;
    let length = a.distance(b);
    (
        Node {
            position_type: PositionType::Absolute,
            width: px(LINE_PX),
            height: px(length),
            left: percent(50),
            top: percent(50),
            margin: UiRect {
                left: px(middle.x - LINE_PX / 2.0),
                top: px(middle.y - length / 2.0),
                ..default()
            },
            ..default()
        },
        UiTransform::from_rotation(Rot2::radians(screen_angle(a, b))),
        BackgroundColor(colors::NEUTRAL),
        Pickable::IGNORE,
    )
}

/// MRK-17: place the dial and turn its arrow to the on-screen direction of
/// north on the plane at the pivot.
#[allow(clippy::type_complexity)]
fn update_north_dial(
    view: ViewNow,
    rig: Res<CameraRig>,
    tunables: Res<Tunables>,
    windows: Query<&Window>,
    mut dials: Query<&mut Node, With<NorthDial>>,
    mut arrows: Query<&mut UiTransform, With<NorthArrow>>,
) {
    let Some((viewport, map)) = map_area(&windows) else {
        return;
    };
    for mut node in &mut dials {
        node.display = Display::Flex;
        node.top = px(map.offset.y + tunables.plot_margin_px);
        node.right = px(viewport.x - (map.offset.x + map.size.x) + tunables.plot_margin_px);
    }
    let step = rig.distance * 0.01;
    let (Some(at), Some(north)) = (
        view.project(rig.pivot),
        view.project(rig.pivot + NORTH * step),
    ) else {
        return;
    };
    let rotation = Rot2::radians(screen_angle(at, north));
    for mut transform in &mut arrows {
        if transform.rotation != rotation {
            transform.rotation = rotation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MRK-16: round distances are 1, 2 or 5 times a power of ten.
    #[test]
    fn round_floor_picks_one_two_or_five() {
        assert_eq!(round_floor(7.3), 5.0);
        assert_eq!(round_floor(19.9), 10.0);
        assert_eq!(round_floor(20.0), 20.0);
        assert_eq!(round_floor(0.034), 0.02);
        assert_eq!(round_floor(1_234_000.0), 1_000_000.0);
    }

    /// MRK-16 / UNIT-01: the bar is round in km below the AU threshold and
    /// round in AU at or above it, and never longer than allowed.
    #[test]
    fn the_scale_bar_is_round_in_its_unit() {
        let threshold = 0.1;
        assert_eq!(scale_bar_km(73_000.0, threshold), 50_000.0);
        assert_eq!(scale_bar_km(3.4 * KM_PER_AU, threshold), 2.0 * KM_PER_AU);
        assert_eq!(scale_bar_km(0.15 * KM_PER_AU, threshold), 0.1 * KM_PER_AU);
        // Just under the threshold, the round km figure must read in km.
        let km = scale_bar_km(0.099 * KM_PER_AU, threshold);
        assert!(km / KM_PER_AU < threshold);
        assert_eq!(km, 10_000_000.0);
        assert_eq!(format_distance(km, threshold), "10,000,000 km");
        assert_eq!(format_distance(2.0 * KM_PER_AU, threshold), "2 AU");
    }

    /// MRK-17: angles are clockwise from screen up.
    #[test]
    fn screen_angles_run_clockwise_from_up() {
        let o = Vec2::new(100.0, 100.0);
        let close = |a: f32, b: f32| (a - b).abs() < 1e-5;
        assert!(close(screen_angle(o, o + Vec2::NEG_Y), 0.0));
        assert!(close(
            screen_angle(o, o + Vec2::X),
            std::f32::consts::FRAC_PI_2
        ));
        assert!(close(
            screen_angle(o, o + Vec2::NEG_X),
            -std::f32::consts::FRAC_PI_2
        ));
    }
}
