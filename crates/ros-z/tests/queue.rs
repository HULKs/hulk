use std::{sync::Arc, thread, time::Duration};

use ros_z::queue::BoundedQueue;

#[test]
fn test_push_pop_basic() {
    let q = BoundedQueue::new(5);
    assert_eq!(q.push(1), None);
    assert_eq!(q.push(2), None);
    assert_eq!(q.try_recv(), Some(1));
    assert_eq!(q.try_recv(), Some(2));
    assert_eq!(q.try_recv(), None);
}

#[test]
fn test_drop_oldest_on_overflow() {
    let q = BoundedQueue::new(3);
    assert_eq!(q.push(1), None);
    assert_eq!(q.push(2), None);
    assert_eq!(q.push(3), None);
    assert_eq!(q.push(4), Some(1));
    assert_eq!(q.push(5), Some(2));
    assert_eq!(q.try_recv(), Some(3));
    assert_eq!(q.try_recv(), Some(4));
    assert_eq!(q.try_recv(), Some(5));
    assert_eq!(q.try_recv(), None);
}

#[test]
fn test_recv_timeout_expires() {
    let q: BoundedQueue<i32> = BoundedQueue::new(5);
    let start = std::time::Instant::now();
    assert!(q.receive_with_timeout(Duration::from_millis(50)).is_none());
    assert!(start.elapsed() >= Duration::from_millis(50));
}

#[test]
fn test_recv_timeout_succeeds() {
    let q = Arc::new(BoundedQueue::new(5));
    let q2 = q.clone();

    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        let _ = q2.push(42);
    });

    let result = q.receive_with_timeout(Duration::from_millis(100));
    assert_eq!(result, Some(42));
    handle.join().unwrap();
}

#[test]
fn test_blocking_recv() {
    let q = Arc::new(BoundedQueue::new(5));
    let q2 = q.clone();

    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        let _ = q2.push(42);
    });

    let val = q.recv();
    assert_eq!(val, 42);
    handle.join().unwrap();
}

#[tokio::test]
async fn test_recv_async() {
    let q = Arc::new(BoundedQueue::new(5));
    let q2 = q.clone();

    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        let _ = q2.push(42);
    });

    let val = q.recv_async().await;
    assert_eq!(val, 42);
    handle.await.unwrap();
}

#[test]
fn test_capacity_one() {
    let q = BoundedQueue::new(1);
    assert_eq!(q.push(1), None);
    assert_eq!(q.push(2), Some(1));
    assert_eq!(q.push(3), Some(2));
    assert_eq!(q.try_recv(), Some(3));
}
