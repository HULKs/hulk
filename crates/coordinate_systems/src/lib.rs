mod framed;
mod into;
mod transform;

pub use framed::Framed;
pub use into::{IntoFramed, IntoTransform};
pub use transform::Transform;

#[macro_export]
macro_rules! transform {
    ($source:ty => $destination:ty; $inner:expr) => {
        $inner.framed_transform::<$source, $destination>()
    };
}
