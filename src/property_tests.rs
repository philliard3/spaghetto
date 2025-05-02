use proptest_derive::Arbitrary;

use super::*;
use proptest::prelude::*;

// simple enum to allow pushes and pops in any order
#[derive(Debug, Clone, Copy, Arbitrary)]
enum DequeOps<T> {
    PushFront(T),
    PopFront,
    PushBack(T),
    PopBack,
}

proptest! {
    // Test that no combination of pushes and pops of i32 will cause the devec to panic
    #[test]
    fn test_push_pop(ref ops in proptest::collection::vec(any::<DequeOps<i32>>(), 0..100)) {
        let mut devec = DeVec::new();
        for &op in ops.iter() {
            match op {
                DequeOps::PushFront(item) => devec.push_front(item),
                DequeOps::PopFront => { let _ = devec.pop_front(); },
                DequeOps::PushBack(item) => devec.push_back(item),
                DequeOps::PopBack => { let _ = devec.pop_back(); },
            }
        }
    }

    // Test that no combination of pushes and pops of String will cause the devec to panic
    #[test]
    fn test_push_pop_string(ref ops in proptest::collection::vec(any::<DequeOps<String>>(), 0..100)) {
        let mut devec = DeVec::new();
        for op in ops.iter() {
            match op {
                DequeOps::PushFront(item) => devec.push_front(item),
                DequeOps::PopFront => { let _ = devec.pop_front(); },
                DequeOps::PushBack(item) => devec.push_back(item),
                DequeOps::PopBack => { let _ = devec.pop_back(); },
            }
        }
    }

    // Test that pushing i32s to the front and back maintains the correct order
    #[test]
    fn test_push_front_back_ops(ref ops in proptest::collection::vec(any::<DequeOps<i32>>(), 0..100)) {
        let mut devec = DeVec::new();
        let mut model_deque = std::collections::VecDeque::new();
        for &op in ops.iter() {
            match op {
                DequeOps::PushFront(item) => devec.push_front(item),
                DequeOps::PopFront => { let _ = devec.pop_front(); },
                DequeOps::PushBack(item) => devec.push_back(item),
                DequeOps::PopBack => { let _ = devec.pop_back(); },
            }

            match op {
                DequeOps::PushFront(item) => model_deque.push_front(item),
                DequeOps::PopFront => { let _ = model_deque.pop_front(); },
                DequeOps::PushBack(item) => model_deque.push_back(item),
                DequeOps::PopBack => { let _ = model_deque.pop_back(); },
            }
        }
        let model_slice = model_deque.make_contiguous();
        prop_assert_eq!(&*devec, model_slice);
    }

    // Test that pushing strings to the front and back maintains the correct order
    #[test]
    fn test_push_front_back_ops_string(ref ops in proptest::collection::vec(any::<DequeOps<String>>(), 0..100)) {
        let mut devec = DeVec::new();
        let mut model_deque = std::collections::VecDeque::new();
        for op in ops.iter() {
            match op {
                DequeOps::PushFront(item) => devec.push_front(item.clone()),
                DequeOps::PopFront => { let _ = devec.pop_front(); },
                DequeOps::PushBack(item) => devec.push_back(item.clone()),
                DequeOps::PopBack => { let _ = devec.pop_back(); },
            }

            match op {
                DequeOps::PushFront(item) => model_deque.push_front(item.clone()),
                DequeOps::PopFront => { let _ = model_deque.pop_front(); },
                DequeOps::PushBack(item) => model_deque.push_back(item.clone()),
                DequeOps::PopBack => { let _ = model_deque.pop_back(); },
            }
        }
        let model_slice = model_deque.make_contiguous();
        prop_assert_eq!(&*devec, model_slice);
    }
}

// TODO: test to make sure size hint is always accurate. record each size hint as an iterator produces values. Then check it against the values that come later. This requires either updating each one over and over or waiting until we get all the items at the end and then going backwards and seeing how many messed up.
