use anyhow::bail;

pub fn gather_results<T>(
    results: Vec<anyhow::Result<T>>,
    error_message: &'static str,
) -> anyhow::Result<()> {
    if results.iter().any(|result| result.is_err()) {
        for result in results {
            if let Err(error) = result {
                eprintln!("{error:?}");
            }
        }
        bail!(error_message);
    }

    Ok(())
}
