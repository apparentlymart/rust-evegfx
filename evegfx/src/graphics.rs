//! Data types to represent geometry and colors for various graphics operations.

mod color;
mod pos;

#[doc(inline)]
pub use pos::Vertex2D;

#[doc(inline)]
pub use pos::Rect;

#[doc(inline)]
pub use color::RGB;

#[doc(inline)]
pub use color::RGBA;

#[doc(inline)]
pub use pos::Vertex2F;

#[doc(inline)]
pub use pos::Vertex2II;

#[doc(inline)]
pub use pos::GlobalTranslation;

#[doc(inline)]
pub use pos::WidgetPos;

#[doc(inline)]
pub use pos::WidgetRect;

#[doc(inline)]
pub use pos::ScissorPos;

#[doc(inline)]
pub use pos::ScissorRect;
