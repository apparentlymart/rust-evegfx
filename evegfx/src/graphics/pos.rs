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

impl<S: CoordinateSystem> core::convert::Into<(S::Dim, S::Dim)> for Vertex2D<S> {
    fn into(self) -> (S::Dim, S::Dim) {
        self.coords()
    }
}

type WidgetDim = <ForCoprocessorWidgets as CoordinateSystem>::Dim;

/// Description of a rectangular region to render a coprocessor widget into.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct WidgetRect {
    pub(crate) x: WidgetDim,
    pub(crate) y: WidgetDim,
    pub(crate) w: WidgetDim,
    pub(crate) h: WidgetDim,
}

impl WidgetRect {
    #[inline]
    pub const fn new(x: WidgetDim, y: WidgetDim, w: WidgetDim, h: WidgetDim) -> Self {
        Self {
            x: x,
            y: y,
            w: w,
            h: h,
        }
    }

    #[inline]
    pub fn top_left(self) -> WidgetPos {
        WidgetPos::new(self.x, self.y)
    }

    #[inline]
    pub fn bottom_right(self) -> WidgetPos {
        WidgetPos::new(self.x + self.w, self.y + self.h)
    }

    #[inline]
    pub fn size(self) -> (WidgetDim, WidgetDim) {
        (self.w, self.h)
    }
}

impl core::convert::From<(WidgetDim, WidgetDim, WidgetDim, WidgetDim)> for WidgetRect {
    fn from(coords: (WidgetDim, WidgetDim, WidgetDim, WidgetDim)) -> Self {
        Self::new(coords.0, coords.1, coords.2, coords.3)
    }
}

impl core::convert::Into<(WidgetDim, WidgetDim, WidgetDim, WidgetDim)> for WidgetRect {
    fn into(self) -> (WidgetDim, WidgetDim, WidgetDim, WidgetDim) {
        (self.x, self.y, self.w, self.h)
    }
}

pub trait CoordinateSystem {
    type Dim: Sized
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
