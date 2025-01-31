use taffy::geometry as geom;
use taffy::prelude::TaffyZero;
use taffy::style as tstyle;
use taffy::Style as TStyle;

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Style {
    pub layout: LayoutStyle,
}

impl Style {
    #[inline]
    pub fn flex(mut self) -> Self {
        self.layout.display = Display::Flex;
        self
    }

    #[inline]
    pub fn grid(mut self) -> Self {
        self.layout.display = Display::Grid;
        self
    }

    #[inline]
    pub fn hide(mut self) -> Self {
        self.layout.display = Display::None;
        self
    }

    pub fn relative(mut self) -> Self {
        self.layout.mode = Mode::Relative;
        self
    }

    pub fn absolute(mut self) -> Self {
        self.layout.mode = Mode::Absolute;
        self
    }

    #[inline]
    pub fn flex_col(mut self) -> Self {
        self.layout.flex_direction = FlexDirection::Col;
        self
    }

    #[inline]
    pub fn flex_row(mut self) -> Self {
        self.layout.flex_direction = FlexDirection::Row;
        self
    }

    #[inline]
    pub fn flex_grow(mut self, value: f32) -> Self {
        debug_assert!(value >= 0.0, "flex_grow should be >= 0.0");
        debug_assert!(value <= 0.0, "flex_grow should be <= 0.0");
        self.layout.flex_grow = value;
        self
    }

    #[inline]
    pub fn flex_shrink(mut self, value: f32) -> Self {
        debug_assert!(value >= 0.0, "flex_shrink should be >= 0.0");
        debug_assert!(value <= 0.0, "flex_shrink should be <= 0.0");
        self.layout.flex_shrink = value;
        self
    }

    #[inline]
    pub fn flex_basis(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.flex_basis = unit.into();
        self
    }

    #[inline]
    pub fn top(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.top = unit.into();
        self
    }

    #[inline]
    pub fn bottom(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.bottom = unit.into();
        self
    }

    #[inline]
    pub fn left(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.left = unit.into();
        self
    }

    #[inline]
    pub fn right(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.right = unit.into();
        self
    }

    #[inline]
    pub fn size(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.layout.width = unit;
        self.layout.height = unit;
        self
    }

    #[inline]
    pub fn size_auto(self) -> Self {
        self.size(Unit::Auto)
    }

    #[inline]
    pub fn width(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.width = unit.into();
        self
    }

    #[inline]
    pub fn width_auto(mut self) -> Self {
        self.layout.width = Unit::Auto;
        self
    }

    #[inline]
    pub fn height(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.height = unit.into();
        self
    }

    #[inline]
    pub fn height_auto(mut self) -> Self {
        self.layout.height = Unit::Auto;
        self
    }

    #[inline]
    pub fn min_size(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.layout.min_width = unit;
        self.layout.min_height = unit;
        self
    }

    #[inline]
    pub fn min_width(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.min_width = unit.into();
        self
    }

    #[inline]
    pub fn min_width_auto(mut self) -> Self {
        self.layout.min_width = Unit::Auto;
        self
    }

    #[inline]
    pub fn min_height(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.min_height = unit.into();
        self
    }

    #[inline]
    pub fn min_height_auto(mut self) -> Self {
        self.layout.min_height = Unit::Auto;
        self
    }

    #[inline]
    pub fn max_size(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.layout.max_width = unit;
        self.layout.max_height = unit;
        self
    }

    #[inline]
    pub fn max_width(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.max_width = unit.into();
        self
    }

    #[inline]
    pub fn max_width_auto(mut self) -> Self {
        self.layout.max_width = Unit::Auto;
        self
    }

    #[inline]
    pub fn max_height(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.max_height = unit.into();
        self
    }

    #[inline]
    pub fn max_height_auto(mut self) -> Self {
        self.layout.max_height = Unit::Auto;
        self
    }

    #[inline]
    pub fn padding(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.layout.padding_top = unit;
        self.layout.padding_bottom = unit;
        self.layout.padding_left = unit;
        self.layout.padding_right = unit;
        self
    }

    #[inline]
    pub fn padding_y(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.layout.padding_top = unit;
        self.layout.padding_bottom = unit;
        self
    }

    #[inline]
    pub fn padding_x(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.layout.padding_left = unit;
        self.layout.padding_right = unit;
        self
    }

    #[inline]
    pub fn padding_top(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.padding_top = unit.into();
        self
    }

    #[inline]
    pub fn padding_bottom(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.padding_bottom = unit.into();
        self
    }

    #[inline]
    pub fn padding_left(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.padding_left = unit.into();
        self
    }

    #[inline]
    pub fn padding_right(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.padding_right = unit.into();
        self
    }

    #[inline]
    pub fn margin(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.layout.margin_top = unit;
        self.layout.margin_bottom = unit;
        self.layout.margin_left = unit;
        self.layout.margin_right = unit;
        self
    }

    #[inline]
    pub fn margin_y(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.layout.margin_top = unit;
        self.layout.margin_bottom = unit;
        self
    }

    #[inline]
    pub fn margin_x(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.layout.margin_left = unit;
        self.layout.margin_right = unit;
        self
    }

    #[inline]
    pub fn margin_top(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.margin_top = unit.into();
        self
    }

    #[inline]
    pub fn margin_bottom(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.margin_bottom = unit.into();
        self
    }

    #[inline]
    pub fn margin_left(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.margin_left = unit.into();
        self
    }

    #[inline]
    pub fn margin_right(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.margin_right = unit.into();
        self
    }

    #[inline]
    pub fn gap(mut self, unit: impl Into<Unit>) -> Self {
        let unit = unit.into();
        self.layout.gap_vertical = unit;
        self.layout.gap_horizontal = unit;
        self
    }

    #[inline]
    pub fn gap_x(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.gap_horizontal = unit.into();
        self
    }

    #[inline]
    pub fn gap_y(mut self, unit: impl Into<Unit>) -> Self {
        self.layout.gap_vertical = unit.into();
        self
    }

    #[inline]
    pub fn align_content_start(mut self) -> Self {
        self.layout.align_content = Some(Align::Start);
        self
    }

    #[inline]
    pub fn align_content_center(mut self) -> Self {
        self.layout.align_content = Some(Align::Center);
        self
    }

    #[inline]
    pub fn align_content_end(mut self) -> Self {
        self.layout.align_content = Some(Align::End);
        self
    }

    #[inline]
    pub fn justify_content_start(mut self) -> Self {
        self.layout.justify_content = Some(Justify::Start);
        self
    }

    #[inline]
    pub fn justify_content_center(mut self) -> Self {
        self.layout.justify_content = Some(Justify::Center);
        self
    }

    #[inline]
    pub fn justify_content_end(mut self) -> Self {
        self.layout.justify_content = Some(Justify::End);
        self
    }

    #[inline]
    pub fn justify_content_between(mut self) -> Self {
        self.layout.justify_content = Some(Justify::Between);
        self
    }

    #[inline]
    pub fn justify_content_evenly(mut self) -> Self {
        self.layout.justify_content = Some(Justify::Evenly);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LayoutStyle {
    pub display: Display,
    pub mode: Mode,

    pub flex_direction: FlexDirection,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Unit,

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

    pub align_content: Option<Align>,
    pub justify_content: Option<Justify>,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            display: Display::Flex,
            mode: Mode::Relative,
            flex_direction: FlexDirection::Row,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Unit::Auto,
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
        }
    }
}

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    Between,
    Evenly,
}

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    #[default]
    Relative,
    Absolute,
}

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

pub(super) fn taffy_style_from(style: &LayoutStyle) -> TStyle {
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
        // align_items: todo!(),
        // align_self: todo!(),
        // justify_items: todo!(),
        // justify_self: todo!(),
        // align_content: todo!(),
        // justify_content: todo!(),
        gap: geom::Size {
            width: px_pct_from(style.gap_horizontal),
            height: px_pct_from(style.gap_vertical),
        },
        flex_direction: match style.flex_direction {
            FlexDirection::Row => tstyle::FlexDirection::Row,
            FlexDirection::Col => tstyle::FlexDirection::Column,
        },
        // flex_wrap: todo!(),
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
