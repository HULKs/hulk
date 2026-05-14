use std::{future::pending, sync::Arc};

use color_eyre::Result;

use ros_z::{IntoEyreResultExt, prelude::*};
use types::{
    field_border::FieldBorder, filtered_segments::FilteredSegments, image_segments::ImageSegments,
};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("segment_filter")
        .build()
        .await
        .into_eyre()?;
    let _field_border_sub = node
        .subscriber::<FieldBorder>("field_border")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _image_segments_sub = node
        .subscriber::<ImageSegments>("image_segments")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _filtered_segments_pub = node
        .publisher::<FilteredSegments>("filtered_segments")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
