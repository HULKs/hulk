use std::{fmt::Display, ops::Range, path::Path};

pub fn output_diagnostic<P, D>(file_path: P, buffer: &str, lines1: Range<usize>, message: D)
where
    P: AsRef<Path>,
    D: Display + Send + Sync + 'static,
{
    eprintln!("{message}");
    eprintln!("  --> {}:{}", file_path.as_ref().display(), lines1.start);
    let number_of_digits = ((lines1.end as f32).log10() + 1.0) as usize;
    eprintln!("{:1$} |", "", number_of_digits);
    let buffer_lines: Vec<_> = buffer.split('\n').collect();
    for line1 in lines1 {
        eprintln!(
            "{0:1$} | {2}",
            line1,
            number_of_digits,
            buffer_lines[line1 - 1]
        );
    }
    eprintln!("{:1$} |", "", number_of_digits);
    eprintln!();
}
