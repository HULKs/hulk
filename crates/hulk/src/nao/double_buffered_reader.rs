use std::{
    io::{self, ErrorKind, Read},
    mem::size_of,
    os::unix::{io::AsRawFd, prelude::RawFd},
};

use super::hula::StateStorage;

pub struct DoubleBufferedReader {
    buffers: [[u8; size_of::<StateStorage>()]; 2],
    active_buffer_index: usize,
    incomplete_offset: usize,
}

impl Default for DoubleBufferedReader {
    fn default() -> Self {
        Self {
            buffers: [[0; size_of::<StateStorage>()]; 2],
            active_buffer_index: Default::default(),
            incomplete_offset: Default::default(),
        }
    }
}

impl DoubleBufferedReader {
    fn swap_buffers(&mut self) {
        self.incomplete_offset = 0;
        self.active_buffer_index = (self.active_buffer_index + 1) % 2;
    }

    pub fn read(
        &mut self,
        reader: &mut (impl AsRawFd + Read),
        mut poll_reader: impl FnMut(RawFd) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut buffers_swapped = false;
        loop {
            match reader.read(&mut self.buffers[self.active_buffer_index][self.incomplete_offset..])
            {
                Ok(number_of_read_bytes) => {
                    self.incomplete_offset += number_of_read_bytes;
                    assert!(self.incomplete_offset <= self.buffers[self.active_buffer_index].len());
                    if self.incomplete_offset == self.buffers[self.active_buffer_index].len() {
                        self.swap_buffers();
                        buffers_swapped = true;
                    }
                }
                Err(ref error) if error.kind() == ErrorKind::Interrupted => {
                    if buffers_swapped {
                        return Ok(());
                    }
                    poll_reader(reader.as_raw_fd())?;
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// Precondition: `read()` has been called once
    pub fn get_last(&self) -> &StateStorage {
        let inactive_buffer_index = (self.active_buffer_index + 1) % 2;
        unsafe { &*(self.buffers[inactive_buffer_index].as_ptr() as *const StateStorage) }
    }
}

#[cfg(test)]
mod tests {
    use std::slice::from_raw_parts;

    use super::*;

    #[test]
    fn error_is_returned() {
        struct Reader;
        impl AsRawFd for Reader {
            fn as_raw_fd(&self) -> RawFd {
                panic!("should not be called");
            }
        }
        impl Read for Reader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(ErrorKind::ConnectionAborted.into())
            }
        }

        let mut double_buffered_reader = DoubleBufferedReader::default();
        let result = double_buffered_reader.read(&mut Reader, |_file_descriptor| {
            panic!("should not be called");
        });
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::ConnectionAborted);
    }

    #[test]
    fn complete_read_terminates() {
        struct Reader {
            data: StateStorage,
            returned: bool,
        }
        impl AsRawFd for Reader {
            fn as_raw_fd(&self) -> RawFd {
                panic!("should not be called");
            }
        }
        impl Read for Reader {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                if self.returned {
                    return Err(ErrorKind::Interrupted.into());
                }
                assert_eq!(buffer.len(), size_of::<StateStorage>());
                let data_slice = unsafe {
                    from_raw_parts(
                        &self.data as *const StateStorage as *const u8,
                        size_of::<StateStorage>(),
                    )
                };
                buffer.copy_from_slice(data_slice);
                self.returned = true;
                Ok(size_of::<StateStorage>())
            }
        }

        let data = StateStorage {
            received_at: 42.1337,
            ..Default::default()
        };
        let mut double_buffered_reader = DoubleBufferedReader::default();
        let result = double_buffered_reader.read(
            &mut Reader {
                data,
                returned: false,
            },
            |_file_descriptor| {
                panic!("should not be called");
            },
        );
        assert!(result.is_ok());
        let read_data = double_buffered_reader.get_last();
        assert_eq!(read_data, &data);
    }

    #[test]
    fn two_complete_reads_terminate_and_return_latest() {
        struct Reader {
            reversed_items: Vec<StateStorage>,
        }
        impl AsRawFd for Reader {
            fn as_raw_fd(&self) -> RawFd {
                panic!("should not be called");
            }
        }
        impl Read for Reader {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                match self.reversed_items.pop() {
                    Some(item) => {
                        assert_eq!(buffer.len(), size_of::<StateStorage>());
                        let data_slice = unsafe {
                            from_raw_parts(
                                &item as *const StateStorage as *const u8,
                                size_of::<StateStorage>(),
                            )
                        };
                        buffer.copy_from_slice(data_slice);
                        Ok(size_of::<StateStorage>())
                    }
                    None => Err(ErrorKind::Interrupted.into()),
                }
            }
        }

        let reversed_items = vec![
            StateStorage {
                received_at: 42.1337,
                ..Default::default()
            },
            StateStorage {
                received_at: 1337.42,
                ..Default::default()
            },
        ];
        let mut double_buffered_reader = DoubleBufferedReader::default();
        let result = double_buffered_reader.read(
            &mut Reader {
                reversed_items: reversed_items.clone(),
            },
            |_file_descriptor| {
                panic!("should not be called");
            },
        );
        assert!(result.is_ok());
        let read_data = double_buffered_reader.get_last();
        assert_eq!(read_data, reversed_items.first().unwrap());
    }

    #[test]
    fn two_partial_reads_terminate() {
        struct Item<'buffer> {
            buffer: &'buffer [u8],
            expected_buffer_size: usize,
        }
        struct Reader<'buffer> {
            reversed_items: Vec<Option<Item<'buffer>>>,
        }
        impl<'buffer> AsRawFd for Reader<'buffer> {
            fn as_raw_fd(&self) -> RawFd {
                42
            }
        }
        impl<'buffer> Read for Reader<'buffer> {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                match self.reversed_items.pop() {
                    Some(Some(item)) => {
                        assert_eq!(buffer.len(), item.expected_buffer_size);
                        buffer[..item.buffer.len()].copy_from_slice(item.buffer);
                        Ok(item.buffer.len())
                    }
                    None | Some(None) => Err(ErrorKind::Interrupted.into()),
                }
            }
        }

        let data = StateStorage {
            received_at: 42.1337,
            ..Default::default()
        };
        let reversed_items = vec![
            Some(Item {
                buffer: unsafe {
                    from_raw_parts(
                        (&data as *const StateStorage as *const u8).add(100),
                        size_of::<StateStorage>() - 100,
                    )
                },
                expected_buffer_size: size_of::<StateStorage>() - 100,
            }),
            None,
            Some(Item {
                buffer: unsafe { from_raw_parts(&data as *const StateStorage as *const u8, 100) },
                expected_buffer_size: size_of::<StateStorage>(),
            }),
        ];
        let mut double_buffered_reader = DoubleBufferedReader::default();
        let mut number_of_polls = 0;
        let result =
            double_buffered_reader.read(&mut Reader { reversed_items }, |file_descriptor| {
                assert_eq!(file_descriptor, 42);
                number_of_polls += 1;
                Ok(())
            });
        assert!(result.is_ok());
        assert_eq!(number_of_polls, 1);
        let read_data = double_buffered_reader.get_last();
        assert_eq!(read_data, &data);
    }

    #[test]
    fn four_partial_reads_terminate_and_return_previous_complete() {
        struct Item<'buffer> {
            buffer: &'buffer [u8],
            expected_buffer_size: usize,
        }
        struct Reader<'buffer> {
            reversed_items: Vec<Option<Item<'buffer>>>,
        }
        impl<'buffer> AsRawFd for Reader<'buffer> {
            fn as_raw_fd(&self) -> RawFd {
                42
            }
        }
        impl<'buffer> Read for Reader<'buffer> {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                match self.reversed_items.pop() {
                    Some(Some(item)) => {
                        assert_eq!(buffer.len(), item.expected_buffer_size);
                        buffer[..item.buffer.len()].copy_from_slice(item.buffer);
                        Ok(item.buffer.len())
                    }
                    None | Some(None) => Err(ErrorKind::Interrupted.into()),
                }
            }
        }

        let returned_data = StateStorage {
            received_at: 42.1337,
            ..Default::default()
        };
        let incomplete_data = StateStorage {
            received_at: 1337.42,
            ..Default::default()
        };
        let reversed_items = vec![
            Some(Item {
                buffer: unsafe {
                    from_raw_parts(&incomplete_data as *const StateStorage as *const u8, 100)
                },
                expected_buffer_size: size_of::<StateStorage>(),
            }),
            Some(Item {
                buffer: unsafe {
                    from_raw_parts(
                        (&returned_data as *const StateStorage as *const u8).add(100),
                        size_of::<StateStorage>() - 100,
                    )
                },
                expected_buffer_size: size_of::<StateStorage>() - 100,
            }),
            None,
            Some(Item {
                buffer: unsafe {
                    from_raw_parts(&returned_data as *const StateStorage as *const u8, 100)
                },
                expected_buffer_size: size_of::<StateStorage>(),
            }),
        ];
        let mut double_buffered_reader = DoubleBufferedReader::default();
        let mut number_of_polls = 0;
        let result =
            double_buffered_reader.read(&mut Reader { reversed_items }, |file_descriptor| {
                assert_eq!(file_descriptor, 42);
                number_of_polls += 1;
                Ok(())
            });
        assert!(result.is_ok());
        assert_eq!(number_of_polls, 1);
        let read_data = double_buffered_reader.get_last();
        assert_eq!(read_data, &returned_data);
    }

    #[test]
    fn four_partial_reads_terminate_and_return_latest() {
        struct Item<'buffer> {
            buffer: &'buffer [u8],
            expected_buffer_size: usize,
        }
        struct Reader<'buffer> {
            reversed_items: Vec<Option<Item<'buffer>>>,
        }
        impl<'buffer> AsRawFd for Reader<'buffer> {
            fn as_raw_fd(&self) -> RawFd {
                42
            }
        }
        impl<'buffer> Read for Reader<'buffer> {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                match self.reversed_items.pop() {
                    Some(Some(item)) => {
                        assert_eq!(buffer.len(), item.expected_buffer_size);
                        buffer[..item.buffer.len()].copy_from_slice(item.buffer);
                        Ok(item.buffer.len())
                    }
                    None | Some(None) => Err(ErrorKind::Interrupted.into()),
                }
            }
        }

        let returned_data = StateStorage {
            received_at: 42.1337,
            ..Default::default()
        };
        let unused_data = StateStorage {
            received_at: 1337.42,
            ..Default::default()
        };
        let reversed_items = vec![
            Some(Item {
                buffer: unsafe {
                    from_raw_parts(
                        (&returned_data as *const StateStorage as *const u8).add(100),
                        size_of::<StateStorage>() - 100,
                    )
                },
                expected_buffer_size: size_of::<StateStorage>() - 100,
            }),
            Some(Item {
                buffer: unsafe {
                    from_raw_parts(&returned_data as *const StateStorage as *const u8, 100)
                },
                expected_buffer_size: size_of::<StateStorage>(),
            }),
            Some(Item {
                buffer: unsafe {
                    from_raw_parts(
                        (&unused_data as *const StateStorage as *const u8).add(100),
                        size_of::<StateStorage>() - 100,
                    )
                },
                expected_buffer_size: size_of::<StateStorage>() - 100,
            }),
            None,
            Some(Item {
                buffer: unsafe {
                    from_raw_parts(&unused_data as *const StateStorage as *const u8, 100)
                },
                expected_buffer_size: size_of::<StateStorage>(),
            }),
        ];
        let mut double_buffered_reader = DoubleBufferedReader::default();
        let mut number_of_polls = 0;
        let result =
            double_buffered_reader.read(&mut Reader { reversed_items }, |file_descriptor| {
                assert_eq!(file_descriptor, 42);
                number_of_polls += 1;
                Ok(())
            });
        assert!(result.is_ok());
        assert_eq!(number_of_polls, 1);
        let read_data = double_buffered_reader.get_last();
        assert_eq!(read_data, &returned_data);
    }
}
