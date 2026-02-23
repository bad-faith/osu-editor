use std::{sync::Arc};

use rand::random;

pub struct TreapNode<T: Clone> {
    weight: i32,
    size: usize,
    value: T,
    left: Treap<T>,
    right: Treap<T>,
}

pub struct Treap<T: Clone>(Option<Arc<TreapNode<T>>>);

impl<T: Clone> Clone for Treap<T> {
    fn clone(&self) -> Self {
        match self {
            Treap(None) => Treap(None),
            Treap(Some(node)) => Treap(Some(Arc::clone(node))),
        }
    }
}

impl<T: Clone> Treap<T> {
    pub fn len(&self) -> usize {
        self.size()
    }

    pub fn new_empty() -> Self {
        Treap(None)
    }

    pub fn new(value: T) -> Self {
        Treap(Some(Arc::new(TreapNode {
            weight: random(),
            size: 1,
            value,
            left: Treap(None),
            right: Treap(None),
        })))
    }

    pub fn from_slice(vec: &[T]) -> Self {
        let mut treap = Treap::new_empty();
        for value in vec {
            treap = treap.merge(Treap::new(value.clone()));
        }
        treap
    }

    pub fn size(&self) -> usize {
        match &self.0 {
            Some(node) => node.size,
            None => 0,
        }
    }

    pub fn split(&self, left_split_size: usize) -> (Treap<T>, Treap<T>) {
        match self {
            Treap(None) => (Treap(None), Treap(None)),
            Treap(Some(node)) => {
                if left_split_size > node.size {
                    panic!("split size cannot be greater than treap size");
                }
                let left_subtree_size = node.left.size();
                if left_subtree_size >= left_split_size {
                    let (left, right) = node.left.split(left_split_size);
                    let left_size = left.size();
                    return (
                        left,
                        Treap(Some(Arc::new(TreapNode {
                            weight: node.weight,
                            size: node.size - left_size,
                            value: node.value.clone(),
                            left: right,
                            right: node.right.clone(),
                        }))),
                    );
                } else {
                    let (left, right) = node.right.split(left_split_size - left_subtree_size - 1);
                    return (
                        Treap(Some(Arc::new(TreapNode {
                            weight: node.weight,
                            size: left_subtree_size + 1 + left.size(),
                            value: node.value.clone(),
                            left: node.left.clone(),
                            right: left,
                        }))),
                        right,
                    );
                }
            }
        }
    }

    pub fn merge(self: Treap<T>, right: Treap<T>) -> Treap<T> {
        match (self, right) {
            (Treap(None), r) => r,
            (l, Treap(None)) => l,
            (Treap(Some(left)), Treap(Some(right))) => {
                if left.weight > right.weight {
                    return Treap(Some(Arc::new(TreapNode {
                        weight: left.weight,
                        size: left.size + right.size,
                        value: left.value.clone(),
                        left: left.left.clone(),
                        right: Treap::merge(left.right.clone(), Treap(Some(right))),
                    })));
                } else {
                    return Treap(Some(Arc::new(TreapNode {
                        weight: right.weight,
                        size: left.size + right.size,
                        value: right.value.clone(),
                        left: Treap::merge(Treap(Some(left)), right.left.clone()),
                        right: right.right.clone(),
                    })));
                }
            }
        }
    }

    pub fn mutate(&self, index: usize, f: impl FnOnce(T) -> T) -> Treap<T> {
        let (left, value, right) = self.split_by_index(index);
        let new_value = f(value);
        return left.merge(Treap::new(new_value)).merge(right);
    }

    pub fn get(&self, index: usize) -> T {
        let (_, value, _) = self.split_by_index(index);
        value
    }

    fn split_by_index(&self, index: usize) -> (Treap<T>, T, Treap<T>) {
        let (left, right) = self.split(index);
        let (middle, right) = right.split(1);
        match middle {
            Treap(None) => panic!("split_by_index should return exactly one middle element"),
            Treap(Some(node)) => return (left, node.value.clone(), right),
        }
    }

    pub fn iter<'a>(&'a self) -> TreapIter<'a, T> {
        match self {
            Treap(None) => TreapIter { vec: Vec::new() },
            Treap(Some(node)) => TreapIter {
                vec: vec![TreeOrValue::Tree(&node)],
            },
        }
    }
}

#[derive(Clone)]
pub struct TreapIter<'a, T: Clone> {
    vec: Vec<TreeOrValue<'a, T>>,
}

impl<'a, T: Clone> Iterator for TreapIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(top) = self.vec.pop() {
            match top {
                TreeOrValue::Tree(node) => {
                    self.vec.extend(node.right.iter().vec);
                    self.vec.push(TreeOrValue::Value(&node.value));
                    self.vec.extend(node.left.iter().vec);
                }
                TreeOrValue::Value(value) => return Some(value),
            }
        }
        return None;
    }
}

#[derive(Clone)]
enum TreeOrValue<'a, T: Clone> {
    Tree(&'a Arc<TreapNode<T>>),
    Value(&'a T),
}
