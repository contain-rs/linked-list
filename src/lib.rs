//! # Description
//!
//! An alternative implementation of standard `LinkedList` featuring a prototype `Cursor`.

#![no_std]

#[cfg(any(test, feature = "std"))]
#[cfg_attr(test, macro_use)]
extern crate std;

use core::cmp::Ordering;
use core::fmt::{self, Debug};
use core::hash::{Hash, Hasher};
use core::iter::FromIterator;
use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;

use allocator_api2::{
    alloc::{Allocator, Global},
    boxed::Box,
};

pub struct LinkedList<T, A: Allocator = Global> {
    front: Link<T>,
    back: Link<T>,
    len: usize,
    alloc: A,
    _boo: PhantomData<T>,
}

type Link<T> = Option<NonNull<Node<T>>>;

struct Node<T> {
    front: Link<T>,
    back: Link<T>,
    elem: T,
}

pub struct Iter<'a, T> {
    front: Link<T>,
    back: Link<T>,
    len: usize,
    _boo: PhantomData<&'a T>,
}

pub struct IterMut<'a, T> {
    front: Link<T>,
    back: Link<T>,
    len: usize,
    _boo: PhantomData<&'a mut T>,
}

pub struct IntoIter<T, A: Allocator = Global> {
    list: LinkedList<T, A>,
}

pub struct CursorMut<'a, T, A: Allocator = Global> {
    list: &'a mut LinkedList<T, A>,
    cur: Link<T>,
    index: Option<usize>,
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        Self::new_in(Default::default())
    }
}

impl<T, A: Allocator> LinkedList<T, A> {
    pub fn new_in(alloc: A) -> Self {
        Self {
            front: None,
            back: None,
            len: 0,
            alloc,
            _boo: PhantomData,
        }
    }

    pub fn push_front(&mut self, elem: T) {
        // SAFETY: it's a linked-list, what do you want?
        unsafe {
            let new = NonNull::new_unchecked(Box::into_raw(Box::new_in(
                Node {
                    front: None,
                    back: None,
                    elem,
                },
                &self.alloc,
            )));
            if let Some(old) = self.front {
                // Put the new front before the old one
                (*old.as_ptr()).front = Some(new);
                (*new.as_ptr()).back = Some(old);
            } else {
                // If there's no front, then we're the empty list and need
                // to set the back too.
                self.back = Some(new);
            }
            // These things always happen!
            self.front = Some(new);
            self.len += 1;
        }
    }

    pub fn push_back(&mut self, elem: T) {
        // SAFETY: it's a linked-list, what do you want?
        unsafe {
            let new = NonNull::new_unchecked(Box::into_raw(Box::new_in(
                Node {
                    back: None,
                    front: None,
                    elem,
                },
                &self.alloc,
            )));
            if let Some(old) = self.back {
                // Put the new back before the old one
                (*old.as_ptr()).back = Some(new);
                (*new.as_ptr()).front = Some(old);
            } else {
                // If there's no back, then we're the empty list and need
                // to set the front too.
                self.front = Some(new);
            }
            // These things always happen!
            self.back = Some(new);
            self.len += 1;
        }
    }

    pub fn pop_front(&mut self) -> Option<T> {
        // workaround for a bug in allocator-api2
        fn into_inner<T, A: Allocator>(boxed: Box<T, A>) -> T {
            use allocator_api2::alloc::Layout;
            let (ptr, alloc) = Box::into_raw_with_allocator(boxed);
            let unboxed = unsafe { ptr.read() };
            unsafe { alloc.deallocate(NonNull::new(ptr).unwrap().cast(), Layout::new::<T>()) };
            unboxed
        }

        unsafe {
            // Only have to do stuff if there is a front node to pop.
            self.front.map(|node| {
                // Bring the Box back to life so we can move out its value and
                // Drop it (Box continues to magically understand this for us).
                let boxed_node = Box::from_raw_in(node.as_ptr(), &self.alloc);
                let node = into_inner(boxed_node);
                let result = node.elem;

                // Make the next node into the new front.
                self.front = node.back;
                if let Some(new) = self.front {
                    // Cleanup its reference to the removed node
                    (*new.as_ptr()).front = None;
                } else {
                    // If the front is now null, then this list is now empty!
                    self.back = None;
                }

                self.len -= 1;
                result
                // Box gets implicitly freed here, knows there is no T.
            })
        }
    }

    pub fn pop_back(&mut self) -> Option<T> {
        // workaround for a bug in allocator-api2
        fn into_inner<T, A: Allocator>(boxed: Box<T, A>) -> T {
            use allocator_api2::alloc::Layout;
            let (ptr, alloc) = Box::into_raw_with_allocator(boxed);
            let unboxed = unsafe { ptr.read() };
            unsafe { alloc.deallocate(NonNull::new(ptr).unwrap().cast(), Layout::new::<T>()) };
            unboxed
        }

        unsafe {
            // Only have to do stuff if there is a back node to pop.
            self.back.map(|node| {
                // Bring the Box front to life so we can move out its value and
                // Drop it (Box continues to magically understand this for us).
                let boxed_node = Box::from_raw(node.as_ptr());
                let node = into_inner(boxed_node);
                let result = node.elem;

                // Make the next node into the new back.
                self.back = node.front;
                if let Some(new) = self.back {
                    // Cleanup its reference to the removed node
                    (*new.as_ptr()).back = None;
                } else {
                    // If the back is now null, then this list is now empty!
                    self.front = None;
                }

                self.len -= 1;
                result
                // Box gets implicitly freed here, knows there is no T.
            })
        }
    }

    pub fn front(&self) -> Option<&T> {
        unsafe { self.front.map(|node| &(*node.as_ptr()).elem) }
    }

    pub fn front_mut(&mut self) -> Option<&mut T> {
        unsafe { self.front.map(|node| &mut (*node.as_ptr()).elem) }
    }

    pub fn back(&self) -> Option<&T> {
        unsafe { self.back.map(|node| &(*node.as_ptr()).elem) }
    }

    pub fn back_mut(&mut self) -> Option<&mut T> {
        unsafe { self.back.map(|node| &mut (*node.as_ptr()).elem) }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        // Oh look it's drop again
        while self.pop_front().is_some() {}
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            front: self.front,
            back: self.back,
            len: self.len,
            _boo: PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            front: self.front,
            back: self.back,
            len: self.len,
            _boo: PhantomData,
        }
    }

    pub fn cursor_mut(&mut self) -> CursorMut<T, A> {
        CursorMut {
            list: self,
            cur: None,
            index: None,
        }
    }
}

impl<T, A: Allocator> Drop for LinkedList<T, A> {
    fn drop(&mut self) {
        // Pop until we have to stop
        while self.pop_front().is_some() {}
    }
}

impl<T, A: Allocator + Default> Default for LinkedList<T, A> {
    fn default() -> Self {
        Self::new_in(Default::default())
    }
}

impl<T: Clone, A: Allocator + Clone> Clone for LinkedList<T, A> {
    fn clone(&self) -> Self {
        let mut new_list = Self::new_in(self.alloc.clone());
        for item in self {
            new_list.push_back(item.clone());
        }
        new_list
    }
}

impl<T, A: Allocator> Extend<T> for LinkedList<T, A> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push_back(item);
        }
    }
}

impl<T, A: Allocator + Default> FromIterator<T> for LinkedList<T, A> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut list = Self::new_in(Default::default());
        list.extend(iter);
        list
    }
}

impl<T: Debug, A: Allocator> Debug for LinkedList<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self).finish()
    }
}

impl<T, U, A1, A2> PartialEq<LinkedList<U, A2>> for LinkedList<T, A1>
where
    T: PartialEq<U>,
    A1: Allocator,
    A2: Allocator,
{
    fn eq(&self, other: &LinkedList<U, A2>) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl<T: Eq, A: Allocator> Eq for LinkedList<T, A> {}

impl<T, A1, A2> PartialOrd<LinkedList<T, A2>> for LinkedList<T, A1>
where
    T: PartialOrd,
    A1: Allocator,
    A2: Allocator,
{
    fn partial_cmp(&self, other: &LinkedList<T, A2>) -> Option<Ordering> {
        self.iter().partial_cmp(other)
    }
}

impl<T: Ord, A: Allocator> Ord for LinkedList<T, A> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other)
    }
}

impl<T: Hash, A: Allocator> Hash for LinkedList<T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len().hash(state);
        for item in self {
            item.hash(state);
        }
    }
}

impl<'a, T, A: Allocator> IntoIterator for &'a LinkedList<T, A> {
    type IntoIter = Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        // While self.front == self.back is a tempting condition to check here,
        // it won't do the right for yielding the last element! That sort of
        // thing only works for arrays because of "one-past-the-end" pointers.
        if self.len > 0 {
            // We could unwrap front, but this is safer and easier
            self.front.map(|node| unsafe {
                self.len -= 1;
                self.front = (*node.as_ptr()).back;
                &(*node.as_ptr()).elem
            })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len > 0 {
            self.back.map(|node| unsafe {
                self.len -= 1;
                self.back = (*node.as_ptr()).front;
                &(*node.as_ptr()).elem
            })
        } else {
            None
        }
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T, A: Allocator> IntoIterator for &'a mut LinkedList<T, A> {
    type IntoIter = IterMut<'a, T>;
    type Item = &'a mut T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        // While self.front == self.back is a tempting condition to check here,
        // it won't do the right for yielding the last element! That sort of
        // thing only works for arrays because of "one-past-the-end" pointers.
        if self.len > 0 {
            // We could unwrap front, but this is safer and easier
            self.front.map(|node| unsafe {
                self.len -= 1;
                self.front = (*node.as_ptr()).back;
                &mut (*node.as_ptr()).elem
            })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len > 0 {
            self.back.map(|node| unsafe {
                self.len -= 1;
                self.back = (*node.as_ptr()).front;
                &mut (*node.as_ptr()).elem
            })
        } else {
            None
        }
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<T, A: Allocator> IntoIterator for LinkedList<T, A> {
    type IntoIter = IntoIter<T, A>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { list: self }
    }
}

impl<T, A: Allocator> Iterator for IntoIter<T, A> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.list.pop_front()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.list.len, Some(self.list.len))
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.list.pop_back()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.list.len
    }
}

impl<'a, T, A: Allocator> CursorMut<'a, T, A> {
    pub fn index(&self) -> Option<usize> {
        self.index
    }

    pub fn move_next(&mut self) {
        if let Some(cur) = self.cur {
            unsafe {
                // We're on a real element, go to its next (back)
                self.cur = (*cur.as_ptr()).back;
                if self.cur.is_some() {
                    *self.index.as_mut().unwrap() += 1;
                } else {
                    // We just walked to the ghost, no more index
                    self.index = None;
                }
            }
        } else if !self.list.is_empty() {
            // We're at the ghost, and there is a real front, so move to it!
            self.cur = self.list.front;
            self.index = Some(0)
        } else {
            // We're at the ghost, but that's the only element... do nothing.
        }
    }

    pub fn move_prev(&mut self) {
        if let Some(cur) = self.cur {
            unsafe {
                // We're on a real element, go to its previous (front)
                self.cur = (*cur.as_ptr()).front;
                if self.cur.is_some() {
                    *self.index.as_mut().unwrap() -= 1;
                } else {
                    // We just walked to the ghost, no more index
                    self.index = None;
                }
            }
        } else if !self.list.is_empty() {
            // We're at the ghost, and there is a real back, so move to it!
            self.cur = self.list.back;
            self.index = Some(self.list.len - 1)
        } else {
            // We're at the ghost, but that's the only element... do nothing.
        }
    }

    pub fn current(&mut self) -> Option<&mut T> {
        unsafe { self.cur.map(|node| &mut (*node.as_ptr()).elem) }
    }

    pub fn peek_next(&mut self) -> Option<&mut T> {
        unsafe {
            let next = if let Some(cur) = self.cur {
                // Normal case, try to follow the cur node's back pointer
                (*cur.as_ptr()).back
            } else {
                // Ghost case, try to use the list's front pointer
                self.list.front
            };

            // Yield the element if the next node exists
            next.map(|node| &mut (*node.as_ptr()).elem)
        }
    }

    pub fn peek_prev(&mut self) -> Option<&mut T> {
        unsafe {
            let prev = if let Some(cur) = self.cur {
                // Normal case, try to follow the cur node's front pointer
                (*cur.as_ptr()).front
            } else {
                // Ghost case, try to use the list's back pointer
                self.list.back
            };

            // Yield the element if the prev node exists
            prev.map(|node| &mut (*node.as_ptr()).elem)
        }
    }

    pub fn split_before(&mut self) -> LinkedList<T, A>
    where
        A: Copy,
    {
        // We have this:
        //
        //     list.front -> A <-> B <-> C <-> D <- list.back
        //                               ^
        //                              cur
        //
        //
        // And we want to produce this:
        //
        //     list.front -> C <-> D <- list.back
        //                   ^
        //                  cur
        //
        //
        //    return.front -> A <-> B <- return.back
        //
        if let Some(cur) = self.cur {
            // We are pointing at a real element, so the list is non-empty.
            unsafe {
                // Current state
                let old_len = self.list.len;
                let old_idx = self.index.unwrap();
                let prev = (*cur.as_ptr()).front;

                // What self will become
                let new_len = old_len - old_idx;
                let new_front = self.cur;
                let new_back = self.list.back;
                let new_idx = Some(0);

                // What the output will become
                let output_len = old_len - new_len;
                let output_front = self.list.front;
                let output_back = prev;

                // Break the links between cur and prev
                if let Some(prev) = prev {
                    (*cur.as_ptr()).front = None;
                    (*prev.as_ptr()).back = None;
                }

                // Produce the result:
                self.list.len = new_len;
                self.list.front = new_front;
                self.list.back = new_back;
                self.index = new_idx;

                LinkedList {
                    front: output_front,
                    back: output_back,
                    len: output_len,
                    alloc: self.list.alloc,
                    _boo: PhantomData,
                }
            }
        } else {
            // We're at the ghost, just replace our list with an empty one.
            // No other state needs to be changed.
            mem::replace(self.list, LinkedList::new_in(self.list.alloc))
        }
    }

    pub fn split_after(&mut self) -> LinkedList<T, A>
    where
        A: Copy,
    {
        // We have this:
        //
        //     list.front -> A <-> B <-> C <-> D <- list.back
        //                         ^
        //                        cur
        //
        //
        // And we want to produce this:
        //
        //     list.front -> A <-> B <- list.back
        //                         ^
        //                        cur
        //
        //
        //    return.front -> C <-> D <- return.back
        //
        if let Some(cur) = self.cur {
            // We are pointing at a real element, so the list is non-empty.
            unsafe {
                // Current state
                let old_len = self.list.len;
                let old_idx = self.index.unwrap();
                let next = (*cur.as_ptr()).back;

                // What self will become
                let new_len = old_idx + 1;
                let new_back = self.cur;
                let new_front = self.list.front;
                let new_idx = Some(old_idx);

                // What the output will become
                let output_len = old_len - new_len;
                let output_front = next;
                let output_back = self.list.back;

                // Break the links between cur and next
                if let Some(next) = next {
                    (*cur.as_ptr()).back = None;
                    (*next.as_ptr()).front = None;
                }

                // Produce the result:
                self.list.len = new_len;
                self.list.front = new_front;
                self.list.back = new_back;
                self.index = new_idx;

                LinkedList {
                    front: output_front,
                    back: output_back,
                    len: output_len,
                    alloc: self.list.alloc,
                    _boo: PhantomData,
                }
            }
        } else {
            // We're at the ghost, just replace our list with an empty one.
            // No other state needs to be changed.
            mem::replace(self.list, LinkedList::new_in(self.list.alloc))
        }
    }

    pub fn splice_before(&mut self, mut input: LinkedList<T, A>) {
        // We have this:
        //
        // input.front -> 1 <-> 2 <- input.back
        //
        // list.front -> A <-> B <-> C <- list.back
        //                     ^
        //                    cur
        //
        //
        // Becoming this:
        //
        // list.front -> A <-> 1 <-> 2 <-> B <-> C <- list.back
        //                                 ^
        //                                cur
        //
        unsafe {
            // We can either `take` the input's pointers or `mem::forget`
            // it. Using `take` is more responsible in case we ever do custom
            // allocators or something that also needs to be cleaned up!
            if input.is_empty() {
                // Input is empty, do nothing.
            } else if let Some(cur) = self.cur {
                // Both lists are non-empty
                let in_front = input.front.take().unwrap();
                let in_back = input.back.take().unwrap();

                if let Some(prev) = (*cur.as_ptr()).front {
                    // General Case, no boundaries, just internal fixups
                    (*prev.as_ptr()).back = Some(in_front);
                    (*in_front.as_ptr()).front = Some(prev);
                    (*cur.as_ptr()).front = Some(in_back);
                    (*in_back.as_ptr()).back = Some(cur);
                } else {
                    // No prev, we're appending to the front
                    (*cur.as_ptr()).front = Some(in_back);
                    (*in_back.as_ptr()).back = Some(cur);
                    self.list.front = Some(in_front);
                }
                // Index moves forward by input length
                *self.index.as_mut().unwrap() += input.len;
            } else if let Some(back) = self.list.back {
                // We're on the ghost but non-empty, append to the back
                let in_front = input.front.take().unwrap();
                let in_back = input.back.take().unwrap();

                (*back.as_ptr()).back = Some(in_front);
                (*in_front.as_ptr()).front = Some(back);
                self.list.back = Some(in_back);
            } else {
                // We're empty, become the input, remain on the ghost
                mem::swap(self.list, &mut input);
            }

            self.list.len += input.len;
            // Not necessary but Polite To Do
            input.len = 0;

            // Input dropped here
        }
    }

    pub fn splice_after(&mut self, mut input: LinkedList<T, A>) {
        // We have this:
        //
        // input.front -> 1 <-> 2 <- input.back
        //
        // list.front -> A <-> B <-> C <- list.back
        //                     ^
        //                    cur
        //
        //
        // Becoming this:
        //
        // list.front -> A <-> B <-> 1 <-> 2 <-> C <- list.back
        //                     ^
        //                    cur
        //
        unsafe {
            // We can either `take` the input's pointers or `mem::forget`
            // it. Using `take` is more responsible in case we ever do custom
            // allocators or something that also needs to be cleaned up!
            if input.is_empty() {
                // Input is empty, do nothing.
            } else if let Some(cur) = self.cur {
                // Both lists are non-empty
                let in_front = input.front.take().unwrap();
                let in_back = input.back.take().unwrap();

                if let Some(next) = (*cur.as_ptr()).back {
                    // General Case, no boundaries, just internal fixups
                    (*next.as_ptr()).front = Some(in_back);
                    (*in_back.as_ptr()).back = Some(next);
                    (*cur.as_ptr()).back = Some(in_front);
                    (*in_front.as_ptr()).front = Some(cur);
                } else {
                    // No next, we're appending to the back
                    (*cur.as_ptr()).back = Some(in_front);
                    (*in_front.as_ptr()).front = Some(cur);
                    self.list.back = Some(in_back);
                }
                // Index doesn't change
            } else if let Some(front) = self.list.front {
                // We're on the ghost but non-empty, append to the front
                let in_front = input.front.take().unwrap();
                let in_back = input.back.take().unwrap();

                (*front.as_ptr()).front = Some(in_back);
                (*in_back.as_ptr()).back = Some(front);
                self.list.front = Some(in_front);
            } else {
                // We're empty, become the input, remain on the ghost
                mem::swap(self.list, &mut input);
            }

            self.list.len += input.len;
            // Not necessary but Polite To Do
            input.len = 0;

            // Input dropped here
        }
    }
}

unsafe impl<T: Send> Send for LinkedList<T> {}
unsafe impl<T: Sync> Sync for LinkedList<T> {}

unsafe impl<'a, T: Send> Send for Iter<'a, T> {}
unsafe impl<'a, T: Sync> Sync for Iter<'a, T> {}

unsafe impl<'a, T: Send> Send for IterMut<'a, T> {}
unsafe impl<'a, T: Sync> Sync for IterMut<'a, T> {}

#[allow(dead_code)]
fn assert_properties() {
    fn is_send<T: Send>() {}
    fn is_sync<T: Sync>() {}

    is_send::<LinkedList<i32>>();
    is_sync::<LinkedList<i32>>();

    is_send::<IntoIter<i32>>();
    is_sync::<IntoIter<i32>>();

    is_send::<Iter<i32>>();
    is_sync::<Iter<i32>>();

    is_send::<IterMut<i32>>();
    is_sync::<IterMut<i32>>();

    fn linked_list_covariant<'a, T>(x: LinkedList<&'static T>) -> LinkedList<&'a T> {
        x
    }
    fn iter_covariant<'i, 'a, T>(x: Iter<'i, &'static T>) -> Iter<'i, &'a T> {
        x
    }
    fn into_iter_covariant<'a, T>(x: IntoIter<&'static T>) -> IntoIter<&'a T> {
        x
    }

    /// ```compile_fail
    /// use linked_list::IterMut;
    ///
    /// fn iter_mut_covariant<'i, 'a, T>(x: IterMut<'i, &'static T>) -> IterMut<'i, &'a T> { x }
    /// ```
    fn iter_mut_invariant() {}
}

#[cfg(feature = "serde")]
impl<T, A> serde::Serialize for LinkedList<T, A>
where
    T: serde::Serialize,
    A: Allocator,
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self)
    }
}

#[cfg(feature = "serde")]
impl<'de, T, A> serde::Deserialize<'de> for LinkedList<T, A>
where
    T: serde::Deserialize<'de>,
    A: Allocator + Default,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SeqVisitor<T, A: Allocator> {
            marker: PhantomData<LinkedList<T, A>>,
        }

        impl<'de, T, A> serde::de::Visitor<'de> for SeqVisitor<T, A>
        where
            T: serde::Deserialize<'de>,
            A: Allocator + Default,
        {
            type Value = LinkedList<T, A>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence")
            }

            #[inline]
            fn visit_seq<B>(self, mut seq: B) -> Result<Self::Value, B::Error>
            where
                B: serde::de::SeqAccess<'de>,
            {
                let mut values = LinkedList::new_in(Default::default());

                while let Some(value) = seq.next_element()? {
                    LinkedList::push_back(&mut values, value);
                }

                Ok(values)
            }
        }

        let visitor = SeqVisitor {
            marker: PhantomData,
        };
        deserializer.deserialize_seq(visitor)
    }

    fn deserialize_in_place<D>(deserializer: D, place: &mut Self) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SeqInPlaceVisitor<'a, T: 'a, A: Allocator + 'a>(&'a mut LinkedList<T, A>);

        impl<'a, 'de, T, A> serde::de::Visitor<'de> for SeqInPlaceVisitor<'a, T, A>
        where
            T: serde::Deserialize<'de>,
            A: Allocator,
        {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence")
            }

            #[inline]
            fn visit_seq<B>(mut self, mut seq: B) -> Result<Self::Value, B::Error>
            where
                B: serde::de::SeqAccess<'de>,
            {
                LinkedList::clear(&mut self.0);

                // FIXME: try to overwrite old values here? (Vec, VecDeque, LinkedList)
                while let Some(value) = seq.next_element()? {
                    LinkedList::push_back(&mut self.0, value);
                }

                Ok(())
            }
        }

        deserializer.deserialize_seq(SeqInPlaceVisitor(place))
    }
}

#[cfg(feature = "miniserde")]
impl<T: miniserde::Serialize, A: Allocator> miniserde::Serialize for LinkedList<T, A> {
    fn begin(&self) -> miniserde::ser::Fragment {
        struct Stream<'a, T: 'a>(Iter<'a, T>);

        impl<'a, T: miniserde::Serialize> miniserde::ser::Seq for Stream<'a, T> {
            fn next(&mut self) -> Option<&dyn miniserde::Serialize> {
                let element = self.0.next()?;
                Some(element)
            }
        }

        miniserde::ser::Fragment::Seq(std::boxed::Box::new(Stream(self.iter())))
    }
}

#[cfg(feature = "miniserde")]
impl<T: miniserde::Deserialize, A: Allocator + Default> miniserde::Deserialize
    for LinkedList<T, A>
{
    fn begin(out: &mut Option<Self>) -> &mut dyn miniserde::de::Visitor {
        miniserde::make_place!(Place);

        impl<T: miniserde::Deserialize, A: Allocator + Default> miniserde::de::Visitor
            for Place<LinkedList<T, A>>
        {
            fn seq(&mut self) -> miniserde::Result<std::boxed::Box<dyn miniserde::de::Seq + '_>> {
                Ok(std::boxed::Box::new(VecBuilder {
                    out: &mut self.out,
                    list: LinkedList::new_in(Default::default()),
                    element: None,
                }))
            }
        }

        struct VecBuilder<'a, T: 'a, A: Allocator + 'a> {
            out: &'a mut Option<LinkedList<T, A>>,
            list: LinkedList<T, A>,
            element: Option<T>,
        }

        impl<'a, T, A: Allocator> VecBuilder<'a, T, A> {
            fn shift(&mut self) {
                if let Some(e) = self.element.take() {
                    self.list.push_back(e);
                }
            }
        }

        impl<'a, T: miniserde::Deserialize, A: Allocator + Default> miniserde::de::Seq
            for VecBuilder<'a, T, A>
        {
            fn element(&mut self) -> miniserde::Result<&mut dyn miniserde::de::Visitor> {
                self.shift();
                Ok(miniserde::Deserialize::begin(&mut self.element))
            }

            fn finish(&mut self) -> miniserde::Result<()> {
                self.shift();
                *self.out = Some(mem::take(&mut self.list));
                Ok(())
            }
        }

        Place::new(out)
    }
}

#[cfg(feature = "nanoserde")]
mod nanoserde_impls {
    use super::*;

    impl<T> nanoserde::SerBin for LinkedList<T>
    where
        T: nanoserde::SerBin,
    {
        fn ser_bin(&self, s: &mut std::vec::Vec<u8>) {
            let len = self.len();
            len.ser_bin(s);
            for item in self.iter() {
                item.ser_bin(s);
            }
        }
    }

    impl<T> nanoserde::DeBin for LinkedList<T>
    where
        T: nanoserde::DeBin,
    {
        fn de_bin(o: &mut usize, d: &[u8]) -> Result<LinkedList<T>, nanoserde::DeBinErr> {
            let len: usize = nanoserde::DeBin::de_bin(o, d)?;
            let mut out = LinkedList::new();
            for _ in 0..len {
                out.push_back(nanoserde::DeBin::de_bin(o, d)?)
            }
            Ok(out)
        }
    }

    impl<T> nanoserde::SerJson for LinkedList<T>
    where
        T: nanoserde::SerJson,
    {
        fn ser_json(&self, d: usize, s: &mut nanoserde::SerJsonState) {
            s.out.push('[');
            if self.len() > 0 {
                let last = self.len() - 1;
                for (index, item) in self.iter().enumerate() {
                    s.indent(d + 1);
                    item.ser_json(d + 1, s);
                    if index != last {
                        s.out.push(',');
                    }
                }
            }
            s.out.push(']');
        }
    }

    impl<T> nanoserde::DeJson for LinkedList<T>
    where
        T: nanoserde::DeJson,
    {
        fn de_json(
            s: &mut nanoserde::DeJsonState,
            i: &mut std::str::Chars,
        ) -> Result<LinkedList<T>, nanoserde::DeJsonErr> {
            let mut out = LinkedList::new();
            s.block_open(i)?;

            while s.tok != nanoserde::DeJsonTok::BlockClose {
                out.push_back(nanoserde::DeJson::de_json(s, i)?);
                s.eat_comma_block(i)?;
            }
            s.block_close(i)?;
            Ok(out)
        }
    }

    impl<T> nanoserde::SerRon for LinkedList<T>
    where
        T: nanoserde::SerRon,
    {
        fn ser_ron(&self, d: usize, s: &mut nanoserde::SerRonState) {
            s.out.push('[');
            if self.len() > 0 {
                let last = self.len() - 1;
                for (index, item) in self.iter().enumerate() {
                    s.indent(d + 1);
                    item.ser_ron(d + 1, s);
                    if index != last {
                        s.out.push(',');
                    }
                }
            }
            s.out.push(']');
        }
    }

    impl<T> nanoserde::DeRon for LinkedList<T>
    where
        T: nanoserde::DeRon,
    {
        fn de_ron(
            s: &mut nanoserde::DeRonState,
            i: &mut std::str::Chars,
        ) -> Result<LinkedList<T>, nanoserde::DeRonErr> {
            let mut out = LinkedList::new();
            s.block_open(i)?;

            while s.tok != nanoserde::DeRonTok::BlockClose {
                out.push_back(nanoserde::DeRon::de_ron(s, i)?);
                s.eat_comma_block(i)?;
            }
            s.block_close(i)?;
            Ok(out)
        }
    }
}

#[cfg(feature = "borsh")]
impl<T, A: Allocator + Default> borsh::BorshDeserialize for LinkedList<T, A>
where
    T: borsh::BorshDeserialize,
{
    #[inline]
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let vec = <std::vec::Vec<T>>::deserialize_reader(reader)?;
        Ok(vec.into_iter().collect::<LinkedList<T, A>>())
    }
}

#[cfg(feature = "borsh")]
impl<T, A: Allocator> borsh::BorshSerialize for LinkedList<T, A>
where
    T: borsh::BorshSerialize,
{
    #[inline]
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        fn check_zst<T>() -> borsh::io::Result<()> {
            if core::mem::size_of::<T>() == 0 {
                return Err(borsh::io::Error::new(
                    borsh::io::ErrorKind::InvalidData,
                    borsh::error::ERROR_ZST_FORBIDDEN,
                ));
            }
            Ok(())
        }

        check_zst::<T>()?;

        writer.write_all(
            &(u32::try_from(self.len()).map_err(|_| borsh::io::ErrorKind::InvalidData)?)
                .to_le_bytes(),
        )?;
        for item in self {
            item.serialize(writer)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::LinkedList;

    use std::vec::Vec;

    fn generate_test() -> LinkedList<i32> {
        list_from(&[0, 1, 2, 3, 4, 5, 6])
    }

    fn list_from<T: Clone>(v: &[T]) -> LinkedList<T> {
        v.iter().map(|x| (*x).clone()).collect()
    }

    #[test]
    fn test_basic_front() {
        let mut list = LinkedList::new();

        // Try to break an empty list
        assert_eq!(list.len(), 0);
        assert_eq!(list.pop_front(), None);
        assert_eq!(list.len(), 0);

        // Try to break a one item list
        list.push_front(10);
        assert_eq!(list.len(), 1);
        assert_eq!(list.pop_front(), Some(10));
        assert_eq!(list.len(), 0);
        assert_eq!(list.pop_front(), None);
        assert_eq!(list.len(), 0);

        // Mess around
        list.push_front(10);
        assert_eq!(list.len(), 1);
        list.push_front(20);
        assert_eq!(list.len(), 2);
        list.push_front(30);
        assert_eq!(list.len(), 3);
        assert_eq!(list.pop_front(), Some(30));
        assert_eq!(list.len(), 2);
        list.push_front(40);
        assert_eq!(list.len(), 3);
        assert_eq!(list.pop_front(), Some(40));
        assert_eq!(list.len(), 2);
        assert_eq!(list.pop_front(), Some(20));
        assert_eq!(list.len(), 1);
        assert_eq!(list.pop_front(), Some(10));
        assert_eq!(list.len(), 0);
        assert_eq!(list.pop_front(), None);
        assert_eq!(list.len(), 0);
        assert_eq!(list.pop_front(), None);
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_basic() {
        let mut m = LinkedList::new();
        assert_eq!(m.pop_front(), None);
        assert_eq!(m.pop_back(), None);
        assert_eq!(m.pop_front(), None);
        m.push_front(1);
        assert_eq!(m.pop_front(), Some(1));
        m.push_back(2);
        m.push_back(3);
        assert_eq!(m.len(), 2);
        assert_eq!(m.pop_front(), Some(2));
        assert_eq!(m.pop_front(), Some(3));
        assert_eq!(m.len(), 0);
        assert_eq!(m.pop_front(), None);
        m.push_back(1);
        m.push_back(3);
        m.push_back(5);
        m.push_back(7);
        assert_eq!(m.pop_front(), Some(1));

        let mut n = LinkedList::new();
        n.push_front(2);
        n.push_front(3);
        {
            assert_eq!(n.front().unwrap(), &3);
            let x = n.front_mut().unwrap();
            assert_eq!(*x, 3);
            *x = 0;
        }
        {
            assert_eq!(n.back().unwrap(), &2);
            let y = n.back_mut().unwrap();
            assert_eq!(*y, 2);
            *y = 1;
        }
        assert_eq!(n.pop_front(), Some(0));
        assert_eq!(n.pop_front(), Some(1));
    }

    #[test]
    fn test_iterator() {
        let m = generate_test();
        for (i, elt) in m.iter().enumerate() {
            assert_eq!(i as i32, *elt);
        }
        let mut n = LinkedList::new();
        assert_eq!(n.iter().next(), None);
        n.push_front(4);
        let mut it = n.iter();
        assert_eq!(it.size_hint(), (1, Some(1)));
        assert_eq!(it.next().unwrap(), &4);
        assert_eq!(it.size_hint(), (0, Some(0)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_iterator_double_end() {
        let mut n = LinkedList::new();
        assert_eq!(n.iter().next(), None);
        n.push_front(4);
        n.push_front(5);
        n.push_front(6);
        let mut it = n.iter();
        assert_eq!(it.size_hint(), (3, Some(3)));
        assert_eq!(it.next().unwrap(), &6);
        assert_eq!(it.size_hint(), (2, Some(2)));
        assert_eq!(it.next_back().unwrap(), &4);
        assert_eq!(it.size_hint(), (1, Some(1)));
        assert_eq!(it.next_back().unwrap(), &5);
        assert_eq!(it.next_back(), None);
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_rev_iter() {
        let m = generate_test();
        for (i, elt) in m.iter().rev().enumerate() {
            assert_eq!(6 - i as i32, *elt);
        }
        let mut n = LinkedList::new();
        assert_eq!(n.iter().rev().next(), None);
        n.push_front(4);
        let mut it = n.iter().rev();
        assert_eq!(it.size_hint(), (1, Some(1)));
        assert_eq!(it.next().unwrap(), &4);
        assert_eq!(it.size_hint(), (0, Some(0)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_mut_iter() {
        let mut m = generate_test();
        let mut len = m.len();
        for (i, elt) in m.iter_mut().enumerate() {
            assert_eq!(i as i32, *elt);
            len -= 1;
        }
        assert_eq!(len, 0);
        let mut n = LinkedList::new();
        assert!(n.iter_mut().next().is_none());
        n.push_front(4);
        n.push_back(5);
        let mut it = n.iter_mut();
        assert_eq!(it.size_hint(), (2, Some(2)));
        assert!(it.next().is_some());
        assert!(it.next().is_some());
        assert_eq!(it.size_hint(), (0, Some(0)));
        assert!(it.next().is_none());
    }

    #[test]
    fn test_iterator_mut_double_end() {
        let mut n = LinkedList::new();
        assert!(n.iter_mut().next_back().is_none());
        n.push_front(4);
        n.push_front(5);
        n.push_front(6);
        let mut it = n.iter_mut();
        assert_eq!(it.size_hint(), (3, Some(3)));
        assert_eq!(*it.next().unwrap(), 6);
        assert_eq!(it.size_hint(), (2, Some(2)));
        assert_eq!(*it.next_back().unwrap(), 4);
        assert_eq!(it.size_hint(), (1, Some(1)));
        assert_eq!(*it.next_back().unwrap(), 5);
        assert!(it.next_back().is_none());
        assert!(it.next().is_none());
    }

    #[test]
    fn test_eq() {
        let mut n: LinkedList<u8> = list_from(&[]);
        let mut m = list_from(&[]);
        assert!(n == m);
        n.push_front(1);
        assert!(n != m);
        m.push_back(1);
        assert!(n == m);

        let n = list_from(&[2, 3, 4]);
        let m = list_from(&[1, 2, 3]);
        assert!(n != m);
    }

    #[test]
    fn test_ord() {
        let n = list_from(&[]);
        let m = list_from(&[1, 2, 3]);
        assert!(n < m);
        assert!(m > n);
        assert!(n <= n);
        assert!(n >= n);
    }

    #[test]
    fn test_ord_nan() {
        let nan = 0.0f64 / 0.0;
        let n = list_from(&[nan]);
        let m = list_from(&[nan]);
        assert!(!(n < m));
        assert!(!(n > m));
        assert!(!(n <= m));
        assert!(!(n >= m));

        let n = list_from(&[nan]);
        let one = list_from(&[1.0f64]);
        assert!(!(n < one));
        assert!(!(n > one));
        assert!(!(n <= one));
        assert!(!(n >= one));

        let u = list_from(&[1.0f64, 2.0, nan]);
        let v = list_from(&[1.0f64, 2.0, 3.0]);
        assert!(!(u < v));
        assert!(!(u > v));
        assert!(!(u <= v));
        assert!(!(u >= v));

        let s = list_from(&[1.0f64, 2.0, 4.0, 2.0]);
        let t = list_from(&[1.0f64, 2.0, 3.0, 2.0]);
        assert!(!(s < t));
        assert!(s > one);
        assert!(!(s <= one));
        assert!(s >= one);
    }

    #[test]
    fn test_debug() {
        let list: LinkedList<i32> = (0..10).collect();
        assert_eq!(format!("{:?}", list), "[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]");

        let list: LinkedList<&str> = vec!["just", "one", "test", "more"]
            .iter()
            .copied()
            .collect();
        assert_eq!(format!("{:?}", list), r#"["just", "one", "test", "more"]"#);
    }

    #[test]
    fn test_hashmap() {
        // Check that HashMap works with this as a key

        let list1: LinkedList<i32> = (0..10).collect();
        let list2: LinkedList<i32> = (1..11).collect();
        let mut map = std::collections::HashMap::new();

        assert_eq!(map.insert(list1.clone(), "list1"), None);
        assert_eq!(map.insert(list2.clone(), "list2"), None);

        assert_eq!(map.len(), 2);

        assert_eq!(map.get(&list1), Some(&"list1"));
        assert_eq!(map.get(&list2), Some(&"list2"));

        assert_eq!(map.remove(&list1), Some("list1"));
        assert_eq!(map.remove(&list2), Some("list2"));

        assert!(map.is_empty());
    }

    #[test]
    fn test_cursor_move_peek() {
        let mut m: LinkedList<u32> = LinkedList::new();
        m.extend([1, 2, 3, 4, 5, 6]);
        let mut cursor = m.cursor_mut();
        cursor.move_next();
        assert_eq!(cursor.current(), Some(&mut 1));
        assert_eq!(cursor.peek_next(), Some(&mut 2));
        assert_eq!(cursor.peek_prev(), None);
        assert_eq!(cursor.index(), Some(0));
        cursor.move_prev();
        assert_eq!(cursor.current(), None);
        assert_eq!(cursor.peek_next(), Some(&mut 1));
        assert_eq!(cursor.peek_prev(), Some(&mut 6));
        assert_eq!(cursor.index(), None);
        cursor.move_next();
        cursor.move_next();
        assert_eq!(cursor.current(), Some(&mut 2));
        assert_eq!(cursor.peek_next(), Some(&mut 3));
        assert_eq!(cursor.peek_prev(), Some(&mut 1));
        assert_eq!(cursor.index(), Some(1));

        let mut cursor = m.cursor_mut();
        cursor.move_prev();
        assert_eq!(cursor.current(), Some(&mut 6));
        assert_eq!(cursor.peek_next(), None);
        assert_eq!(cursor.peek_prev(), Some(&mut 5));
        assert_eq!(cursor.index(), Some(5));
        cursor.move_next();
        assert_eq!(cursor.current(), None);
        assert_eq!(cursor.peek_next(), Some(&mut 1));
        assert_eq!(cursor.peek_prev(), Some(&mut 6));
        assert_eq!(cursor.index(), None);
        cursor.move_prev();
        cursor.move_prev();
        assert_eq!(cursor.current(), Some(&mut 5));
        assert_eq!(cursor.peek_next(), Some(&mut 6));
        assert_eq!(cursor.peek_prev(), Some(&mut 4));
        assert_eq!(cursor.index(), Some(4));
    }

    #[test]
    fn test_cursor_mut_insert() {
        let mut m: LinkedList<u32> = LinkedList::new();
        m.extend([1, 2, 3, 4, 5, 6]);
        let mut cursor = m.cursor_mut();
        cursor.move_next();
        cursor.splice_before(Some(7).into_iter().collect());
        cursor.splice_after(Some(8).into_iter().collect());
        // check_links(&m);
        assert_eq!(
            m.iter().cloned().collect::<Vec<_>>(),
            &[7, 1, 8, 2, 3, 4, 5, 6]
        );
        let mut cursor = m.cursor_mut();
        cursor.move_next();
        cursor.move_prev();
        cursor.splice_before(Some(9).into_iter().collect());
        cursor.splice_after(Some(10).into_iter().collect());
        check_links(&m);
        assert_eq!(
            m.iter().cloned().collect::<Vec<_>>(),
            &[10, 7, 1, 8, 2, 3, 4, 5, 6, 9]
        );

        /* remove_current not impl'd
        let mut cursor = m.cursor_mut();
        cursor.move_next();
        cursor.move_prev();
        assert_eq!(cursor.remove_current(), None);
        cursor.move_next();
        cursor.move_next();
        assert_eq!(cursor.remove_current(), Some(7));
        cursor.move_prev();
        cursor.move_prev();
        cursor.move_prev();
        assert_eq!(cursor.remove_current(), Some(9));
        cursor.move_next();
        assert_eq!(cursor.remove_current(), Some(10));
        check_links(&m);
        assert_eq!(m.iter().cloned().collect::<Vec<_>>(), &[1, 8, 2, 3, 4, 5, 6]);
        */

        let mut m: LinkedList<u32> = LinkedList::new();
        m.extend([1, 8, 2, 3, 4, 5, 6]);
        let mut cursor = m.cursor_mut();
        cursor.move_next();
        let mut p: LinkedList<u32> = LinkedList::new();
        p.extend([100, 101, 102, 103]);
        let mut q: LinkedList<u32> = LinkedList::new();
        q.extend([200, 201, 202, 203]);
        cursor.splice_after(p);
        cursor.splice_before(q);
        check_links(&m);
        assert_eq!(
            m.iter().cloned().collect::<Vec<_>>(),
            &[200, 201, 202, 203, 1, 100, 101, 102, 103, 8, 2, 3, 4, 5, 6]
        );
        let mut cursor = m.cursor_mut();
        cursor.move_next();
        cursor.move_prev();
        let tmp = cursor.split_before();
        let expected: &[u32] = &[];
        assert_eq!(m.into_iter().collect::<Vec<u32>>(), expected);
        m = tmp;
        let mut cursor = m.cursor_mut();
        cursor.move_next();
        cursor.move_next();
        cursor.move_next();
        cursor.move_next();
        cursor.move_next();
        cursor.move_next();
        cursor.move_next();
        let tmp = cursor.split_after();
        assert_eq!(
            tmp.into_iter().collect::<Vec<_>>(),
            &[102, 103, 8, 2, 3, 4, 5, 6]
        );
        check_links(&m);
        assert_eq!(
            m.iter().cloned().collect::<Vec<_>>(),
            &[200, 201, 202, 203, 1, 100, 101]
        );
    }

    fn check_links<T: Eq + std::fmt::Debug>(list: &LinkedList<T>) {
        let from_front: Vec<_> = list.iter().collect();
        let from_back: Vec<_> = list.iter().rev().collect();
        let re_reved: Vec<_> = from_back.into_iter().rev().collect();

        assert_eq!(from_front, re_reved);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serialization() {
        let linked_list: LinkedList<bool> = LinkedList::new();
        let serialized = serde_json::to_string(&linked_list).unwrap();
        let unserialized: LinkedList<bool> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(linked_list, unserialized);

        let bools = vec![true, false, true, true];
        let linked_list: LinkedList<bool> = bools.iter().map(|n| *n).collect();
        let serialized = serde_json::to_string(&linked_list).unwrap();
        let unserialized: LinkedList<bool> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(linked_list, unserialized);
    }

    #[cfg(feature = "miniserde")]
    #[test]
    fn test_miniserde_serialization() {
        let linked_list: LinkedList<bool> = LinkedList::new();
        let serialized = miniserde::json::to_string(&linked_list);
        let unserialized: LinkedList<bool> = miniserde::json::from_str(&serialized[..]).unwrap();
        assert_eq!(linked_list, unserialized);

        let bools = vec![true, false, true, true];
        let linked_list: LinkedList<bool> = bools.iter().map(|n| *n).collect();
        let serialized = miniserde::json::to_string(&linked_list);
        let unserialized: LinkedList<bool> = miniserde::json::from_str(&serialized[..]).unwrap();
        assert_eq!(linked_list, unserialized);
    }

    #[cfg(feature = "nanoserde")]
    #[test]
    fn test_nanoserde_json_serialization() {
        use nanoserde::{DeJson, SerJson};

        let linked_list: LinkedList<bool> = LinkedList::new();
        let serialized = linked_list.serialize_json();
        let unserialized: LinkedList<bool> = LinkedList::deserialize_json(&serialized[..]).unwrap();
        assert_eq!(linked_list, unserialized);

        let bools = vec![true, false, true, true];
        let linked_list: LinkedList<bool> = bools.iter().map(|n| *n).collect();
        let serialized = linked_list.serialize_json();
        let unserialized: LinkedList<bool> = LinkedList::deserialize_json(&serialized[..]).unwrap();
        assert_eq!(linked_list, unserialized);
    }

    #[cfg(feature = "borsh")]
    #[test]
    fn test_borsh_serialization() {
        let linked_list: LinkedList<bool> = LinkedList::new();
        let serialized = borsh::to_vec(&linked_list).unwrap();
        let unserialized: LinkedList<bool> = borsh::from_slice(&serialized[..]).unwrap();
        assert_eq!(linked_list, unserialized);

        let bools = vec![true, false, true, true];
        let linked_list: LinkedList<bool> = bools.iter().map(|n| *n).collect();
        let serialized = borsh::to_vec(&linked_list).unwrap();
        let unserialized: LinkedList<bool> = borsh::from_slice(&serialized[..]).unwrap();
        assert_eq!(linked_list, unserialized);
    }
}
