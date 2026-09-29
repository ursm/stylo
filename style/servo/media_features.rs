/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! Servo's media feature list and evaluator.

use crate::derives::*;
use crate::queries::feature::{AllowsRanges, Evaluator, FeatureFlags, QueryFeatureDescription};
use crate::media_queries::MediaType;
use crate::queries::values::{Orientation, PrefersColorScheme};
use crate::values::specified::color::ForcedColors;
use crate::values::computed::{CSSPixelLength, Context, Ratio, Resolution};
use std::fmt::Debug;

/// https://drafts.csswg.org/mediaqueries-4/#width
fn eval_width(context: &Context) -> CSSPixelLength {
    CSSPixelLength::new(context.device().au_viewport_size().width.to_f32_px())
}

/// https://drafts.csswg.org/mediaqueries-4/#height
fn eval_height(context: &Context) -> CSSPixelLength {
    CSSPixelLength::new(context.device().au_viewport_size().height.to_f32_px())
}

/// https://drafts.csswg.org/mediaqueries-4/#device-width
fn eval_device_width(context: &Context) -> CSSPixelLength {
    let device = context.device();
    let scaled = device.device_size() / device.device_pixel_ratio();
    CSSPixelLength::new(scaled.width)
}

/// https://drafts.csswg.org/mediaqueries-4/#device-height
fn eval_device_height(context: &Context) -> CSSPixelLength {
    let device = context.device();
    let scaled = device.device_size() / device.device_pixel_ratio();
    CSSPixelLength::new(scaled.height)
}

/// https://drafts.csswg.org/mediaqueries-4/#orientation
fn eval_orientation(context: &Context, value: Option<Orientation>) -> bool {
    Orientation::eval(context.device().au_viewport_size(), value)
}

#[derive(Clone, Copy, Debug, FromPrimitive, Parse, ToCss)]
#[repr(u8)]
enum Scan {
    Progressive,
    Interlace,
}

/// https://drafts.csswg.org/mediaqueries-4/#scan
fn eval_scan(_: &Context, _: Option<Scan>) -> bool {
    // Since we doesn't support the 'tv' media type, the 'scan' feature never
    // matches.
    false
}

/// https://drafts.csswg.org/mediaqueries-4/#resolution
fn eval_resolution(context: &Context) -> Resolution {
    Resolution::from_dppx(context.device().device_pixel_ratio().0)
}

/// https://compat.spec.whatwg.org/#css-media-queries-webkit-device-pixel-ratio
fn eval_device_pixel_ratio(context: &Context) -> f32 {
    eval_resolution(context).dppx()
}

fn eval_prefers_color_scheme(context: &Context, query_value: Option<PrefersColorScheme>) -> bool {
    match query_value {
        Some(v) => context.device().color_scheme() == v,
        None => true,
    }
}

bitflags! {
    /// https://drafts.csswg.org/mediaqueries-4/#mf-interaction
    #[derive(Debug, Clone, Copy)]
    pub struct PointerCapabilities: u8 {
        /// The input mechanism includes a pointing device of limited accuracy, such as a finger on a touchscreen.
        const COARSE = 0b001;
        /// The input mechanism includes an accurate pointing device, such as a mouse.
        const FINE = 0b010;
        /// The input mechanism can conveniently hover over elements.
        const HOVER = 0b100;
    }
}

impl Default for PointerCapabilities {
    #[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos"))]
    fn default() -> Self {
        PointerCapabilities::COARSE
    }
    #[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
    fn default() -> Self {
        PointerCapabilities::FINE | PointerCapabilities::HOVER
    }
}

#[derive(Clone, Copy, Debug, FromPrimitive, Parse, ToCss)]
#[repr(u8)]
enum Pointer {
    None,
    Coarse,
    Fine,
}

fn eval_pointer_capabilities(
    query_value: Option<Pointer>,
    pointer_capabilities: PointerCapabilities,
) -> bool {
    match query_value {
        None => !pointer_capabilities.is_empty(),
        Some(Pointer::None) => pointer_capabilities.is_empty(),
        Some(Pointer::Coarse) => pointer_capabilities.intersects(PointerCapabilities::COARSE),
        Some(Pointer::Fine) => pointer_capabilities.intersects(PointerCapabilities::FINE),
    }
}

/// https://drafts.csswg.org/mediaqueries-4/#pointer
fn eval_pointer(context: &Context, query_value: Option<Pointer>) -> bool {
    eval_pointer_capabilities(query_value, context.device().primary_pointer_capabilities())
}

/// https://drafts.csswg.org/mediaqueries-4/#descdef-media-any-pointer
fn eval_any_pointer(context: &Context, query_value: Option<Pointer>) -> bool {
    eval_pointer_capabilities(query_value, context.device().all_pointer_capabilities())
}

#[derive(Clone, Copy, Debug, FromPrimitive, Parse, ToCss)]
#[repr(u8)]
enum Hover {
    None,
    Hover,
}

fn eval_hover_capabilities(
    query_value: Option<Hover>,
    pointer_capabilities: PointerCapabilities,
) -> bool {
    let can_hover = pointer_capabilities.intersects(PointerCapabilities::HOVER);
    match query_value {
        Some(Hover::None) => !can_hover,
        Some(Hover::Hover) => can_hover,
        None => return can_hover,
    }
}

/// https://drafts.csswg.org/mediaqueries-4/#hover
fn eval_hover(context: &Context, query_value: Option<Hover>) -> bool {
    eval_hover_capabilities(query_value, context.device().primary_pointer_capabilities())
}

/// https://drafts.csswg.org/mediaqueries-4/#descdef-media-any-hover
fn eval_any_hover(context: &Context, query_value: Option<Hover>) -> bool {
    eval_hover_capabilities(query_value, context.device().all_pointer_capabilities())
}

/// <https://drafts.csswg.org/mediaqueries-4/#aspect-ratio>
fn eval_aspect_ratio(context: &Context) -> Ratio {
    let size = context.device().au_viewport_size();
    Ratio::new(size.width.0 as f32, size.height.0 as f32)
}

// The media features a desktop browser answers about the display and the user's preferences, as one answers them
// with no preference set: an sRGB colour screen that scrolls and updates, scripting on, nothing reduced, inverted,
// forced or raised in contrast, and a browser tab rather than an installed app. On `print` media, overflow pages
// and nothing updates.

/// https://drafts.csswg.org/mediaqueries-4/#color
fn eval_color(_: &Context) -> i32 {
    8
}

/// https://drafts.csswg.org/mediaqueries-4/#color-index
fn eval_color_index(_: &Context) -> i32 {
    0
}

/// https://drafts.csswg.org/mediaqueries-4/#monochrome
fn eval_monochrome(_: &Context) -> i32 {
    0
}

/// https://drafts.csswg.org/mediaqueries-4/#grid
fn eval_grid(_: &Context) -> bool {
    false
}

/// https://drafts.csswg.org/mediaqueries-4/#color-gamut
#[derive(Clone, Copy, Debug, FromPrimitive, Parse, PartialEq, PartialOrd, ToCss)]
#[repr(u8)]
enum ColorGamut {
    Srgb,
    P3,
    Rec2020,
}

fn eval_color_gamut(_: &Context, query_value: Option<ColorGamut>) -> bool {
    query_value.is_some_and(|v| v <= ColorGamut::Srgb)
}

/// https://drafts.csswg.org/mediaqueries-5/#dynamic-range
#[derive(Clone, Copy, Debug, FromPrimitive, Parse, PartialEq, PartialOrd, ToCss)]
#[repr(u8)]
enum DynamicRange {
    Standard,
    High,
}

fn eval_dynamic_range(_: &Context, query_value: Option<DynamicRange>) -> bool {
    query_value.is_some_and(|v| v <= DynamicRange::Standard)
}

/// https://w3c.github.io/manifest/#the-display-mode-media-feature
#[derive(Clone, Copy, Debug, FromPrimitive, Parse, PartialEq, ToCss)]
#[repr(u8)]
enum DisplayMode {
    Browser,
    MinimalUi,
    Standalone,
    Fullscreen,
}

fn eval_display_mode(_: &Context, query_value: Option<DisplayMode>) -> bool {
    query_value.is_none_or(|v| v == DisplayMode::Browser)
}

/// https://drafts.csswg.org/mediaqueries-5/#prefers-reduced-motion
#[derive(Clone, Copy, Debug, FromPrimitive, Parse, PartialEq, ToCss)]
#[repr(u8)]
enum PrefersReduced {
    NoPreference,
    Reduce,
}

fn eval_prefers_reduced(_: &Context, query_value: Option<PrefersReduced>) -> bool {
    query_value == Some(PrefersReduced::NoPreference)
}

/// https://drafts.csswg.org/mediaqueries-5/#prefers-contrast
#[derive(Clone, Copy, Debug, FromPrimitive, Parse, PartialEq, ToCss)]
#[repr(u8)]
enum PrefersContrast {
    More,
    Less,
    Custom,
    NoPreference,
}

fn eval_prefers_contrast(_: &Context, query_value: Option<PrefersContrast>) -> bool {
    query_value == Some(PrefersContrast::NoPreference)
}

/// https://drafts.csswg.org/mediaqueries-5/#forced-colors
fn eval_forced_colors(_: &Context, query_value: Option<ForcedColors>) -> bool {
    query_value == Some(ForcedColors::None)
}

/// https://drafts.csswg.org/mediaqueries-5/#inverted-colors
#[derive(Clone, Copy, Debug, FromPrimitive, Parse, PartialEq, ToCss)]
#[repr(u8)]
enum InvertedColors {
    None,
    Inverted,
}

fn eval_inverted_colors(_: &Context, query_value: Option<InvertedColors>) -> bool {
    query_value == Some(InvertedColors::None)
}

/// https://drafts.csswg.org/mediaqueries-5/#scripting
#[derive(Clone, Copy, Debug, FromPrimitive, Parse, PartialEq, ToCss)]
#[repr(u8)]
enum Scripting {
    None,
    InitialOnly,
    Enabled,
}

fn eval_scripting(_: &Context, query_value: Option<Scripting>) -> bool {
    query_value.is_none_or(|v| v == Scripting::Enabled)
}

fn is_print(context: &Context) -> bool {
    context.device().media_type() == MediaType::print()
}

/// https://drafts.csswg.org/mediaqueries-4/#overflow-block
#[derive(Clone, Copy, Debug, FromPrimitive, Parse, PartialEq, ToCss)]
#[repr(u8)]
enum OverflowBlock {
    None,
    Scroll,
    Paged,
}

fn eval_overflow_block(context: &Context, query_value: Option<OverflowBlock>) -> bool {
    match query_value {
        None => true,
        Some(OverflowBlock::None) => false,
        Some(OverflowBlock::Scroll) => !is_print(context),
        Some(OverflowBlock::Paged) => is_print(context),
    }
}

/// https://drafts.csswg.org/mediaqueries-4/#overflow-inline
#[derive(Clone, Copy, Debug, FromPrimitive, Parse, PartialEq, ToCss)]
#[repr(u8)]
enum OverflowInline {
    None,
    Scroll,
}

fn eval_overflow_inline(context: &Context, query_value: Option<OverflowInline>) -> bool {
    match query_value {
        None | Some(OverflowInline::Scroll) => !is_print(context),
        Some(OverflowInline::None) => is_print(context),
    }
}

/// https://drafts.csswg.org/mediaqueries-4/#update
#[derive(Clone, Copy, Debug, FromPrimitive, Parse, PartialEq, ToCss)]
#[repr(u8)]
enum Update {
    None,
    Slow,
    Fast,
}

fn eval_update(context: &Context, query_value: Option<Update>) -> bool {
    match query_value {
        None | Some(Update::Fast) => !is_print(context),
        Some(Update::Slow) => false,
        Some(Update::None) => is_print(context),
    }
}

/// A list with all the media features that Servo supports.
pub static MEDIA_FEATURES: [QueryFeatureDescription; 31] = [
    feature!(
        atom!("width"),
        AllowsRanges::Yes,
        Evaluator::Length(eval_width),
        FeatureFlags::VIEWPORT_DEPENDENT,
    ),
    feature!(
        atom!("height"),
        AllowsRanges::Yes,
        Evaluator::Length(eval_height),
        FeatureFlags::VIEWPORT_DEPENDENT,
    ),
    feature!(
        atom!("orientation"),
        AllowsRanges::No,
        keyword_evaluator!(eval_orientation, Orientation),
        FeatureFlags::VIEWPORT_DEPENDENT,
    ),
    feature!(
        atom!("pointer"),
        AllowsRanges::No,
        keyword_evaluator!(eval_pointer, Pointer),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("any-pointer"),
        AllowsRanges::No,
        keyword_evaluator!(eval_any_pointer, Pointer),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("hover"),
        AllowsRanges::No,
        keyword_evaluator!(eval_hover, Hover),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("any-hover"),
        AllowsRanges::No,
        keyword_evaluator!(eval_any_hover, Hover),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("aspect-ratio"),
        AllowsRanges::Yes,
        Evaluator::NumberRatio(eval_aspect_ratio),
        FeatureFlags::VIEWPORT_DEPENDENT,
    ),
    feature!(
        atom!("device-width"),
        AllowsRanges::Yes,
        Evaluator::Length(eval_device_width),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("device-height"),
        AllowsRanges::Yes,
        Evaluator::Length(eval_device_height),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("scan"),
        AllowsRanges::No,
        keyword_evaluator!(eval_scan, Scan),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("resolution"),
        AllowsRanges::Yes,
        Evaluator::Resolution(eval_resolution),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("device-pixel-ratio"),
        AllowsRanges::Yes,
        Evaluator::Float(eval_device_pixel_ratio),
        FeatureFlags::WEBKIT_PREFIX,
    ),
    feature!(
        atom!("-moz-device-pixel-ratio"),
        AllowsRanges::Yes,
        Evaluator::Float(eval_device_pixel_ratio),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("prefers-color-scheme"),
        AllowsRanges::No,
        keyword_evaluator!(eval_prefers_color_scheme, PrefersColorScheme),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("color"),
        AllowsRanges::Yes,
        Evaluator::Integer(eval_color),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("color-index"),
        AllowsRanges::Yes,
        Evaluator::Integer(eval_color_index),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("monochrome"),
        AllowsRanges::Yes,
        Evaluator::Integer(eval_monochrome),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("grid"),
        AllowsRanges::No,
        Evaluator::BoolInteger(eval_grid),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("color-gamut"),
        AllowsRanges::No,
        keyword_evaluator!(eval_color_gamut, ColorGamut),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("dynamic-range"),
        AllowsRanges::No,
        keyword_evaluator!(eval_dynamic_range, DynamicRange),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("display-mode"),
        AllowsRanges::No,
        keyword_evaluator!(eval_display_mode, DisplayMode),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("prefers-reduced-motion"),
        AllowsRanges::No,
        keyword_evaluator!(eval_prefers_reduced, PrefersReduced),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("prefers-reduced-transparency"),
        AllowsRanges::No,
        keyword_evaluator!(eval_prefers_reduced, PrefersReduced),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("prefers-contrast"),
        AllowsRanges::No,
        keyword_evaluator!(eval_prefers_contrast, PrefersContrast),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("forced-colors"),
        AllowsRanges::No,
        keyword_evaluator!(eval_forced_colors, ForcedColors),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("inverted-colors"),
        AllowsRanges::No,
        keyword_evaluator!(eval_inverted_colors, InvertedColors),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("scripting"),
        AllowsRanges::No,
        keyword_evaluator!(eval_scripting, Scripting),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("overflow-block"),
        AllowsRanges::No,
        keyword_evaluator!(eval_overflow_block, OverflowBlock),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("overflow-inline"),
        AllowsRanges::No,
        keyword_evaluator!(eval_overflow_inline, OverflowInline),
        FeatureFlags::empty(),
    ),
    feature!(
        atom!("update"),
        AllowsRanges::No,
        keyword_evaluator!(eval_update, Update),
        FeatureFlags::empty(),
    ),
];
