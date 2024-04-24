use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
    time::{Duration, SystemTime},
};

use bincode::{deserialize_from, Error};
use color_eyre::eyre::WrapErr;

#[derive(Debug)]
pub struct RecordingIndex {
    file: File,
    frames: Vec<RecordingFrameMetadata>,
    scan_offset: u64,
}

impl RecordingIndex {
    pub fn read_from(recording_file: impl AsRef<Path>) -> color_eyre::Result<Self> {
        let file = File::open(&recording_file)
            .wrap_err_with(|| format!("failed to open {}", recording_file.as_ref().display()))?;
        Ok(Self {
            file,
            frames: Vec::new(),
            scan_offset: 0,
        })
    }

    pub fn collect_next_frame_metadata(&mut self) -> color_eyre::Result<Option<f32>> {
        self.file
            .seek(SeekFrom::End(0))
            .wrap_err("failed to seek to end of file")?;
        let file_length = self
            .file
            .stream_position()
            .wrap_err("failed to get stream position of end of file")?;
        self.file
            .seek(SeekFrom::Start(self.scan_offset))
            .wrap_err("failed to rewind file")?;

        if self.scan_offset >= file_length {
            return Ok(None);
        }
        let Some(timestamp) = end_of_file_error_as_option(deserialize_from(&mut self.file))
            .wrap_err("failed to deserialize timestamp")?
        else {
            eprintln!("unexpected end of file of recording file while deserializing timestamp");
            return Ok(None);
        };
        let Some(duration) = end_of_file_error_as_option(deserialize_from(&mut self.file))
            .wrap_err("failed to deserialize duration")?
        else {
            eprintln!("unexpected end of file of recording file while deserializing duration");
            return Ok(None);
        };
        let Some(length) = end_of_file_error_as_option(deserialize_from(&mut self.file))
            .wrap_err("failed to deserialize data length")?
        else {
            eprintln!("unexpected end of file of recording file while deserializing length");
            return Ok(None);
        };
        let header_length = self
            .file
            .stream_position()
            .wrap_err("failed to get stream position")?
            - self.scan_offset;
        self.file
            .seek(SeekFrom::Current(length as i64))
            .wrap_err("failed to seek to end of data")?;
        if self.scan_offset + header_length + length as u64 > file_length {
            eprintln!("unexpected end of file of recording file");
            return Ok(None);
        }

        self.scan_offset = self
            .file
            .stream_position()
            .wrap_err("failed to get stream position")?;
        self.file.rewind().wrap_err("failed to rewind file")?;

        self.frames.push(RecordingFrameMetadata {
            timing: Timing {
                timestamp,
                duration,
            },
            offset: self.scan_offset.try_into().unwrap(),
            header_offset: header_length.try_into().unwrap(),
            length,
        });
        Ok(Some(self.scan_offset as f32 / file_length as f32))
    }

    pub fn number_of_frames(&self) -> usize {
        self.frames.len()
    }

    pub fn find_latest_frame_up_to(
        &mut self,
        timestamp: SystemTime,
    ) -> color_eyre::Result<Option<RecordingFrame>> {
        let frame = match self
            .frames
            .iter()
            .rev()
            .find(|frame| frame.timing.timestamp <= timestamp)
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
            timing: frame.timing,
            data,
        }))
    }

    pub fn first_timing(&self) -> Option<Timing> {
        self.frames.first().map(|frame| frame.timing)
    }

    pub fn last_timing(&self) -> Option<Timing> {
        self.frames.last().map(|frame| frame.timing)
    }

    pub fn iter(&self) -> impl Iterator<Item = Timing> + '_ {
        self.frames.iter().map(|frame| frame.timing)
    }
}

#[derive(Debug)]
struct RecordingFrameMetadata {
    timing: Timing,
    offset: usize,
    header_offset: usize,
    length: usize,
}

#[derive(Debug)]
pub struct RecordingFrame {
    pub timing: Timing,
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct Timing {
    pub timestamp: SystemTime,
    pub duration: Duration,
}

fn end_of_file_error_as_option<T>(result: Result<T, Error>) -> Result<Option<T>, Error> {
    result.map(Some).or_else(|error| {
        if let bincode::ErrorKind::Io(ref error) = *error {
            if error.kind() == io::ErrorKind::UnexpectedEof {
                return Ok(None);
            }
        }
        Err(error)
    })
}
