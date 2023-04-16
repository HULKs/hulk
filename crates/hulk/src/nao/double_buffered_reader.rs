use std::{
    io::{self, ErrorKind, Read},
    mem::{size_of, MaybeUninit},
    os::unix::{io::AsRawFd, prelude::RawFd},
    ptr::null_mut,
};

use libc::{fd_set, select, FD_SET, FD_ZERO};

use super::hula::StateStorage;

const NUMBER_OF_BUFFERS: usize = 2;

pub struct DoubleBufferedReader<Reader, Poller> {
    reader: Reader,
    poller: Poller,
    buffers: [[u8; size_of::<StateStorage>()]; NUMBER_OF_BUFFERS],
    active_buffer_index: usize,
    number_of_read_bytes_in_active_buffer: usize,
}

impl<Reader, Poller> DoubleBufferedReader<Reader, Poller>
where
    Reader: AsRawFd + Read,
    Poller: Poll,
{
    pub fn from_reader_and_poller(reader: Reader, poller: Poller) -> Self {
        Self {
            reader,
            poller,
            buffers: [[0; size_of::<StateStorage>()]; NUMBER_OF_BUFFERS],
            active_buffer_index: Default::default(),
            number_of_read_bytes_in_active_buffer: Default::default(),
        }
    }

    fn previous_active_buffer_index(&self) -> usize {
        (self.active_buffer_index + NUMBER_OF_BUFFERS - 1) % NUMBER_OF_BUFFERS
    }

    fn next_active_buffer_index(&self) -> usize {
        (self.active_buffer_index + 1) % NUMBER_OF_BUFFERS
    }

    fn activate_next_buffer(&mut self) {
        self.active_buffer_index = self.next_active_buffer_index();
        self.number_of_read_bytes_in_active_buffer = 0;
    }

    pub fn drain(&mut self) -> io::Result<&StateStorage> {
        let mut is_at_least_one_buffer_complete = false;
        loop {
            match self.reader.read(
                &mut self.buffers[self.active_buffer_index]
                    [self.number_of_read_bytes_in_active_buffer..],
            ) {
                Ok(number_of_read_bytes) => {
                    self.number_of_read_bytes_in_active_buffer += number_of_read_bytes;
                    assert!(
                        self.number_of_read_bytes_in_active_buffer
                            <= self.buffers[self.active_buffer_index].len()
                    );
                    let is_active_buffer_complete = self.number_of_read_bytes_in_active_buffer
                        == self.buffers[self.active_buffer_index].len();
                    if is_active_buffer_complete {
                        self.activate_next_buffer();
                        is_at_least_one_buffer_complete = true;
                    }
                }
                Err(ref error) if error.kind() == ErrorKind::WouldBlock => {
                    if is_at_least_one_buffer_complete {
                        return Ok(unsafe {
                            &*(self.buffers[self.previous_active_buffer_index()].as_ptr()
                                as *const StateStorage)
                        });
                    }
                    self.poller.poll(self.reader.as_raw_fd())?;
                }
                Err(error) => return Err(error),
            }
        }
    }
}

pub trait Poll {
    fn poll(&mut self, file_descriptor: RawFd) -> io::Result<()>;
}

pub struct SelectPoller;

impl Poll for SelectPoller {
    fn poll(&mut self, file_descriptor: RawFd) -> io::Result<()> {
        unsafe {
            let mut set = MaybeUninit::<fd_set>::uninit();
            FD_ZERO(set.as_mut_ptr());
            let mut set = set.assume_init();
            FD_SET(file_descriptor, &mut set);
            if select(
                file_descriptor + 1,
                &mut set,
                null_mut(),
                null_mut(),
                null_mut(),
            ) < 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        slice::from_raw_parts,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    use super::*;

    struct PanickingPoller;

    impl Poll for PanickingPoller {
        fn poll(&mut self, _file_descriptor: RawFd) -> io::Result<()> {
            panic!("should not be called");
        }
    }

    struct ErroringPoller;

    impl Poll for ErroringPoller {
        fn poll(&mut self, file_descriptor: RawFd) -> io::Result<()> {
            assert_eq!(file_descriptor, FIXED_FILE_DESCRIPTOR);
            Err(ErrorKind::ConnectionAborted.into())
        }
    }

    const FIXED_FILE_DESCRIPTOR: RawFd = 42;

    #[derive(Default)]
    struct CountingPoller {
        number_of_polls: Arc<AtomicUsize>,
    }

    impl Poll for CountingPoller {
        fn poll(&mut self, file_descriptor: RawFd) -> io::Result<()> {
            assert_eq!(file_descriptor, FIXED_FILE_DESCRIPTOR);
            self.number_of_polls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn read_error_is_returned() {
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

        let mut double_buffered_reader =
            DoubleBufferedReader::from_reader_and_poller(Reader, PanickingPoller);
        let result = double_buffered_reader.drain();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::ConnectionAborted);
    }

    #[test]
    fn poll_error_is_returned() {
        struct Reader;
        impl AsRawFd for Reader {
            fn as_raw_fd(&self) -> RawFd {
                FIXED_FILE_DESCRIPTOR
            }
        }
        impl Read for Reader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(ErrorKind::WouldBlock.into())
            }
        }

        let mut double_buffered_reader =
            DoubleBufferedReader::from_reader_and_poller(Reader, ErroringPoller);
        let result = double_buffered_reader.drain();
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
                    return Err(ErrorKind::WouldBlock.into());
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
        let mut double_buffered_reader = DoubleBufferedReader::from_reader_and_poller(
            Reader {
                data,
                returned: false,
            },
            PanickingPoller,
        );
        let result = double_buffered_reader.drain();
        assert!(result.is_ok());
        let read_data = result.unwrap();
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
                    None => Err(ErrorKind::WouldBlock.into()),
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
        let mut double_buffered_reader = DoubleBufferedReader::from_reader_and_poller(
            Reader {
                reversed_items: reversed_items.clone(),
            },
            PanickingPoller,
        );
        let result = double_buffered_reader.drain();
        assert!(result.is_ok());
        let read_data = result.unwrap();
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
                FIXED_FILE_DESCRIPTOR
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
                    None | Some(None) => Err(ErrorKind::WouldBlock.into()),
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
        let number_of_polls: Arc<AtomicUsize> = Default::default();
        let mut double_buffered_reader = DoubleBufferedReader::from_reader_and_poller(
            Reader { reversed_items },
            CountingPoller {
                number_of_polls: number_of_polls.clone(),
            },
        );
        let result = double_buffered_reader.drain();
        assert!(result.is_ok());
        assert_eq!(number_of_polls.load(Ordering::SeqCst), 1);
        let read_data = result.unwrap();
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
                FIXED_FILE_DESCRIPTOR
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
                    None | Some(None) => Err(ErrorKind::WouldBlock.into()),
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
        let number_of_polls: Arc<AtomicUsize> = Default::default();
        let mut double_buffered_reader = DoubleBufferedReader::from_reader_and_poller(
            Reader { reversed_items },
            CountingPoller {
                number_of_polls: number_of_polls.clone(),
            },
        );
        let result = double_buffered_reader.drain();
        assert!(result.is_ok());
        assert_eq!(number_of_polls.load(Ordering::SeqCst), 1);
        let read_data = result.unwrap();
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
                FIXED_FILE_DESCRIPTOR
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
                    None | Some(None) => Err(ErrorKind::WouldBlock.into()),
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
        let number_of_polls: Arc<AtomicUsize> = Default::default();
        let mut double_buffered_reader = DoubleBufferedReader::from_reader_and_poller(
            Reader { reversed_items },
            CountingPoller {
                number_of_polls: number_of_polls.clone(),
            },
        );
        let result = double_buffered_reader.drain();
        assert!(result.is_ok());
        assert_eq!(number_of_polls.load(Ordering::SeqCst), 1);
        let read_data = result.unwrap();
        assert_eq!(read_data, &returned_data);
    }
}
