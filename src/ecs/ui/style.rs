use bevy_ecs::prelude::*;
use taffy::geometry as geom;
use taffy::prelude::TaffyZero;
use taffy::style as tstyle;
use taffy::style::Style as TStyle;

#[derive(Component, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct UIStyle {
    pub display: Display,
    pub mode: Mode,

    pub flex_direction: FlexDirection,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Unit,
    pub flex_wrap: FlexWrap,

    pub top: Unit,
    pub bottom: Unit,
    pub left: Unit,
    pub right: Unit,

    pub width: Unit,
    pub height: Unit,

    pub min_width: Unit,
    pub min_height: Unit,

    pub max_width: Unit,
    pub max_height: Unit,

    pub gap_horizontal: Unit,
    pub gap_vertical: Unit,

    pub margin_top: Unit,
    pub margin_bottom: Unit,
    pub margin_left: Unit,
    pub margin_right: Unit,

    pub padding_top: Unit,
    pub padding_bottom: Unit,
    pub padding_left: Unit,
    pub padding_right: Unit,

    pub align_items: Option<AlignItems>,
    pub align_self: Option<AlignSelf>,
    pub justify_items: Option<JustifyItems>,
    pub justify_self: Option<JustifySelf>,
    pub align_content: Option<AlignContent>,
    pub justify_content: Option<JustifyContent>,
}

impl UIStyle {
    #[inline]
    pub fn flex(mut self) -> Self {
        self.display = Display::Flex;
        self
    }

    #[inline]
    pub fn grid(mut self) -> Self {
        self.display = Display::Grid;
        self
    }

    #[inline]
    pub fn hide(mut self) -> Self {
        self.display = Display::None;
        self
    }

    pub fn relative(mut self) -> Self {
        self.mode = Mode::Relative;
        self
    }

    pub fn absolute(mut self) -> Self {
        self.mode = Mode::Absolute;
        self
    }

    #[inline]
    pub fn flex_col(mut self) -> Self {
        self.flex_direction = FlexDirection::Col;
        self
    }

    #[inline]
    pub fn flex_row(mut self) -> Self {
        self.flex_direction = FlexDirection::Row;
        self
    }

    #[inline]
    pub fn flex_wrap(mut self) -> Self {
        self.flex_wrap = FlexWrap::Wrap;
        self
    }

    #[inline]
    pub fn flex_grow(mut self, value: f32) -> Self {
        self.flex_grow = value;
        self
    }

    #[inline]
    pub fn flex_shrink(mut self, value: f32) -> Self {
        self.flex_shrink = value;
        self
    }

    #[inline]
    pub fn flex_basis(mut self, unit: impl Into<Unit>) -> Self {
        self.flex_basis = unit.into();
        self
    }

    #[inline]
    pub fn top(mut self, unit: impl Into<Unit>) -> Self {
        self.top = unit.into();
        self
    }

    #[inline]
    pub fn bottom(mut self, unit: impl Into<Unit>) -> Self {
        self.bottom = unit.into();
        self
    }

    #[inline]
    pub fn left(mut self, unit: impl Into<Unit>) -> Self {
        self.left = unit.into();
        self
    }

    #[inline]
    pub fn right(mut self, unit: impl Into<Unit>) -> Self {
        self.right = unit.into();
        self
    }

    #[inline]
    pub fn size(mut self, x: impl Into<Unit>, y: impl Into<Unit>) -> Self {
        self.width = x.into();
        self.height = y.into();
        self
    }

    #[inline]
    pub fn size_auto(self) -> Self {
        self.size(auto(), auto())
    }

    #[inline]
    pub fn size_full(self) -> Self {
        self.size(Unit::Relative(1.0), Unit::Relative(1.0))
    }

    #[inline]
    pub fn width(mut self, unit: impl Into<Unit>) -> Self {
        self.width = unit.into();
        self
    }

    #[inline]
    pub fn width_auto(mut self) -> Self {
        self.width = Unit::Auto;
        self
    }

    #[inline]
    pub fn height(mut self, unit: impl Into<Unit>) -> Self {
        self.height = unit.into();
        self
    }

    #[inline]
    pub fn height_auto(mut self) -> Self {
        self.height = Unit::Auto;
        self
    }

    #[inline]
    pub fn min_size(mut self, x: impl Into<Unit>, y: impl Into<Unit>) -> Self {
        self.min_width = x.into();
        self.min_height = y.into();
        self
    }

    #[inline]
    pub fn min_width(mut self, unit: impl Into<Unit>) -> Self {
        self.min_width = unit.into();
        self
    }

    #[inline]
    pub fn min_width_auto(mut self) -> Self {
        self.min_width = Unit::Auto;
        self
    }

    #[inline]
    pub fn min_height(mut self, unit: impl Into<Unit>) -> Self {
        self.min_height = unit.into();
        self
    }

    #[inline]
    pub fn min_height_auto(mut self) -> Self {
        self.min_height = Unit::Auto;
        self
    }

    #[inline]
    pub fn max_size(mut self, x: impl Into<Unit>, y: impl Into<Unit>) -> Self {
        self.max_width = x.into();
        self.max_height = y.into();
        self
    }

    #[inline]
    pub fn max_width(mut self, unit: impl Into<Unit>) -> Self {
        self.max_width = unit.into();
        self
    }

    #[inline]
    pub fn max_width_auto(mut self) -> Self {
        self.max_width = Unit::Auto;
        self
    }

    #[inline]
    pub fn max_height(mut self, unit: impl Into<Unit>) -> Self {
        self.max_height = unit.into();
        self
    }

    #[inline]
    pub fn max_height_auto(mut self) -> Self {
        self.max_height = Unit::Auto;
        self
    }

    #[inline]
    pub fn padding(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.padding_top = unit;
        self.padding_bottom = unit;
        self.padding_left = unit;
        self.padding_right = unit;
        self
    }

    #[inline]
    pub fn padding_y(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.padding_top = unit;
        self.padding_bottom = unit;
        self
    }

    #[inline]
    pub fn padding_x(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.padding_left = unit;
        self.padding_right = unit;
        self
    }

    #[inline]
    pub fn padding_top(mut self, unit: impl Into<Unit>) -> Self {
        self.padding_top = unit.into();
        self
    }

    #[inline]
    pub fn padding_bottom(mut self, unit: impl Into<Unit>) -> Self {
        self.padding_bottom = unit.into();
        self
    }

    #[inline]
    pub fn padding_left(mut self, unit: impl Into<Unit>) -> Self {
        self.padding_left = unit.into();
        self
    }

    #[inline]
    pub fn padding_right(mut self, unit: impl Into<Unit>) -> Self {
        self.padding_right = unit.into();
        self
    }

    #[inline]
    pub fn margin(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.margin_top = unit;
        self.margin_bottom = unit;
        self.margin_left = unit;
        self.margin_right = unit;
        self
    }

    #[inline]
    pub fn margin_y(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.margin_top = unit;
        self.margin_bottom = unit;
        self
    }

    #[inline]
    pub fn margin_x(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.margin_left = unit;
        self.margin_right = unit;
        self
    }

    #[inline]
    pub fn margin_top(mut self, unit: impl Into<Unit>) -> Self {
        self.margin_top = unit.into();
        self
    }

    #[inline]
    pub fn margin_bottom(mut self, unit: impl Into<Unit>) -> Self {
        self.margin_bottom = unit.into();
        self
    }

    #[inline]
    pub fn margin_left(mut self, unit: impl Into<Unit>) -> Self {
        self.margin_left = unit.into();
        self
    }

    #[inline]
    pub fn margin_right(mut self, unit: impl Into<Unit>) -> Self {
        self.margin_right = unit.into();
        self
    }

    #[inline]
    pub fn gap(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.gap_vertical = unit;
        self.gap_horizontal = unit;
        self
    }

    #[inline]
    pub fn gap_x(mut self, unit: impl Into<Unit>) -> Self {
        self.gap_horizontal = unit.into();
        self
    }

    #[inline]
    pub fn gap_y(mut self, unit: impl Into<Unit>) -> Self {
        self.gap_vertical = unit.into();
        self
    }

    #[inline]
    pub fn align_items_start(mut self) -> Self {
        self.align_items = Some(AlignItems::Start);
        self
    }

    #[inline]
    pub fn align_items_end(mut self) -> Self {
        self.align_items = Some(AlignItems::End);
        self
    }

    #[inline]
    pub fn align_items_flex_start(mut self) -> Self {
        self.align_items = Some(AlignItems::FlexStart);
        self
    }

    #[inline]
    pub fn align_items_flex_end(mut self) -> Self {
        self.align_items = Some(AlignItems::FlexEnd);
        self
    }

    #[inline]
    pub fn align_items_center(mut self) -> Self {
        self.align_items = Some(AlignItems::Center);
        self
    }

    #[inline]
    pub fn align_items_baseline(mut self) -> Self {
        self.align_items = Some(AlignItems::Baseline);
        self
    }

    #[inline]
    pub fn align_items_stretch(mut self) -> Self {
        self.align_items = Some(AlignItems::Stretch);
        self
    }

    #[inline]
    pub fn align_self_start(mut self) -> Self {
        self.align_self = Some(AlignSelf::Start);
        self
    }

    #[inline]
    pub fn align_self_end(mut self) -> Self {
        self.align_self = Some(AlignSelf::End);
        self
    }

    #[inline]
    pub fn align_self_flex_start(mut self) -> Self {
        self.align_self = Some(AlignSelf::FlexStart);
        self
    }

    #[inline]
    pub fn align_self_flex_end(mut self) -> Self {
        self.align_self = Some(AlignSelf::FlexEnd);
        self
    }

    #[inline]
    pub fn align_self_center(mut self) -> Self {
        self.align_self = Some(AlignSelf::Center);
        self
    }

    #[inline]
    pub fn align_self_baseline(mut self) -> Self {
        self.align_self = Some(AlignSelf::Baseline);
        self
    }

    #[inline]
    pub fn align_self_stretch(mut self) -> Self {
        self.align_self = Some(AlignSelf::Stretch);
        self
    }

    #[inline]
    pub fn justify_items_start(mut self) -> Self {
        self.justify_items = Some(JustifyItems::Start);
        self
    }

    #[inline]
    pub fn justify_items_end(mut self) -> Self {
        self.justify_items = Some(JustifyItems::End);
        self
    }

    #[inline]
    pub fn justify_items_center(mut self) -> Self {
        self.justify_items = Some(JustifyItems::Center);
        self
    }

    #[inline]
    pub fn justify_items_baseline(mut self) -> Self {
        self.justify_items = Some(JustifyItems::Baseline);
        self
    }

    #[inline]
    pub fn justify_items_stretch(mut self) -> Self {
        self.justify_items = Some(JustifyItems::Stretch);
        self
    }

    #[inline]
    pub fn justify_self_start(mut self) -> Self {
        self.justify_self = Some(JustifySelf::Start);
        self
    }

    #[inline]
    pub fn justify_self_end(mut self) -> Self {
        self.justify_self = Some(JustifySelf::End);
        self
    }

    #[inline]
    pub fn justify_self_center(mut self) -> Self {
        self.justify_self = Some(JustifySelf::Center);
        self
    }

    #[inline]
    pub fn justify_self_baseline(mut self) -> Self {
        self.justify_self = Some(JustifySelf::Baseline);
        self
    }

    #[inline]
    pub fn justify_self_stretch(mut self) -> Self {
        self.justify_self = Some(JustifySelf::Stretch);
        self
    }

    #[inline]
    pub fn align_content_start(mut self) -> Self {
        self.align_content = Some(AlignContent::Start);
        self
    }

    #[inline]
    pub fn align_content_end(mut self) -> Self {
        self.align_content = Some(AlignContent::End);
        self
    }

    #[inline]
    pub fn align_content_center(mut self) -> Self {
        self.align_content = Some(AlignContent::Center);
        self
    }

    #[inline]
    pub fn align_content_stretch(mut self) -> Self {
        self.align_content = Some(AlignContent::Stretch);
        self
    }

    #[inline]
    pub fn align_content_space_between(mut self) -> Self {
        self.align_content = Some(AlignContent::SpaceBetween);
        self
    }

    #[inline]
    pub fn align_content_space_evenly(mut self) -> Self {
        self.align_content = Some(AlignContent::SpaceEvenly);
        self
    }

    #[inline]
    pub fn align_content_space_around(mut self) -> Self {
        self.align_content = Some(AlignContent::SpaceAround);
        self
    }

    #[inline]
    pub fn justify_content_start(mut self) -> Self {
        self.justify_content = Some(JustifyContent::Start);
        self
    }

    #[inline]
    pub fn justify_content_end(mut self) -> Self {
        self.justify_content = Some(JustifyContent::End);
        self
    }

    #[inline]
    pub fn justify_content_center(mut self) -> Self {
        self.justify_content = Some(JustifyContent::Center);
        self
    }

    #[inline]
    pub fn justify_content_stretch(mut self) -> Self {
        self.justify_content = Some(JustifyContent::Stretch);
        self
    }

    #[inline]
    pub fn justify_content_space_between(mut self) -> Self {
        self.justify_content = Some(JustifyContent::SpaceBetween);
        self
    }

    #[inline]
    pub fn justify_content_space_evenly(mut self) -> Self {
        self.justify_content = Some(JustifyContent::SpaceEvenly);
        self
    }

    #[inline]
    pub fn justify_content_space_around(mut self) -> Self {
        self.justify_content = Some(JustifyContent::SpaceAround);
        self
    }

    pub(super) fn to_taffy(&self) -> TStyle {
        taffy_style_from(&self)
    }
}

impl Default for UIStyle {
    fn default() -> Self {
        Self {
            display: Display::Flex,
            mode: Mode::Relative,
            flex_direction: FlexDirection::Row,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Unit::Auto,
            flex_wrap: FlexWrap::NoWrap,
            top: Unit::ZERO,
            bottom: Unit::ZERO,
            left: Unit::ZERO,
            right: Unit::ZERO,
            width: Unit::Auto,
            height: Unit::Auto,
            min_width: Unit::Auto,
            min_height: Unit::Auto,
            max_width: Unit::Auto,
            max_height: Unit::Auto,
            gap_horizontal: Unit::ZERO,
            gap_vertical: Unit::ZERO,
            margin_top: Unit::ZERO,
            margin_bottom: Unit::ZERO,
            margin_left: Unit::ZERO,
            margin_right: Unit::ZERO,
            padding_top: Unit::ZERO,
            padding_bottom: Unit::ZERO,
            padding_left: Unit::ZERO,
            padding_right: Unit::ZERO,

            align_content: None,
            justify_content: None,
            align_items: None,
            align_self: None,
            justify_items: None,
            justify_self: None,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    Reverse,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Mode {
    #[default]
    Relative,
    Absolute,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Display {
    #[default]
    Flex,
    Grid,
    None,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Unit {
    #[default]
    Auto,
    Pixel(f32),
    Relative(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum AlignItems {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

pub type JustifyItems = AlignItems;
pub type AlignSelf = AlignItems;
pub type JustifySelf = AlignItems;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum AlignContent {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

pub type JustifyContent = AlignContent;

#[inline]
pub fn px(unit: f32) -> Unit {
    Unit::Pixel(unit)
}

#[inline]
pub fn rel(unit: f32) -> Unit {
    Unit::Relative(unit)
}

#[inline]
pub fn auto() -> Unit {
    Unit::Auto
}

impl Unit {
    pub const ZERO: Self = Self::Pixel(0.0);
    pub const AUTO: Self = Self::Auto;
    pub const FULL: Self = Self::Relative(1.0);
    pub const HALF: Self = Self::Relative(0.5);
}

impl From<f32> for Unit {
    #[inline]
    fn from(value: f32) -> Self {
        Unit::Pixel(value)
    }
}

impl From<f64> for Unit {
    #[inline]
    fn from(value: f64) -> Self {
        Unit::Pixel(value as _)
    }
}

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlexDirection {
    #[default]
    Row,
    Col,
}

fn px_pct_from(unit: Unit) -> tstyle::LengthPercentage {
    match unit {
        Unit::Pixel(u) => tstyle::LengthPercentage::Length(u),
        Unit::Relative(u) => tstyle::LengthPercentage::Percent(u),
        _ => tstyle::LengthPercentage::ZERO,
    }
}

fn px_pct_auto_from(unit: Unit) -> tstyle::LengthPercentageAuto {
    match unit {
        Unit::Auto => tstyle::LengthPercentageAuto::Auto,
        Unit::Pixel(u) => tstyle::LengthPercentageAuto::Length(u),
        Unit::Relative(u) => tstyle::LengthPercentageAuto::Percent(u),
    }
}

fn dimension_from(unit: Unit) -> tstyle::Dimension {
    match unit {
        Unit::Auto => tstyle::Dimension::Auto,
        Unit::Pixel(u) => tstyle::Dimension::Length(u),
        Unit::Relative(u) => tstyle::Dimension::Percent(u),
    }
}

pub(super) fn taffy_style_from(style: &UIStyle) -> TStyle {
    TStyle {
        display: match style.display {
            Display::Flex => tstyle::Display::Flex,
            Display::Grid => tstyle::Display::Grid,
            Display::None => tstyle::Display::None,
        },
        box_sizing: tstyle::BoxSizing::ContentBox,
        position: match style.mode {
            Mode::Relative => tstyle::Position::Relative,
            Mode::Absolute => tstyle::Position::Absolute,
        },
        inset: geom::Rect {
            left: px_pct_auto_from(style.left),
            right: px_pct_auto_from(style.right),
            top: px_pct_auto_from(style.top),
            bottom: px_pct_auto_from(style.bottom),
        },
        size: geom::Size {
            width: dimension_from(style.width),
            height: dimension_from(style.height),
        },
        min_size: geom::Size {
            width: dimension_from(style.min_width),
            height: dimension_from(style.min_height),
        },
        max_size: geom::Size {
            width: dimension_from(style.max_width),
            height: dimension_from(style.max_height),
        },
        margin: geom::Rect {
            left: px_pct_auto_from(style.margin_left),
            right: px_pct_auto_from(style.margin_right),
            top: px_pct_auto_from(style.margin_top),
            bottom: px_pct_auto_from(style.margin_bottom),
        },
        padding: geom::Rect {
            left: px_pct_from(style.padding_left),
            right: px_pct_from(style.padding_right),
            top: px_pct_from(style.padding_top),
            bottom: px_pct_from(style.padding_bottom),
        },
        align_items: style.align_items.map(|a| match a {
            AlignItems::Start => tstyle::AlignItems::Start,
            AlignItems::End => tstyle::AlignItems::End,
            AlignItems::FlexStart => tstyle::AlignItems::FlexStart,
            AlignItems::FlexEnd => tstyle::AlignItems::FlexEnd,
            AlignItems::Center => tstyle::AlignItems::Center,
            AlignItems::Baseline => tstyle::AlignItems::Baseline,
            AlignItems::Stretch => tstyle::AlignItems::Stretch,
        }),
        align_self: style.align_self.map(|a| match a {
            AlignSelf::Start => tstyle::AlignSelf::Start,
            AlignSelf::End => tstyle::AlignSelf::End,
            AlignSelf::FlexStart => tstyle::AlignSelf::FlexStart,
            AlignSelf::FlexEnd => tstyle::AlignSelf::FlexEnd,
            AlignSelf::Center => tstyle::AlignSelf::Center,
            AlignSelf::Baseline => tstyle::AlignSelf::Baseline,
            AlignSelf::Stretch => tstyle::AlignSelf::Stretch,
        }),
        justify_items: style.justify_items.map(|j| match j {
            JustifyItems::Start => tstyle::JustifyItems::Start,
            JustifyItems::End => tstyle::JustifyItems::End,
            JustifyItems::Center => tstyle::JustifyItems::Center,
            JustifyItems::Baseline => tstyle::JustifyItems::Baseline,
            JustifyItems::Stretch => tstyle::JustifyItems::Stretch,
            JustifyItems::FlexStart => tstyle::JustifyItems::FlexStart,
            JustifyItems::FlexEnd => tstyle::JustifyItems::FlexEnd,
        }),
        justify_self: style.justify_self.map(|j| match j {
            JustifySelf::Start => tstyle::JustifySelf::Start,
            JustifySelf::End => tstyle::JustifySelf::End,
            JustifySelf::Center => tstyle::JustifySelf::Center,
            JustifySelf::Baseline => tstyle::JustifySelf::Baseline,
            JustifySelf::Stretch => tstyle::JustifySelf::Stretch,
            JustifySelf::FlexStart => tstyle::JustifySelf::FlexStart,
            JustifySelf::FlexEnd => tstyle::JustifySelf::FlexEnd,
        }),
        align_content: style.align_content.map(|a| match a {
            AlignContent::Start => tstyle::AlignContent::Start,
            AlignContent::End => tstyle::AlignContent::End,
            AlignContent::FlexStart => tstyle::AlignContent::FlexStart,
            AlignContent::FlexEnd => tstyle::AlignContent::FlexEnd,
            AlignContent::Center => tstyle::AlignContent::Center,
            AlignContent::Stretch => tstyle::AlignContent::Stretch,
            AlignContent::SpaceBetween => tstyle::AlignContent::SpaceBetween,
            AlignContent::SpaceEvenly => tstyle::AlignContent::SpaceEvenly,
            AlignContent::SpaceAround => tstyle::AlignContent::SpaceAround,
        }),
        justify_content: style.justify_content.map(|j| match j {
            JustifyContent::Start => tstyle::JustifyContent::Start,
            JustifyContent::End => tstyle::JustifyContent::End,
            JustifyContent::Center => tstyle::JustifyContent::Center,
            JustifyContent::Stretch => tstyle::JustifyContent::Stretch,
            JustifyContent::SpaceBetween => tstyle::JustifyContent::SpaceBetween,
            JustifyContent::SpaceEvenly => tstyle::JustifyContent::SpaceEvenly,
            JustifyContent::SpaceAround => tstyle::JustifyContent::SpaceAround,
            JustifyContent::FlexStart => tstyle::JustifyContent::FlexStart,
            JustifyContent::FlexEnd => tstyle::JustifyContent::FlexEnd,
        }),
        gap: geom::Size {
            width: px_pct_from(style.gap_horizontal),
            height: px_pct_from(style.gap_vertical),
        },
        flex_direction: match style.flex_direction {
            FlexDirection::Row => tstyle::FlexDirection::Row,
            FlexDirection::Col => tstyle::FlexDirection::Column,
        },
        flex_wrap: match style.flex_wrap {
            FlexWrap::NoWrap => tstyle::FlexWrap::NoWrap,
            FlexWrap::Wrap => tstyle::FlexWrap::Wrap,
            FlexWrap::Reverse => tstyle::FlexWrap::WrapReverse,
        },
        flex_basis: dimension_from(style.flex_basis),
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        // grid_template_rows: todo!(),
        // grid_template_columns: todo!(),
        // grid_auto_rows: todo!(),
        // grid_auto_columns: todo!(),
        // grid_auto_flow: todo!(),
        // grid_row: todo!(),
        // grid_column: todo!(),
        ..Default::default()
    }
}
