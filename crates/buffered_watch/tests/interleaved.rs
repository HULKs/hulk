use buffered_watch::channel;

#[test]
fn read_while_write() {
    let (mut writer, mut reader) = channel(0);

    {
        let mut slot = writer.borrow_mut();
        *slot = 42;

        {
            let slot = reader.borrow_and_mark_as_seen();
            assert_eq!(*slot, 0);
        }

        *slot = 1337;
    }

    let slot = reader.borrow_and_mark_as_seen();
    assert_eq!(*slot, 1337);
}

#[test]
fn read_while_write_multiple_readers() {
    let (mut writer, mut reader) = channel(0);
    let mut reader2 = reader.clone();
    {
        let mut slot = writer.borrow_mut();
        *slot = 42;
        {
            let slot = reader.borrow_and_mark_as_seen();
            assert_eq!(*slot, 0);
        }
        {
            let slot = reader2.borrow_and_mark_as_seen();
            assert_eq!(*slot, 0);
        }
        *slot = 1337;
    }
    let slot = reader.borrow_and_mark_as_seen();
    assert_eq!(*slot, 1337);
    let slot = reader2.borrow_and_mark_as_seen();
    assert_eq!(*slot, 1337);
}

#[test]
fn write_while_reading() {
    let (mut writer, mut reader) = channel(0);
    {
        let slot = reader.borrow_and_mark_as_seen();
        assert_eq!(*slot, 0);
        {
            let mut slot = writer.borrow_mut();
            *slot = 42;
        }
        assert_eq!(*slot, 0);
    }
    let slot = reader.borrow_and_mark_as_seen();
    assert_eq!(*slot, 42);
}
