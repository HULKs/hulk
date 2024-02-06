mod framed;
mod framed_nalgebra;
mod into;
mod transform;
mod transform_nalgebra;

pub use framed::Framed;
pub use framed_nalgebra::{center, distance, distance_squared};
pub use into::{IntoFramed, IntoTransform};
pub use transform::Transform;

#[macro_export]
macro_rules! transform {
    ($source:ty => $destination:ty; $inner:expr) => {
        $inner.framed_transform::<$source, $destination>()
    };
}
