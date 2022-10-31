use std::path::Path;

use itertools::Itertools;
use syn::{spanned::Spanned, File, Item};

use crate::output::output_diagnostic;

pub fn check<P>(file_path: P, buffer: &str, file: &File) -> bool
where
    P: AsRef<Path>,
{
    for (item_before, item_after) in file
        .items
        .iter()
        .filter(|item| {
            matches!(
                item,
                Item::Enum(_) | Item::Fn(_) | Item::Impl(_) | Item::Struct(_) | Item::Trait(_)
            )
        })
        .tuple_windows()
    {
        let line_before = item_before.span().end().line;
        let line_after = item_after.span().start().line;
        if line_before + 1 == line_after {
            output_diagnostic(
                file_path,
                buffer,
                line_before..line_after + 1,
                "error: `enum`, `fn`, `impl`, `struct`, and `trait` must be separated by one empty line",
            );
            return false;
        }
        if let Item::Impl(item) = item_before {
            for (item_before, item_after) in item.items.iter().tuple_windows() {
                let line_before = item_before.span().end().line;
                let line_after = item_after.span().start().line;
                if line_before + 1 == line_after {
                    output_diagnostic(
                        file_path,
                        buffer,
                        line_before..line_after + 1,
                        "error: items in `impl` must be separated by one empty line",
                    );
                    return false;
                }
            }
        }
    }
    true
}
