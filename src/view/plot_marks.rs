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
use super::screen::{MapArea, MapView};
use super::symbol_shape::{SymbolMask, SymbolPart};
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

/// The distance the scale bar stands for this frame, in km on the
/// orbital plane, or 0 before it is first measured (MRK-16). The frame
/// cue ticks its arms at its multiples (REF-05).
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct ScaleBarKm(pub f64);

/// `ScaleBarKm` has been measured for this frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScaleBarMeasured;

/// The north dial, the holder its head turns with, and its "N".
#[derive(Component)]
pub struct NorthDial;
#[derive(Component)]
pub struct NorthArrow;
#[derive(Component)]
pub struct NorthLabel;

const LABEL_FONT_PX: f32 = 10.0;
/// The scale bar's end and middle ticks.
const TICK_PX: f32 = 5.0;
const LINE_PX: f32 = 1.0;
/// The box the dial's "N" is centred in, and its gap outside the rim.
const N_BOX_PX: f32 = 10.0;
const N_GAP_PX: f32 = 1.0;

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
        app.init_resource::<ScaleBarKm>()
            .add_systems(
                Startup,
                (spawn_scale_bar, spawn_north_dial).in_set(Boot::Overlay),
            )
            .add_systems(
                Update,
                (update_scale_bar.in_set(ScaleBarMeasured), update_north_dial)
                    .in_set(Stage::Overlay),
            );
    }
}

/// The window and the map area in it this frame (INS-07).
fn map_area(map: &MapView) -> Option<(Vec2, MapArea)> {
    Some((map.window()?, map.area()?))
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
            fonts.data(LABEL_FONT_PX),
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
/// screen's horizontal, kept to the longest round distance that fits; the
/// distance is shared for the frame cue's ticks (REF-05).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_scale_bar(
    view: ViewNow,
    rig: Res<CameraRig>,
    tunables: Res<Tunables>,
    map_view: MapView,
    mut scale_km: ResMut<ScaleBarKm>,
    mut bars: Query<&mut Node, (With<ScaleBar>, Without<ScaleBarLine>)>,
    mut lines: Query<&mut Node, (With<ScaleBarLine>, Without<ScaleBar>)>,
    mut lengths: Query<&mut Text, With<ScaleBarLength>>,
) {
    let (Some((viewport, map)), Some((_, camera))) = (map_area(&map_view), view.camera()) else {
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
    scale_km.0 = km;

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

/// MRK-17: where the dial's "N" sits, from the dial's centre (y down):
/// just outside the circle, beyond the head's tip at `angle` clockwise
/// from screen up.
pub fn north_label_offset(angle: f32, dial_px: f32) -> Vec2 {
    let reach = dial_px / 2.0 + N_GAP_PX + N_BOX_PX / 2.0;
    Vec2::new(angle.sin(), -angle.cos()) * reach
}

/// MRK-17: a flat, upright circle with a filled head touching its rim and
/// an upright "N" beyond the tip; clicking it does what N does (CAM-03).
fn spawn_north_dial(mut commands: Commands, fonts: Res<Fonts>, tunables: Res<Tunables>) {
    let size = tunables.north_dial_px;
    let head = tunables.north_head_px;
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
            // The head hangs from the top of a full-size holder that turns
            // about the dial's centre, so its tip stays on the rim.
            dial.spawn((
                NorthArrow,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|holder| {
                holder.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(0),
                        left: percent(50),
                        margin: UiRect::left(px(-head / 2.0)),
                        width: px(head),
                        height: px(head),
                        ..default()
                    },
                    SymbolPart {
                        mask: SymbolMask::Head,
                        color: colors::NEUTRAL,
                        size_px: head,
                    },
                    Pickable::IGNORE,
                ));
            });
            dial.spawn((
                NorthLabel,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(N_BOX_PX),
                    height: px(N_BOX_PX),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|label| {
                label.spawn((
                    Text::new("N"),
                    TextLayout::linebreak(LineBreak::NoWrap),
                    fonts.data(LABEL_FONT_PX),
                    TextColor(colors::NEUTRAL),
                    Pickable::IGNORE,
                ));
            });
        });
}

/// MRK-17: place the dial and turn its arrow to the on-screen direction of
/// north on the plane at the pivot.
#[allow(clippy::type_complexity)]
fn update_north_dial(
    view: ViewNow,
    rig: Res<CameraRig>,
    tunables: Res<Tunables>,
    map_view: MapView,
    mut dials: Query<&mut Node, (With<NorthDial>, Without<NorthLabel>)>,
    mut arrows: Query<&mut UiTransform, With<NorthArrow>>,
    mut labels: Query<&mut Node, (With<NorthLabel>, Without<NorthDial>)>,
) {
    let Some((viewport, map)) = map_area(&map_view) else {
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
    let angle = screen_angle(at, north);
    let rotation = Rot2::radians(angle);
    for mut transform in &mut arrows {
        if transform.rotation != rotation {
            transform.rotation = rotation;
        }
    }
    // The "N" moves round with the tip but stays upright. Children are
    // placed inside the dial's border.
    let centre = tunables.north_dial_px / 2.0 - LINE_PX;
    let at = Vec2::splat(centre) + north_label_offset(angle, tunables.north_dial_px)
        - Vec2::splat(N_BOX_PX / 2.0);
    for mut node in &mut labels {
        node.left = px(at.x);
        node.top = px(at.y);
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

    /// MRK-17: the "N" sits beyond the tip, outside the circle, at every
    /// angle.
    #[test]
    fn the_n_sits_outside_the_rim_beyond_the_tip() {
        let dial = 34.0;
        for angle in [0.0_f32, 1.0, 2.5, -2.0] {
            let at = north_label_offset(angle, dial);
            assert!(at.length() - N_BOX_PX / 2.0 > dial / 2.0);
            assert!((screen_angle(Vec2::ZERO, at) - angle).abs() < 1e-4);
        }
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
