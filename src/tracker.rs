use std::ops::{Deref, DerefMut};
use take_mut;

/// Wraps an internal value, and keeps track of whether this value has
/// been modified or not.
///
/// Internally uses `Deref`/`DerefMut` to make sure you can use the
/// wrapped value as usual, but any use of `DerefMut` will change the
/// internal state. T has to be Unpin since the internal crate this
/// library uses, `take_mut`, uses ptr::read, and even though this
/// *shouldn't* change the location of the inner value, I'm not
/// risking it.
#[derive(Debug)]
pub enum Tracked<T: Unpin> {
    Unmodified(T),
    Modified(T),
}

impl<T: Unpin> Tracked<T> {
    /// Construct a new Tracked set to unmodified
    /// ```
    /// use tracked::Tracked;
    ///
    /// let tracker = Tracked::new(5);
    ///
    /// assert!(tracker.is_unmodified());
    /// ```
    pub fn new(x: T) -> Self {
        Tracked::Unmodified(x)
    }

    /// Returns whether this value is unmodified
    /// ```
    /// use tracked::Tracked;
    ///
    /// let mut tracker = Tracked::new(5);
    /// assert!(tracker.is_unmodified());
    /// *tracker = 4;
    /// assert!(!tracker.is_unmodified());
    /// ```
    pub fn is_unmodified(&self) -> bool {
        if let Tracked::Unmodified(_) = self {
            true
        } else {
            false
        }
    }

    /// Returns whether this value has been modified
    /// ```
    /// use tracked::Tracked;
    ///
    /// let mut tracker = Tracked::new(5);
    /// *tracker = 4;
    /// assert!(tracker.is_modified());
    /// ```
    pub fn is_modified(&self) -> bool {
        if let Tracked::Modified(_) = self {
            true
        } else {
            false
        }
    }

    /// Reset this tracker to an unmodified state
    /// ```
    /// use tracked::Tracked;
    ///
    /// let mut tracker = Tracked::new(5);
    ///
    /// *tracker = 4;
    /// assert!(tracker.is_modified());
    ///
    /// tracker.reset();
    /// assert!(tracker.is_unmodified());
    /// ```
    pub(crate) fn reset(&mut self) -> bool {
        let mut did_something = false;
        take_mut::take(self, |tracker| match tracker {
            Tracked::Modified(inner) => {
                did_something = true;
                Tracked::Unmodified(inner)
            }
            x => x,
        });
        did_something
    }

    pub fn into_inner(self) -> T {
        match self {
            Tracked::Modified(x) => x,
            Tracked::Unmodified(x) => x,
        }
    }
}

impl<T: Unpin + Clone> Clone for Tracked<T> {
    fn clone(&self) -> Tracked<T> {
        match self {
            Tracked::Unmodified(x) => Tracked::Unmodified(x.clone()),
            Tracked::Modified(x) => Tracked::Modified(x.clone()),
        }
    }
}

impl<T: Unpin + Copy> Copy for Tracked<T> {}

impl<T: Unpin> Deref for Tracked<T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            &Tracked::Unmodified(ref inner) => inner,
            &Tracked::Modified(ref inner) => inner,
        }
    }
}

impl<T: Unpin> DerefMut for Tracked<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        take_mut::take(self, |unmodified| match unmodified {
            Tracked::Unmodified(inner) => Tracked::Modified(inner),
            modified => modified,
        });
        match self {
            Tracked::Unmodified(inner) => inner,
            Tracked::Modified(inner) => inner,
        }
    }
}
