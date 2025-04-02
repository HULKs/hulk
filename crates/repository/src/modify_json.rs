use std::path::Path;

use color_eyre::{eyre::Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use tokio::fs::{read_to_string, write};

/// Modifies a JSON file in place.
///
/// This function reads the contents of a JSON file, deserializes it into a value, applies a
/// modification to the value, serializes the modified value back into JSON, and writes the JSON
/// back to the file.
pub async fn modify_json_inplace<I, O>(
    path: impl AsRef<Path>,
    modification: impl FnOnce(I) -> O,
) -> Result<()>
where
    for<'de> I: Deserialize<'de>,
    O: Serialize,
{
    let file_contents = read_to_string(&path)
        .await
        .wrap_err("failed to read contents")?;

    let value = serde_json::from_str(&file_contents).wrap_err("failed to deserialize value")?;

    let out = modification(value);

    let json = to_string_pretty(&out).wrap_err("failed to serialize value")?;
    let file_contents = json + "\n";
    write(&path, file_contents.as_bytes())
        .await
        .wrap_err("failed to write file contents back")?;

    Ok(())
}
