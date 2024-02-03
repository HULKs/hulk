use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    time::SystemTime,
};

use bincode::deserialize_from;
use color_eyre::{eyre::WrapErr, Result};

#[derive(Debug)]
pub struct RecordingIndex {
    file: File,
    frames: Vec<RecordingFrameMetadata>,
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
        recording_file.rewind().wrap_err("failed to rewind file")?;

        let mut offset = 0;
        while offset < end_of_file_offset {
            let timestamp = deserialize_from(&mut recording_file)
                .wrap_err("failed to deserialize timestamp")?;
            let length = deserialize_from(&mut recording_file)
                .wrap_err("failed to deserialize data length")?;
            let header_offset = recording_file
                .stream_position()
                .wrap_err("failed to get stream position")?
                - offset;
            recording_file
                .seek(SeekFrom::Current(length as i64))
                .wrap_err("failed to seek to end of data")?;
            frames.push(RecordingFrameMetadata {
                timestamp,
                offset: offset.try_into().unwrap(),
                header_offset: header_offset.try_into().unwrap(),
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

    pub fn before_or_equal_of(&mut self, timestamp: SystemTime) -> Result<Option<RecordingFrame>> {
        let frame = match self
            .frames
            .iter()
            .rev()
            .find(|frame| frame.timestamp <= timestamp)
        {
            Some(frame) => frame,
            None => return Ok(None),
        };
        self.file
            .seek(SeekFrom::Start(
                (frame.offset + frame.header_offset).try_into().unwrap(),
            ))
            .wrap_err("failed to seek to frame")?;
        let mut data = Vec::new();
        data.resize_with(frame.length, Default::default);
        self.file
            .read_exact(&mut data)
            .wrap_err("failed to read from recording file")?;
        Ok(Some(RecordingFrame {
            timestamp: frame.timestamp,
            data,
        }))
    }

    pub fn first_timestamp(&self) -> Option<SystemTime> {
        self.frames.first().map(|frame| frame.timestamp)
    }

    pub fn last_timestamp(&self) -> Option<SystemTime> {
        self.frames.last().map(|frame| frame.timestamp)
    }
}

#[derive(Debug)]
struct RecordingFrameMetadata {
    timestamp: SystemTime,
    offset: usize,
    header_offset: usize,
    length: usize,
}

#[derive(Debug)]
pub struct RecordingFrame {
    pub timestamp: SystemTime,
    pub data: Vec<u8>,
}
