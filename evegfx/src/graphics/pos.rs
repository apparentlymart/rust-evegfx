use core::marker::Sized;
use core::ops::{Add, BitAnd, BitOr, Div, Mul, Sub};

/// Vertex type for the `VERTEX2F` display list command.
///
/// This is scaled by the current vertex format in the graphics context and
/// offset by the current coordinate offset.
///
/// | `VertexFormat` | Subpixels per Pixel | Addressable Pixel Range |
/// |--|--|--|
/// | `Whole` | 1 | -16384 to 16383 |
/// | `Half` | 2 | -8192 to 8191 |
/// | `Quarter` | 4 | -4096 to 4095 |
/// | `Eighth` | 8 | -2048 to 2047 |
/// | `Sixteenth` | 16 | -1024 to 1023 |
pub type Vertex2F = Vertex2D<Scaled>;

/// Vertex type for the `VERTEX2II` display list command.
///
/// This always uses whole pixel coordinates but has a limited dimension range
/// from 0 to 511, although drawing commands will also honor the current
/// coordinate offset.
pub type Vertex2II = Vertex2D<Fixed>;

/// Vertex type for representing the global translation offset for subsequent
/// drawing commands.
///
/// Global translation can be used to compensate for an
/// otherwise-limited coordinate range, or to address subpixels at a finer
/// grain than the selected vertex format, particularly if working with
/// [`Vertex2II`](Vertex2II) coordinates. Translation coordinates are given
/// in sixteenths of a pixel and have a range of -37268 to 37267 subpixels,
/// or -2048 to 2047 whole pixels.
pub type GlobalTranslation = Vertex2D<ForGlobalTranslate>;

/// Vertex type for specifying the rendering location of widgets provided by
/// the EVE coprocessor.
///
/// Widget coordinates are given in whole pixels, with a pixel range of
/// -37268 to 37267 pixels.
pub type WidgetPos = Vertex2D<ForCoprocessorWidgets>;

/// Represents bounding rectangles for coprocessor widgets.
pub type WidgetRect = Rect<ForCoprocessorWidgets>;

/// Represents 2D coordinates in a specific coordinate system.
///
/// Most functions that expect vertices as arguments are generic over all
/// types that can convert to `Vertex2D`. When it's obvious from context
/// that such an argument is a vertex, it's conventional to represent it as
/// a bare tuple of X and Y coordinates, `(x, y)`, to avoid the visual noise
/// of explicitly calling the `Vertex2D` constructor function.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Vertex2D<S: CoordinateSystem> {
    pub(crate) x: S::Dim,
    pub(crate) y: S::Dim,
}

impl<S: CoordinateSystem> Vertex2D<S> {
    #[inline]
    pub fn new(x: S::Dim, y: S::Dim) -> Self {
        Self {
            x: S::mask_value(x),
            y: S::mask_value(y),
        }
    }

    #[inline]
    pub fn coords(self) -> (S::Dim, S::Dim) {
        (self.x, self.y)
    }
}

impl<S: CoordinateSystem> core::convert::From<(S::Dim, S::Dim)> for Vertex2D<S> {
    fn from(coords: (S::Dim, S::Dim)) -> Self {
        Self::new(coords.0, coords.1)
    }
}
impl<S: CoordinateSystem> core::convert::From<Vertex2D<S>> for (S::Dim, S::Dim) {
    fn from(vertex: Vertex2D<S>) -> Self {
        vertex.coords()
    }
}

/// Represents a rectangular region a specific coordinate system, as a top
/// left coordinate vertex, a width, and a height.
///
/// Most functions that expect rectangles as arguments are generic over all
/// types that can convert to `Rect`. That includes the following shorthands
/// based on anonymous tuple types:
///
/// * `(x, y, w, h)`
/// * `(v, w, h)` where `v` is anything that can convert to `Vertex2D`.
/// * `(v1, v2)` where both can convert to `Vertex2D`, which is the same as
///   calling `Rect::with_bounds` with those vertices.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Rect<S: CoordinateSystem> {
    pub(crate) x: S::Dim,
    pub(crate) y: S::Dim,
    pub(crate) w: S::Dim,
    pub(crate) h: S::Dim,
}

impl<S: CoordinateSystem> Rect<S> {
    #[inline]
    pub fn new(x: S::Dim, y: S::Dim, w: S::Dim, h: S::Dim) -> Self {
        Self {
            x: x,
            y: y,
            w: w,
            h: h,
        }
    }

    #[inline]
    pub fn with_bounds(top_left: Vertex2D<S>, bottom_right: Vertex2D<S>) -> Self {
        Self {
            x: top_left.x,
            y: top_left.y,
            w: bottom_right.x - top_left.x,
            h: bottom_right.y - top_left.y,
        }
    }

    #[inline]
    pub fn top_left(self) -> Vertex2D<S> {
        Vertex2D::new(self.x, self.y)
    }

    #[inline]
    pub fn bottom_right(self) -> Vertex2D<S> {
        Vertex2D::new(self.x + self.w, self.y + self.h)
    }

    #[inline]
    pub fn size(self) -> (S::Dim, S::Dim) {
        (self.w, self.h)
    }

    #[inline]
    pub fn bounds(self) -> (Vertex2D<S>, Vertex2D<S>) {
        let top_left = Vertex2D::new(self.x, self.y);
        let bottom_right = Vertex2D::new(self.x + self.w, self.y + self.h);
        (top_left, bottom_right)
    }
}

impl<S: CoordinateSystem> core::convert::From<(S::Dim, S::Dim, S::Dim, S::Dim)> for Rect<S> {
    fn from(coords: (S::Dim, S::Dim, S::Dim, S::Dim)) -> Self {
        Self::new(coords.0, coords.1, coords.2, coords.3)
    }
}

impl<S: CoordinateSystem, V: Into<Vertex2D<S>>> core::convert::From<(V, S::Dim, S::Dim)>
    for Rect<S>
{
    fn from(coords: (V, S::Dim, S::Dim)) -> Self {
        let tl: Vertex2D<S> = coords.0.into();
        Self::new(tl.x, tl.y, coords.1, coords.2)
    }
}

impl<S: CoordinateSystem, V1: Into<Vertex2D<S>>, V2: Into<Vertex2D<S>>>
    core::convert::From<(V1, V2)> for Rect<S>
{
    fn from(coords: (V1, V2)) -> Self {
        Self::with_bounds(coords.0.into(), coords.1.into())
    }
}

impl<S: CoordinateSystem> core::convert::From<Rect<S>> for (S::Dim, S::Dim, S::Dim, S::Dim) {
    fn from(rect: Rect<S>) -> (S::Dim, S::Dim, S::Dim, S::Dim) {
        (rect.x, rect.y, rect.w, rect.h)
    }
}

impl<S: CoordinateSystem> core::convert::From<Rect<S>> for (Vertex2D<S>, Vertex2D<S>) {
    fn from(rect: Rect<S>) -> (Vertex2D<S>, Vertex2D<S>) {
        rect.bounds()
    }
}

impl<S: CoordinateSystem> core::convert::From<Rect<S>> for ((S::Dim, S::Dim), (S::Dim, S::Dim)) {
    fn from(rect: Rect<S>) -> ((S::Dim, S::Dim), (S::Dim, S::Dim)) {
        let bounds = rect.bounds();
        (bounds.0.into(), bounds.1.into())
    }
}

impl<S: CoordinateSystem> core::convert::From<Rect<S>> for (Vertex2D<S>, S::Dim, S::Dim) {
    fn from(rect: Rect<S>) -> (Vertex2D<S>, S::Dim, S::Dim) {
        (Vertex2D::new(rect.x, rect.y), rect.w, rect.h)
    }
}

pub trait CoordinateSystem {
    type Dim: Sized
        + Clone
        + Copy
        + Add<Output = Self::Dim>
        + Sub<Output = Self::Dim>
        + Mul<Output = Self::Dim>
        + Div<Output = Self::Dim>
        + BitAnd<Output = Self::Dim>
        + BitOr<Output = Self::Dim>;

    fn mask_value(v: Self::Dim) -> Self::Dim;
}

pub enum Scaled {}
impl CoordinateSystem for Scaled {
    type Dim = i16;

    fn mask_value(v: Self::Dim) -> Self::Dim {
        unsafe {
            let raw: u16 = core::mem::transmute(v);
            core::mem::transmute(raw & 0b11111111111111)
        }
    }
}

pub enum Fixed {}
impl CoordinateSystem for Fixed {
    type Dim = u16;

    fn mask_value(v: Self::Dim) -> Self::Dim {
        v & 0b111111111
    }
}

pub enum ForGlobalTranslate {}
impl CoordinateSystem for ForGlobalTranslate {
    type Dim = i16;

    fn mask_value(v: Self::Dim) -> Self::Dim {
        v
    }
}

pub enum ForCoprocessorWidgets {}
impl CoordinateSystem for ForCoprocessorWidgets {
    type Dim = i16;

    fn mask_value(v: Self::Dim) -> Self::Dim {
        v
    }
}
