use std::{
    fs::File,
    io::{Seek, SeekFrom},
    path::Path,
    time::SystemTime,
};

use bincode::deserialize_from;
use color_eyre::{eyre::WrapErr, Result};

#[derive(Debug)]
pub struct RecordingIndex {
    file: File,
    frames: Vec<RecordingFrame>,
}

impl RecordingIndex {
    pub fn read_from(recording_file: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(&recording_file)
            .wrap_err_with(|| format!("failed to open {}", recording_file.as_ref().display()))?;
        Self::collect_frames(file).wrap_err("failed to collect frames")
    }

    fn collect_frames(mut recording_file: File) -> Result<Self> {
        let mut frames = Vec::new();

        recording_file
            .seek(SeekFrom::End(0))
            .wrap_err("failed to seek to end of file")?;
        let end_of_file_offset = recording_file
            .stream_position()
            .wrap_err("failed to get stream position of end of file")?;
        dbg!(end_of_file_offset);
        recording_file.rewind().wrap_err("failed to rewind file")?;

        let mut offset = 0;
        while offset < end_of_file_offset {
            let timestamp = deserialize_from(&mut recording_file)
                .wrap_err("failed to deserialize timestamp")?;
            let length = deserialize_from(&mut recording_file)
                .wrap_err("failed to deserialize data length")?;
            dbg!(offset, timestamp, length);
            recording_file
                .seek(SeekFrom::Current(length as i64))
                .wrap_err("failed to seek to end of data")?;
            frames.push(RecordingFrame {
                timestamp,
                offset: offset.try_into().unwrap(),
                length,
            });
            offset = recording_file
                .stream_position()
                .wrap_err("failed to get stream position")?;
        }

        recording_file.rewind().wrap_err("failed to rewind file")?;

        Ok(Self {
            file: recording_file,
            frames,
        })
    }

    pub fn before_or_equal_of(&self, timestamp: SystemTime) -> Option<&RecordingFrame> {
        self.frames
            .iter()
            .rev()
            .find(|frame| frame.timestamp <= timestamp)
    }
}

#[derive(Debug)]
pub struct RecordingFrame {
    timestamp: SystemTime,
    offset: usize,
    length: usize,
}
