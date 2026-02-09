use std::{
    cell::{Cell, RefCell, RefMut},
    cmp::Ordering,
    collections::VecDeque,
    mem::take,
    ops::{Deref, DerefMut},
};

use arrayvec::ArrayVec;
use replace_with::replace_with_or_abort;

const B: usize = 150;

pub struct Map<K, V> {
    root: RefCell<Option<Node<K, V>>>,
}

impl<K, V> Map<K, V> {
    pub fn new() -> Self {
        Map {
            root: RefCell::new(Some(Node {
                buffer: Default::default(),
                buffer_is_sorted: Cell::new(true),
                array: Array::Leaf(LeafArray {
                    elements: Default::default(),
                }),
            })),
        }
    }
}

impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord, V> Map<K, V> {
    pub fn insert(&mut self, key: K, value: V) {
        self.root.borrow_mut().as_mut().unwrap().insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let mut root_borrow = self.root.borrow_mut();
        let root = root_borrow.as_mut().unwrap();
        let (mut search_result, mut new_branches) = root.search(key);

        drop(root_borrow);
        while !new_branches.is_empty() {
            let new_array = InternalArray {
                first_child: Box::new(self.root.take().unwrap()),
                elements: Default::default(),
            };
            new_branches = new_array.process_branches(new_branches.into_iter());

            search_result = match search_result {
                SearchResult::HeadOfBranch => {
                    let search_new_array = |array: &mut ArrayVec<Branch<K, V>, B>| {
                        let idx = array.binary_search_by(|b| b.key.cmp(key)).unwrap();
                        &mut *array[idx].value as *mut _
                    };
                    match new_branches.binary_search_by(|b| b.key.cmp(key)) {
                        Ok(_) => SearchResult::HeadOfBranch,
                        Err(0) => SearchResult::Some(search_new_array(
                            &mut new_array.elements.borrow_mut(),
                        )),
                        Err(i) => SearchResult::Some(search_new_array(
                            &mut new_branches[i - 1]
                                .child
                                .array
                                .unwrap_internal_mut()
                                .elements
                                .borrow_mut(),
                        )),
                    }
                }
                sr => sr,
            };

            *self.root.borrow_mut() = Some(Node {
                buffer: Default::default(),
                buffer_is_sorted: Cell::new(true),
                array: Array::Internal(new_array),
            });
        }

        match search_result {
            SearchResult::HeadOfBranch => unreachable!(),
            SearchResult::Some(p) => Some(unsafe { &*p }),
            SearchResult::None => None,
        }
    }
}

struct Node<K, V> {
    buffer: RefCell<VecDeque<(K, V)>>,
    buffer_is_sorted: Cell<bool>,
    array: Array<K, V>,
}

enum Array<K, V> {
    Internal(InternalArray<K, V>),
    Leaf(LeafArray<K, V>),
}

impl<K, V> Array<K, V> {
    fn unwrap_internal_mut(&mut self) -> &mut InternalArray<K, V> {
        match self {
            Array::Internal(ia) => ia,
            _ => unreachable!(),
        }
    }
}

struct InternalArray<K, V> {
    first_child: Box<Node<K, V>>,
    elements: RefCell<Box<ArrayVec<Branch<K, V>, B>>>,
}

struct LeafArray<K, V> {
    elements: RefCell<Box<ArrayVec<(K, V), B>>>,
}

struct Branch<K, V> {
    key: K,
    value: MaybeBox<V>,
    child: Box<Node<K, V>>,
}

enum MaybeBox<V> {
    Inline(V),
    Boxed(Box<V>),
}

impl<V> MaybeBox<V> {
    fn boxify(&mut self) {
        replace_with_or_abort(self, |this| match this {
            MaybeBox::Inline(v) => MaybeBox::Boxed(Box::new(v)),
            mb => mb,
        });
    }
}

impl<V> Deref for MaybeBox<V> {
    type Target = V;

    fn deref(&self) -> &V {
        match self {
            MaybeBox::Boxed(b) => b,
            MaybeBox::Inline(v) => v,
        }
    }
}

impl<V> DerefMut for MaybeBox<V> {
    fn deref_mut(&mut self) -> &mut V {
        match self {
            MaybeBox::Boxed(b) => &mut *b,
            MaybeBox::Inline(v) => v,
        }
    }
}

impl<K: Ord, V> Node<K, V> {
    fn insert(&mut self, key: K, value: V) {
        let mut buffer = self.buffer.borrow_mut();
        if self.buffer_is_sorted.get()
            && let Some((front, _)) = buffer.front()
        {
            if front >= &key {
                buffer.push_front((key, value));
                return;
            } else {
                let (back, _) = buffer.back().unwrap();
                if back > &key {
                    self.buffer_is_sorted.set(false);
                }
            }
        }
        buffer.push_back((key, value));
    }

    fn append(&self, thief: SliceThief<(K, V)>, is_sorted: bool) {
        let mut buffer = self.buffer.borrow_mut();
        buffer.reserve(thief.len);
        if is_sorted
            && self.buffer_is_sorted.get()
            && let Some((front, _)) = buffer.front()
        {
            let last = thief.peek_last();
            if front >= &last.0 {
                for item in thief.rev() {
                    buffer.push_front(item);
                }
            } else {
                let (back, _) = buffer.back().unwrap();
                let first = thief.peek_first();
                if back > &first.0 {
                    self.buffer_is_sorted.set(false);
                }
                buffer.extend(thief);
            }
        } else {
            self.buffer_is_sorted
                .set(is_sorted && self.buffer_is_sorted.get());
            buffer.extend(thief);
        }
    }

    fn search(&self, key: &K) -> (SearchResult<V>, Vec<Branch<K, V>>) {
        match &self.array {
            Array::Internal(ia) => {
                let mut buffer = self.buffer.borrow_mut();
                if !buffer.is_empty() {
                    replace_with_or_abort(&mut *buffer, |buffer| {
                        let mut vec = Vec::from(buffer);
                        if !self.buffer_is_sorted.get() {
                            vec.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
                        }
                        ia.push_down(&mut vec);
                        VecDeque::from(vec)
                    });
                }

                let mut elements = ia.elements.borrow_mut();

                match elements.binary_search_by(|branch| branch.key.cmp(key)) {
                    Ok(i) => {
                        let value = &mut elements[i].value;
                        value.boxify();
                        (SearchResult::Some(&**value), vec![])
                    }
                    Err(i) => {
                        let (search_result, mut new_branches) = if i == 0 {
                            ia.first_child.search(key)
                        } else {
                            elements[i - 1].child.search(key)
                        };
                        drop(elements);
                        if !new_branches.is_empty() {
                            new_branches = ia.process_branches(new_branches.into_iter());
                        }

                        match search_result {
                            SearchResult::HeadOfBranch => {
                                match new_branches.binary_search_by(|b| b.key.cmp(key)) {
                                    Ok(_) => (SearchResult::HeadOfBranch, new_branches),
                                    Err(0) => {
                                        let elements = ia.elements.borrow();
                                        match elements.binary_search_by(|b| b.key.cmp(key)) {
                                            Ok(i) => (
                                                SearchResult::Some(&*elements[i].value),
                                                new_branches,
                                            ),
                                            _ => unreachable!(),
                                        }
                                    }
                                    Err(i) => {
                                        let elements = match &new_branches[i - 1].child.array {
                                            Array::Internal(ia) => ia.elements.borrow(),
                                            _ => unreachable!(),
                                        };
                                        let search_result =
                                            match elements.binary_search_by(|b| b.key.cmp(key)) {
                                                Ok(i) => SearchResult::Some(&*elements[i].value),
                                                _ => unreachable!(),
                                            };
                                        drop(elements);
                                        (search_result, new_branches)
                                    }
                                }
                            }
                            sr => (sr, new_branches),
                        }
                    }
                }
            }
            Array::Leaf(la) => {
                let mut buffer = self.buffer.borrow_mut();

                if !buffer.is_empty() {
                    if !self.buffer_is_sorted.get() {
                        buffer
                            .make_contiguous()
                            .sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
                    }
                    let new_branches = la.process_buffer(buffer.drain(..));
                    match new_branches.binary_search_by(|b| b.key.cmp(key)) {
                        Ok(_) => (SearchResult::HeadOfBranch, new_branches),
                        Err(0) => {
                            let elements = la.elements.borrow();
                            match elements.binary_search_by(|(k, _)| k.cmp(key)) {
                                Ok(i) => (SearchResult::Some(&elements[i].1), new_branches),
                                Err(_) => (SearchResult::None, new_branches),
                            }
                        }
                        Err(i) => {
                            let elements = match &new_branches[i - 1].child.array {
                                Array::Leaf(la) => la.elements.borrow(),
                                _ => unreachable!(),
                            };
                            let search_result = match elements.binary_search_by(|(k, _)| k.cmp(key))
                            {
                                Ok(i) => SearchResult::Some(&elements[i].1),
                                Err(_) => SearchResult::None,
                            };
                            drop(elements);
                            (search_result, new_branches)
                        }
                    }
                } else {
                    let elements = la.elements.borrow();
                    (
                        elements
                            .binary_search_by(|(k, _)| k.cmp(key))
                            .ok()
                            .map(|i| SearchResult::Some(&elements[i].1 as *const _))
                            .unwrap_or(SearchResult::None),
                        vec![],
                    )
                }
            }
        }
    }
}

enum SearchResult<V> {
    Some(*const V),
    HeadOfBranch,
    None,
}

struct VecSlicer<'a, T> {
    slice_start: usize,
    current_index: usize,
    vec: &'a mut Vec<T>,
}

impl<'a, T> VecSlicer<'a, T> {
    fn new(vec: &'a mut Vec<T>) -> Self {
        Self {
            slice_start: 0,
            current_index: 0,
            vec,
        }
    }

    fn advance(&mut self, count: usize) {
        self.current_index += count;
    }

    fn slice(&mut self) -> SliceThief<T> {
        let thief = SliceThief {
            start: &self.vec[self.slice_start],
            current: 0,
            len: self.current_index - self.slice_start,
        };
        self.slice_start = self.current_index;
        thief
    }

    fn take(&mut self) -> T {
        debug_assert_eq!(self.slice_start, self.current_index);
        let result = unsafe { (&self.vec[self.current_index] as *const T).read() };
        self.slice_start += 1;
        self.current_index = self.slice_start;
        result
    }

    fn remaining(&self) -> usize {
        self.vec.len() - self.current_index
    }

    fn current(&self) -> &T {
        &self.vec[self.current_index]
    }

    fn slice_to_end(&mut self) -> SliceThief<T> {
        self.current_index = self.vec.len();
        self.slice()
    }
}

impl<T> Drop for VecSlicer<'_, T> {
    fn drop(&mut self) {
        debug_assert_eq!(self.current_index, self.vec.len());
        debug_assert_eq!(self.slice_start, self.vec.len());
        unsafe {
            self.vec.set_len(0);
        }
    }
}

struct SliceThief<T> {
    start: *const T,
    current: usize,
    len: usize,
}

impl<T> SliceThief<T> {
    fn peek_first(&self) -> &T {
        debug_assert_ne!(self.len, 0, "Cannot peek first element of empty slice");
        debug_assert_eq!(
            self.current, 0,
            "Cannot peek first element when it has already been consumed."
        );
        unsafe { &*self.start }
    }

    fn peek_last(&self) -> &T {
        debug_assert_ne!(self.len, 0, "Cannot peek last element of empty slice");
        debug_assert_ne!(
            self.current, self.len,
            "Cannot peek last element when it has already been consumed."
        );
        unsafe { &*self.start.add(self.len - 1) }
    }
}

impl<T> Drop for SliceThief<T> {
    fn drop(&mut self) {
        debug_assert_eq!(self.current, self.len);
    }
}

impl<T> Iterator for SliceThief<T> {
    type Item = T;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.len {
            let result = unsafe { Some(self.start.add(self.current).read()) };
            self.current += 1;
            result
        } else {
            None
        }
    }
}

impl<T> ExactSizeIterator for SliceThief<T> {}

impl<T> DoubleEndedIterator for SliceThief<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current < self.len {
            self.len -= 1;
            unsafe { Some(self.start.add(self.len).read()) }
        } else {
            None
        }
    }
}

impl<K: Ord, V> InternalArray<K, V> {
    fn push_down(&self, buffer: &mut Vec<(K, V)>) {
        let mut elements = self.elements.borrow_mut();
        let mut elements_iter = elements.iter_mut();

        let mut slicer = VecSlicer::new(buffer);

        let mut push_to = &self.first_child;

        if let Some(mut active_element) = elements_iter.next() {
            loop {
                let next_insert = slicer.current();
                if next_insert.0 < active_element.key {
                    slicer.advance(1);
                    if slicer.remaining() == 0 {
                        break;
                    }
                } else if next_insert.0 == active_element.key {
                    let slice = slicer.slice();
                    if slice.len() != 0 {
                        push_to.append(slice, true);
                    }
                    let next_insert = slicer.take();
                    active_element.key = next_insert.0;
                    active_element.value = MaybeBox::Inline(next_insert.1);
                    if slicer.remaining() == 0 {
                        break;
                    }
                } else {
                    let slice = slicer.slice();
                    if slice.len() != 0 {
                        push_to.append(slice, true);
                    }
                    push_to = &active_element.child;
                    if let Some(ae) = elements_iter.next() {
                        active_element = ae;
                    } else {
                        break;
                    }
                }
            }
        }

        let last_slice = slicer.slice_to_end();
        if last_slice.len() != 0 {
            push_to.append(last_slice, true);
        }
    }

    fn process_branches(
        &self,
        branches: impl ExactSizeIterator<Item = Branch<K, V>>,
    ) -> Vec<Branch<K, V>> {
        process_buffer(
            self.elements.borrow_mut(),
            branches,
            |branch| {
                let child = Box::new(Node {
                    buffer: Default::default(),
                    buffer_is_sorted: Cell::new(true),
                    array: Array::Internal(InternalArray {
                        first_child: branch.child,
                        elements: Default::default(),
                    }),
                });
                let push_to = match &child.array {
                    Array::Internal(ia) => {
                        let mut elem_borrow = ia.elements.borrow_mut();
                        let ptr = &mut **elem_borrow as *mut _;
                        drop(elem_borrow);
                        ptr
                    }
                    _ => unreachable!(),
                };
                (Branch { child, ..branch }, push_to)
            },
            |b1, b2| b1.key.cmp(&b2.key),
        )
    }
}

impl<K: Ord, V> LeafArray<K, V> {
    fn process_buffer(&self, buffer: impl ExactSizeIterator<Item = (K, V)>) -> Vec<Branch<K, V>> {
        process_buffer(
            self.elements.borrow_mut(),
            buffer,
            |(key, value)| {
                let child = Box::new(Node {
                    buffer: Default::default(),
                    buffer_is_sorted: Cell::new(true),
                    array: Array::Leaf(LeafArray {
                        elements: Default::default(),
                    }),
                });
                let push_to = match &child.array {
                    Array::Leaf(la) => {
                        let mut elem_borrow = la.elements.borrow_mut();
                        let ptr = &mut **elem_borrow as *mut _;
                        drop(elem_borrow);
                        ptr
                    }
                    _ => unreachable!(),
                };
                (
                    Branch {
                        key,
                        value: MaybeBox::Inline(value),
                        child,
                    },
                    push_to,
                )
            },
            |(k1, _), (k2, _)| k1.cmp(k2),
        )
    }
}

#[allow(clippy::type_complexity)]
fn process_buffer<I, K, V>(
    mut elements_ref: RefMut<Box<ArrayVec<I, B>>>,
    buffer: impl ExactSizeIterator<Item = I>,
    branch_builder: fn(I) -> (Branch<K, V>, *mut ArrayVec<I, B>),
    item_comparator: fn(&I, &I) -> Ordering,
) -> Vec<Branch<K, V>> {
    let total_count = buffer.len() + elements_ref.len();

    if total_count <= B && buffer.len() <= 2 {
        for item in buffer {
            match elements_ref.binary_search_by(|i| item_comparator(i, &item)) {
                Ok(i) => elements_ref[i] = item,
                Err(i) => elements_ref.insert(i, item),
            }
        }
        vec![]
    } else {
        let mut elements_vec = take(&mut *elements_ref);
        let mut elements = elements_vec.drain(..);
        let mut buffer = buffer.peekable();

        let mut next_element = elements.next();
        let mut next_insert = buffer.next();

        let mut counter = 0;

        let mut result = vec![];
        let mut push_to = &mut **elements_ref as *mut ArrayVec<I, B>;
        let mut apply = |item| {
            if (counter + 1) % (B / 2 + 1) == 0 && total_count - counter > B / 2 {
                let (new_branch, new_push_to) = branch_builder(item);
                push_to = new_push_to;
                result.push(new_branch);
            } else {
                unsafe { &mut *push_to }.push(item)
            }

            counter += 1;
        };

        while let Some(ne) = &next_element
            && let Some(ni) = &next_insert
        {
            match item_comparator(ne, ni) {
                Ordering::Less => {
                    apply(next_element.take().unwrap());
                    next_element = elements.next();
                }
                Ordering::Greater => {
                    let mut to_apply = next_insert.take().unwrap();
                    while let Some(peek) = buffer.peek()
                        && item_comparator(&to_apply, peek).is_eq()
                    {
                        to_apply = buffer.next().unwrap();
                    }
                    apply(to_apply);
                    next_insert = buffer.next();
                }
                Ordering::Equal => {
                    next_element = elements.next();
                }
            }
        }

        while let Some(ne) = next_element {
            apply(ne);
            next_element = elements.next();
        }

        while let Some(ni) = next_insert {
            let mut to_apply = ni;
            while let Some(peek) = buffer.peek()
                && item_comparator(&to_apply, peek).is_eq()
            {
                to_apply = buffer.next().unwrap();
            }
            apply(to_apply);
            next_insert = buffer.next();
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use rand::seq::SliceRandom;

    use crate::{B, Map};

    #[test]
    fn insert_one() {
        let mut map = Map::new();
        map.insert(2, 3);
    }

    #[test]
    fn get_from_empty() {
        let map: Map<usize, usize> = Map::new();
        assert_eq!(map.get(&5), None);
    }

    #[test]
    fn get_one() {
        let mut map = Map::new();
        map.insert(2, 3);

        assert_eq!(map.get(&2), Some(&3));
    }

    #[test]
    fn insert_ordered() {
        let mut map = Map::new();
        for i in 10..15 {
            map.insert(i, i * 2);
        }

        assert_eq!(map.get(&12), Some(&24));
        assert_eq!(map.get(&14), Some(&28));
        assert_eq!(map.get(&16), None);
        assert_eq!(map.get(&7), None);
    }

    #[test]
    fn insert_ordered_overflow() {
        let mut map = Map::new();
        for i in 10..10 + B * 3 {
            map.insert(i, i * 2);
        }

        let index: Vec<_> = (10..10 + B * 3).collect();
        for i in index {
            assert_eq!(map.get(&i), Some(&(i * 2)));
        }

        assert_eq!(map.get(&7), None);
        assert_eq!(map.get(&(15 + B * 3)), None);
    }

    #[test]
    fn insert_ordered_overflow_get_random() {
        let mut map = Map::new();
        for i in 10..10 + B * 3 {
            map.insert(i, i * 2);
        }

        let mut index: Vec<_> = (10..10 + B * 3).collect();
        index.shuffle(&mut rand::rng());
        for i in index {
            assert_eq!(map.get(&i), Some(&(i * 2)));
        }

        assert_eq!(map.get(&7), None);
        assert_eq!(map.get(&(15 + B * 3)), None);
    }

    #[test]
    fn get_head_of_branch() {
        let mut map = Map::new();
        for i in 0..(B * 3) {
            map.insert(i, i * 2);
        }

        assert_eq!(map.get(&(B / 2)), Some(&B));
    }

    #[test]
    fn get_in_new_branch() {
        let mut map = Map::new();
        for i in 0..(B * 3) {
            map.insert(i, i * 2);
        }

        assert_eq!(map.get(&(B / 2 * 3)), Some(&(B * 3)));
    }

    #[test]
    fn insert_ordered_recursive_overflow() {
        let mut map = Map::new();
        let max = 10 + B * B * 3;
        for i in 10..max {
            map.insert(i, i * 2);
        }

        let index: Vec<_> = (10..max).collect();
        for i in index {
            assert_eq!(map.get(&i), Some(&(i * 2)));
        }

        assert_eq!(map.get(&7), None);
        assert_eq!(map.get(&(15 + max)), None);
    }

    #[test]
    fn insert_ordered_recursive_overflow_get_random() {
        let mut map = Map::new();
        let max = 10 + B * B * 3;
        for i in 10..max {
            map.insert(i, i * 2);
        }

        let mut index: Vec<_> = (10..max).collect();
        index.shuffle(&mut rand::rng());
        for i in index {
            assert_eq!(map.get(&i), Some(&(i * 2)));
        }

        assert_eq!(map.get(&7), None);
        assert_eq!(map.get(&(15 + max)), None);
    }

    #[test]
    fn alternate() {
        let mut map = Map::new();

        for i in 0..B * B * 3 {
            map.insert(i, i);
            assert_eq!(map.get(&i), Some(&i));
        }
    }
}
