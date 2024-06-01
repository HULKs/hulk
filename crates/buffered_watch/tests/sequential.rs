use buffered_watch::channel;

#[test]
fn read_and_write() {
    let (mut writer, mut reader) = channel(0);
    {
        let mut slot = writer.borrow_mut();
        *slot = 42;
    }
    {
        let slot = reader.borrow_and_mark_as_seen();
        assert_eq!(*slot, 42);
    }
    {
        let mut slot = writer.borrow_mut();
        *slot = 1337;
    }
    {
        let slot = reader.borrow_and_mark_as_seen();
        assert_eq!(*slot, 1337);
    }
}

#[test]
fn multiple_readers() {
    let (mut writer, mut reader) = channel(0);
    let mut reader2 = reader.clone();
    {
        let mut slot = writer.borrow_mut();
        *slot = 42;
    }
    {
        let slot = reader.borrow_and_mark_as_seen();
        assert_eq!(*slot, 42);
    }
    {
        let slot = reader2.borrow_and_mark_as_seen();
        assert_eq!(*slot, 42);
    }
}

#[test]
fn subsequent_reads() {
    let (mut writer, mut reader) = channel(0);
    {
        let mut slot = writer.borrow_mut();
        *slot = 42;
    }
    {
        let slot = reader.borrow_and_mark_as_seen();
        assert_eq!(*slot, 42);
    }
    {
        let slot = reader.borrow_and_mark_as_seen();
        assert_eq!(*slot, 42);
    }
}

#[test]
fn dynamic_number_of_readers() {
    let (mut writer, mut reader) = channel(0);
    {
        let mut slot = writer.borrow_mut();
        *slot = 42;
    }
    {
        let slot = reader.borrow_and_mark_as_seen();
        assert_eq!(*slot, 42);
    }
    let mut reader2 = reader.clone();
    {
        let slot = reader2.borrow_and_mark_as_seen();
        assert_eq!(*slot, 42);
    }
}
