use crate::{Branch, Map, Motion, Visitor};

struct GetVisitor<'a, K, V> {
    key: &'a K,
    result: Option<*mut V>,
}

impl<K: Ord, V> Visitor<K, V> for GetVisitor<'_, K, V> {
    #[inline]
    fn visit_internal(&mut self, array: &mut [Branch<K, V>]) -> Motion {
        match array.binary_search_by(|b| b.key.cmp(self.key)) {
            Ok(i) => {
                self.result = Some(array[i].value.boxify());
                Motion::Finish
            }
            Err(i) => Motion::Down(i),
        }
    }

    #[inline]
    fn visit_leaf(&mut self, array: &mut [(K, V)]) {
        if let Ok(i) = array.binary_search_by(|(k, _)| k.cmp(self.key)) {
            self.result = Some(&mut array[i].1);
        }
    }
}

struct GetKeyValueVisitor<'a, K, V> {
    key: &'a K,
    result: Option<(*const K, *mut V)>,
}

impl<K: Ord + Clone, V> Visitor<K, V> for GetKeyValueVisitor<'_, K, V> {
    #[inline]
    fn visit_internal(&mut self, array: &mut [Branch<K, V>]) -> Motion {
        match array.binary_search_by(|b| b.key.cmp(self.key)) {
            Ok(i) => {
                self.result = Some((array[i].boxify_key(), array[i].value.boxify()));
                Motion::Finish
            }
            Err(i) => Motion::Down(i),
        }
    }

    #[inline]
    fn visit_leaf(&mut self, array: &mut [(K, V)]) {
        if let Ok(i) = array.binary_search_by(|(k, _)| k.cmp(self.key)) {
            self.result = Some((&array[i].0, &mut array[i].1));
        }
    }
}

impl<K: Ord, V> Map<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        let mut visitor = GetVisitor { key, result: None };
        self.accept_visitor(&mut visitor);
        visitor.result.map(|ptr| unsafe { &*ptr })
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let mut visitor = GetVisitor { key, result: None };
        self.accept_visitor(&mut visitor);
        visitor.result.map(|ptr| unsafe { &mut *ptr })
    }

    pub fn get_key_value(&self, key: &K) -> Option<(&K, &V)>
    where
        K: Clone,
    {
        let mut visitor = GetKeyValueVisitor { key, result: None };
        self.accept_visitor(&mut visitor);
        visitor.result.map(|(key, val)| unsafe { (&*key, &*val) })
    }

    pub fn get_key_value_mut(&mut self, key: &K) -> Option<(&K, &mut V)>
    where
        K: Clone,
    {
        let mut visitor = GetKeyValueVisitor { key, result: None };
        self.accept_visitor(&mut visitor);
        visitor
            .result
            .map(|(key, val)| unsafe { (&*key, &mut *val) })
    }

    pub fn get_before(&self, key: &K) -> Option<&V> {
        let mut visitor = GetValueBeforeVisitor {
            key,
            inclusive: false,
            result: None,
            previous_branch: None,
        };
        self.accept_visitor(&mut visitor);
        visitor.result.map(|ptr| unsafe { &*ptr })
    }

    pub fn get_before_inc(&self, key: &K) -> Option<&V> {
        let mut visitor = GetValueBeforeVisitor {
            key,
            inclusive: true,
            result: None,
            previous_branch: None,
        };
        self.accept_visitor(&mut visitor);
        visitor.result.map(|ptr| unsafe { &*ptr })
    }

    pub fn get_key_value_before(&self, key: &K) -> Option<(&K, &V)>
    where
        K: Clone,
    {
        let mut visitor = GetKeyValueBeforeVisitor {
            key,
            inclusive: false,
            result: None,
            previous_branch: None,
        };
        self.accept_visitor(&mut visitor);
        visitor.result.map(|(key, val)| unsafe { (&*key, &*val) })
    }

    pub fn get_key_value_before_inc(&self, key: &K) -> Option<(&K, &V)>
    where
        K: Clone,
    {
        let mut visitor = GetKeyValueBeforeVisitor {
            key,
            inclusive: true,
            result: None,
            previous_branch: None,
        };
        self.accept_visitor(&mut visitor);
        visitor.result.map(|(key, val)| unsafe { (&*key, &*val) })
    }
}

struct GetValueBeforeVisitor<'a, K, V> {
    key: &'a K,
    inclusive: bool,
    previous_branch: Option<*mut Branch<K, V>>,
    result: Option<*mut V>,
}

impl<K: Ord, V> Visitor<K, V> for GetValueBeforeVisitor<'_, K, V> {
    #[inline]
    fn visit_internal(&mut self, array: &mut [Branch<K, V>]) -> Motion {
        match array.binary_search_by(|b| b.key.cmp(self.key)) {
            Ok(i) if self.inclusive => {
                self.result = Some(array[i].value.boxify());
                Motion::Finish
            }
            Ok(i) | Err(i) => {
                if i != 0 {
                    self.previous_branch = Some(&mut array[i - 1]);
                }
                Motion::Down(i)
            }
        }
    }

    #[inline]
    fn visit_leaf(&mut self, array: &mut [(K, V)]) {
        match array.binary_search_by(|(k, _)| k.cmp(self.key)) {
            Ok(i) if self.inclusive => self.result = Some(&mut array[i].1),
            Ok(i) | Err(i) => {
                if i != 0 {
                    self.result = Some(&mut array[i - 1].1);
                } else {
                    self.result = self.previous_branch.map(|b| {
                        let b = unsafe { &mut *b };
                        b.value.boxify()
                    })
                }
            }
        }
    }
}

struct GetKeyValueBeforeVisitor<'a, K, V> {
    key: &'a K,
    inclusive: bool,
    previous_branch: Option<*mut Branch<K, V>>,
    result: Option<(*const K, *mut V)>,
}

impl<K: Ord + Clone, V> Visitor<K, V> for GetKeyValueBeforeVisitor<'_, K, V> {
    #[inline]
    fn visit_internal(&mut self, array: &mut [Branch<K, V>]) -> Motion {
        match array.binary_search_by(|b| b.key.cmp(self.key)) {
            Ok(i) if self.inclusive => {
                self.result = Some((array[i].boxify_key(), array[i].value.boxify()));
                Motion::Finish
            }
            Ok(i) | Err(i) => {
                if i != 0 {
                    self.previous_branch = Some(&mut array[i - 1]);
                }
                Motion::Down(i)
            }
        }
    }

    #[inline]
    fn visit_leaf(&mut self, array: &mut [(K, V)]) {
        match array.binary_search_by(|(k, _)| k.cmp(self.key)) {
            Ok(i) if self.inclusive => self.result = Some((&mut array[i].0, &mut array[i].1)),
            Ok(i) | Err(i) => {
                if i != 0 {
                    self.result = Some((&mut array[i - 1].0, &mut array[i - 1].1));
                } else {
                    self.result = self.previous_branch.map(|b| {
                        let b = unsafe { &mut *b };
                        (b.boxify_key(), b.value.boxify())
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::seq::SliceRandom;

    use crate::{B, Map};

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
    fn get_mut() {
        let mut map = Map::new();
        map.insert(1, 1);
        *map.get_mut(&1).unwrap() = 5;
        assert_eq!(map.get(&1), Some(&5));
    }

    #[test]
    fn get_before() {
        let mut map = Map::new();
        map.insert(1, 1);
        map.insert(3, 3);
        assert_eq!(map.get_before(&3), Some(&1));
        assert_eq!(map.get_before(&2), Some(&1));
        assert_eq!(map.get_before(&1), None);
    }

    #[test]
    fn get_before_inc() {
        let mut map = Map::new();
        map.insert(1, 1);
        map.insert(3, 3);
        assert_eq!(map.get_before_inc(&1), Some(&1));
        assert_eq!(map.get_before_inc(&3), Some(&3));
    }

    #[test]
    fn get_key_value_before() {
        let mut map = Map::new();
        map.insert(1, "asdf");
        map.insert(3, "zxcv");
        assert_eq!(map.get_key_value_before(&3), Some((&1, &"asdf")));
        assert_eq!(map.get_key_value_before(&2), Some((&1, &"asdf")));
        assert_eq!(map.get_key_value_before(&1), None);
    }

    #[test]
    fn get_key_value_before_inc() {
        let mut map = Map::new();
        map.insert(1, "asdf");
        map.insert(3, "zxcv");
        assert_eq!(map.get_key_value_before_inc(&1), Some((&1, &"asdf")));
        assert_eq!(map.get_key_value_before_inc(&3), Some((&3, &"zxcv")));
    }

    #[test]
    fn get_key_value_mut() {
        let mut map = Map::new();
        map.insert(1, 2);
        let (key, val) = map.get_key_value_mut(&1).unwrap();
        assert_eq!(*key, 1);
        assert_eq!(*val, 2);

        *val = 5;
        assert_eq!(map.get(&1).unwrap(), &5);
    }
}
