use std::{cell::RefCell, cmp::Ordering, collections::HashMap, error::Error, fmt};

use crate::layout::{Direction, Rect};

#[derive(Debug, Clone)]
pub struct LayoutOverflowError {
    pub min_size: u16,
    pub actual_size: u16,
    pub direction: Direction,
}

impl fmt::Display for LayoutOverflowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self {
            min_size,
            actual_size,
            direction,
        } = self;
        let cols_rows = match direction {
            Direction::Horizontal => "columns",
            Direction::Vertical => "rows",
        };
        write!(f, "layout needs at least {min_size} {cols_rows}, but the passed rect only allows for {actual_size} {cols_rows}")
    }
}

impl Error for LayoutOverflowError {}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct FlexSpace {
    pub base_size: u16,
    pub growth: Option<FlexGrow>,
    pub shrinkage: Option<FlexShrink>,
}

impl FlexSpace {
    pub const fn new(base_size: u16) -> Self {
        Self {
            base_size,
            growth: None,
            shrinkage: None,
        }
    }

    pub fn shrinkage<Shrink: Into<FlexShrink>>(mut self, shrinkage: Shrink) -> Self {
        self.shrinkage = Some(shrinkage.into());
        self
    }

    /// Shorthand for default shrinkage of 1 and min_size 0
    pub fn shrinkable(self) -> Self {
        self.shrinkage(FlexShrink::new(1))
    }

    pub fn growth<Grow: Into<FlexGrow>>(mut self, growth: Grow) -> Self {
        self.growth = Some(growth.into());
        self
    }

    /// Shorthand for default growth of 1 and max_size u16::MAX
    pub fn growable(self) -> Self {
        self.growth(FlexGrow::new(1))
    }
}

impl From<u16> for FlexSpace {
    fn from(base_size: u16) -> Self {
        Self::new(base_size)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct FlexShrink {
    /// The flex_share value determines how much this space shrinks in relation
    /// to all the other shrinking spaces. This is always a relative amount, but it can be used
    /// as percentages if you choose values that add up to a 100.
    ///
    /// Values of `[25, 25, 50]` are equivalent to `[1, 1, 2]`, `[3, 3, 6]`,
    /// `[250, 250, 500]`, and so on.
    ///
    /// The above example would mean that if you have three spaces which together have
    /// an base_size that is too large to fit into the layout - the last space shrinks twice
    /// as much to fit into the layout.
    pub flex_share: usize,
    /// A minimum size for this space - It can't shrink further than to this size.
    ///
    /// Defaults to `0`
    pub min_size: u16,
}

impl FlexShrink {
    pub const fn new(flex_share: usize) -> Self {
        Self {
            flex_share,
            min_size: 0,
        }
    }

    pub const fn min_size(mut self, min_size: u16) -> Self {
        self.min_size = min_size;
        self
    }
}

impl From<usize> for FlexShrink {
    fn from(share: usize) -> Self {
        Self::new(share)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct FlexGrow {
    /// The flex_share value determines how much this space grows in relation
    /// to all the other growing spaces. This is always a relative amount, but it can be used
    /// as percentages if you choose values that add up to a 100.
    ///
    /// Values of `[25, 25, 50]` are equivalent to `[1, 1, 2]`, `[3, 3, 6]`,
    /// `[250, 250, 500]`, and so on.
    ///
    /// The above example would mean that if you have three spaces which together have
    /// an base_size that leaves extra space in the layout - the last one will grow
    /// twice as much to fill up the layout.
    ///
    /// If the base_size of the spaces is the same (e.g. 0), this would mean the
    /// first two spaces each take up 25%, and the third space 50% of the layout.
    pub flex_share: usize,
    pub max_size: u16,
}

impl FlexGrow {
    pub const fn new(flex_share: usize) -> Self {
        Self {
            flex_share,
            max_size: u16::MAX,
        }
    }

    pub const fn max_size(mut self, max_size: u16) -> Self {
        self.max_size = max_size;
        self
    }
}

impl From<usize> for FlexGrow {
    fn from(share: usize) -> Self {
        Self::new(share)
    }
}

/// The FlexLayout is a powerful layouting component, heavily inspired by `flex`
/// layouts in CSS.
///
/// You define a layout by passing a list of [FlexSpace]s. Each space defines
/// rules for the "ideal" size of that space (like `flex-basis` in CSS), and how fast it will
/// shrink or grow in relation to the other items, if the layout is too small or
/// too big to exactly fit all "ideal" sizes (like `flex-shrink`/`flex-grow` in CSS).
///
/// A 3-column layout where each column has the same size can look like this:
///
/// ```
/// # use tui::layout::{Rect, Direction};
/// # use tui::flex_layout::{FlexLayout, FlexSpace, FlexGrow};
/// let layout = FlexLayout::new(Direction::Horizontal)
///     .gap(FlexSpace::new(2))
///     .margins(FlexSpace::new(2))
///     .flex_spaces([
///         FlexSpace::new(0).growable(),
///         FlexSpace::new(0).growable(),
///         FlexSpace::new(0).growable(),
///     ]);
///
/// let screen_area = Rect { x: 0, y: 0, width: 100, height: 100 };
/// let column_areas = layout.split(screen_area);
///
/// // If the space cannot be distributed exactly, the first item(s) in the list
/// // get(s) dips on the leftover space.
/// assert_eq!(column_areas, vec![
///     Rect { x: 2, y: 0, width: 31, height: 100 },
///     Rect { x: 35, y: 0, width: 31, height: 100 },
///     Rect { x: 68, y: 0, width: 30, height: 100 },
/// ])
///
/// ```
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct FlexLayout {
    pub direction: Direction,
    pub flex_spaces: Vec<FlexSpace>,
    pub gap: Option<FlexSpace>,
    pub margin_start: Option<FlexSpace>,
    pub margin_end: Option<FlexSpace>,
}

// Utility enum during layout calculations
enum FlexChange {
    Shrinking,
    Growing,
}

// Utility struct during layout calculations
#[derive(Debug)]
struct SpaceSize {
    base_size: u16,
    flex_share: usize,
    // How much is this space allowed to grow/shrink at max?
    size_delta_max: u16,
    // How much will this space actually grow/shrink?
    size_delta: u16,
    is_virtual: bool,
}

type CacheKey = (Rect, FlexLayout);
type CacheVal = (Vec<Rect>, Option<LayoutOverflowError>);

thread_local! {
    static LAYOUT_CACHE: RefCell<HashMap<CacheKey, CacheVal>> = RefCell::new(HashMap::new());
}

impl FlexLayout {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            flex_spaces: Vec::new(),
            gap: None,
            margin_start: None,
            margin_end: None,
        }
    }

    pub fn flex_spaces<Items: Into<Vec<FlexSpace>>>(mut self, flex_spaces: Items) -> FlexLayout {
        self.flex_spaces = flex_spaces.into();
        self
    }

    pub fn gap<FS: Into<FlexSpace>>(mut self, gap: FS) -> FlexLayout {
        self.gap = Some(gap.into());
        self
    }

    pub fn margin_start<FS: Into<FlexSpace>>(mut self, margin: FS) -> FlexLayout {
        self.margin_start = Some(margin.into());
        self
    }

    pub fn margin_end<FS: Into<FlexSpace>>(mut self, margin: FS) -> FlexLayout {
        self.margin_end = Some(margin.into());
        self
    }

    pub fn margins<FS: Into<FlexSpace>>(mut self, margin: FS) -> FlexLayout {
        let margin = margin.into();
        self.margin_start = Some(margin.to_owned());
        self.margin_end = Some(margin);
        self
    }

    /// Algorithm:
    ///
    /// Compare actual size with ideal size sum
    ///
    /// - `if available_size == base_size_sum`
    ///     - return ideal sizes
    /// - `if available_size > base_size_sum`
    ///     - Let all growable elements grow by
    ///       `((flex_share / flex_share_sum) * rest_amount).min(max_size)`
    ///     - Repeat with all non-maxed-out elements until all elements are
    ///       maxed out or rest_amount is 0.
    /// - `if available_size < base_size_sum`
    ///     - Let all shrinkable elements shrink by
    ///       `((flex_share / flex_share_sum) * rest_amount).max(min_size)`
    ///     - Repeat with all non-minned-out elements until all elements are
    ///       minned out or rest_amount is 0.
    ///     - If we stopped because all elements were minned out, return an Err in the
    ///       try_split method.
    fn base_split(&self, area: Rect) -> (Vec<Rect>, Option<LayoutOverflowError>) {
        // We add "virtual" spaces, which are spaces we insert based on gap and
        // margin properties. They will be filtered out before returning the sizes.
        let all_spaces: Vec<(&FlexSpace, bool)> = self
            .flex_spaces
            .iter()
            .enumerate()
            .flat_map(|(i, space)| {
                let mut spaces = Vec::new();

                let before_space = if i == 0 {
                    self.margin_start.as_ref()
                } else {
                    self.gap.as_ref()
                };

                if let Some(before_space) = before_space {
                    spaces.push((before_space, true));
                }

                spaces.push((space, false));

                if i == self.flex_spaces.len() - 1 {
                    if let Some(after_space) = self.margin_end.as_ref() {
                        spaces.push((after_space, true));
                    }
                }

                spaces
            })
            .collect();

        let base_size_sum = all_spaces
            .iter()
            .fold(0_u16, |sum, (space, _)| sum.saturating_add(space.base_size));

        let available_size = match self.direction {
            Direction::Horizontal => area.width,
            Direction::Vertical => area.height,
        };

        // do the items have to shrink, grow, or do nothing to fit the available size?
        let flex_todo = match base_size_sum.cmp(&available_size) {
            Ordering::Equal => None,
            Ordering::Less => Some((FlexChange::Growing, available_size - base_size_sum)),
            Ordering::Greater => Some((FlexChange::Shrinking, base_size_sum - available_size)),
        };

        let mut overflow_error: Option<LayoutOverflowError> = None;

        let new_sizes: Vec<(u16, bool)> = if let Some((flex_change, delta)) = flex_todo {
            let mut rest_delta = delta;

            // True when none of the items got any growth last iteration, that
            // means we are certain the rest is just leftovers from rounding.
            // Strategy: just add 1 to all items starting at the top until
            // the rest is used up.
            let mut is_rest_iteration = false;

            // Initialize space size deltas with 0 (== ideal size)
            let mut spaces: Vec<SpaceSize> = all_spaces
                .into_iter()
                .map(|(space, is_virtual)| {
                    let flex_and_max = match flex_change {
                        FlexChange::Growing => space.growth.as_ref().map(|growth| {
                            let sanitized_max = growth.max_size.max(space.base_size);
                            let max_delta = sanitized_max - space.base_size;
                            (growth.flex_share, max_delta)
                        }),
                        FlexChange::Shrinking => space.shrinkage.as_ref().map(|shrinkage| {
                            let sanitized_min = shrinkage.min_size.min(space.base_size);
                            let max_delta = space.base_size - sanitized_min;
                            (shrinkage.flex_share, max_delta)
                        }),
                    };
                    // We can use flex_share 0 and limit 0 as a default because
                    // they have the same effect as if the space didn't grow/shrink.
                    let (flex_share, size_delta_max) = flex_and_max.unwrap_or((0, 0));

                    SpaceSize {
                        base_size: space.base_size,
                        flex_share,
                        size_delta_max,
                        size_delta: 0,
                        is_virtual,
                    }
                })
                .collect();

            while rest_delta > 0 {
                let resizable_spaces: Vec<_> = spaces
                    .iter_mut()
                    .filter(|space| space.flex_share > 0 && space.size_delta < space.size_delta_max)
                    .collect();

                if resizable_spaces.is_empty() {
                    // None of the spaces can resize any further but we didn't
                    // grow as much as we could or shrink as much as we should.
                    // If this happened during shrinking, we'll need to save the
                    // layout overflow error.
                    if matches!(flex_change, FlexChange::Shrinking) {
                        overflow_error = Some(LayoutOverflowError {
                            min_size: available_size + rest_delta,
                            actual_size: available_size,
                            direction: self.direction.to_owned(),
                        });
                    }
                    break;
                }

                // We know this cannot be 0 because we have at least one space
                // in this list which survived the `space.flex_share > 0` filter
                let flex_sum: usize = resizable_spaces.iter().map(|space| space.flex_share).sum();

                let mut new_rest_delta = rest_delta;

                for space in resizable_spaces {
                    let mut iteration_delta: u16 = if is_rest_iteration {
                        1
                    } else {
                        let flex_factor = space.flex_share as f64 / flex_sum as f64;
                        let delta = flex_factor * rest_delta as f64;
                        delta.floor() as u16
                    };

                    iteration_delta = iteration_delta.min(space.size_delta_max - space.size_delta);

                    space.size_delta += iteration_delta;
                    new_rest_delta -= iteration_delta;

                    if new_rest_delta == 0 {
                        break;
                    }
                }

                // Next iteration is rest iteration if the for loop above didn't have
                // any further effects.
                is_rest_iteration = new_rest_delta == rest_delta;
                rest_delta = new_rest_delta;
            }

            spaces
                .iter()
                .map(|space| match flex_change {
                    FlexChange::Growing => (space.base_size + space.size_delta, space.is_virtual),
                    FlexChange::Shrinking => (space.base_size - space.size_delta, space.is_virtual),
                })
                .collect()
        } else {
            all_spaces
                .into_iter()
                .map(|(space, is_virtual)| (space.base_size, is_virtual))
                .collect()
        };

        // Get all the relative space coordinates
        // Vec<(x_delta, width)> | Vec<(y_delta, height)>
        // Also filter out the virtual spaces
        let rect_sizes: Vec<(u16, u16)> = new_sizes
            .into_iter()
            .fold(
                (Vec::new(), 0_u16),
                |(mut acc, position_delta), (size, is_virtual)| {
                    if !is_virtual {
                        acc.push((position_delta, size))
                    }
                    (acc, position_delta + size)
                },
            )
            .0;

        let Rect {
            x,
            y,
            height,
            width,
        } = area;

        let new_rects = match self.direction {
            Direction::Horizontal => rect_sizes
                .into_iter()
                .map(|(x_delta, space_width)| {
                    // handle overflows by cutting the space off
                    let x_delta = x_delta.min(width);
                    let space_width = space_width.min(width - x_delta);

                    Rect {
                        x: x + x_delta,
                        width: space_width,
                        y,
                        height,
                    }
                })
                .collect(),
            Direction::Vertical => rect_sizes
                .into_iter()
                .map(|(y_delta, space_height)| {
                    // handle overflows by cutting the space off
                    let y_delta = y_delta.min(height);
                    let space_height = space_height.min(height - y_delta);
                    Rect {
                        x,
                        width,
                        y: y + y_delta,
                        height: space_height,
                    }
                })
                .collect(),
        };

        (new_rects, overflow_error)
    }

    fn base_split_memoized(&self, area: Rect) -> (Vec<Rect>, Option<LayoutOverflowError>) {
        // TODO: Maybe use a fixed size cache ?
        LAYOUT_CACHE.with(|c| {
            c.borrow_mut()
                .entry((area, self.clone()))
                .or_insert_with(|| self.base_split(area))
                .clone()
        })
    }

    pub fn split(&self, area: Rect) -> Vec<Rect> {
        // ignore overflows
        self.base_split_memoized(area).0
    }

    pub fn try_split(&self, area: Rect) -> Result<Vec<Rect>, LayoutOverflowError> {
        // Error for overflows
        match self.base_split_memoized(area) {
            (_, Some(err)) => Err(err),
            (result, None) => Ok(result),
        }
    }
}
