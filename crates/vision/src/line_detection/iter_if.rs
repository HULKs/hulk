use std::slice::Iter;

pub fn iter_if<T>(enable: bool, iterator: Iter<T>) -> Iter<T> {
    if enable {
        iterator
    } else {
        [].iter()
    }
}
