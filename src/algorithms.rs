//! Generic parent-child hierarchy lookup algorithm.
//!
//! Ported and enhanced from `PinWin/Algorithms/ParentChildHierarchyLookup.cs`
//! with cycle/depth protection so corrupt window trees cannot cause infinite loops.

/// Maximum traversal depth to guard against cyclic window parent/owner chains.
pub const MAX_HIERARCHY_DEPTH: usize = 256;

/// Trait defining a parent-child hierarchy lookup strategy.
pub trait HierarchyLookup<T: Copy + PartialEq> {
    /// Retrieve the parent element for a given child element.
    fn get_parent_element(&self, child: T) -> T;

    /// Return `true` when the traversal should stop (meaning `child` is the root parent).
    fn stop_condition(&self, parent: T) -> bool;

    /// Walk up the hierarchy from `child` until `stop_condition(parent)` holds
    /// or `MAX_HIERARCHY_DEPTH` / a self-cycle is reached.
    fn find_parent(&self, mut child: T) -> T {
        for _ in 0..MAX_HIERARCHY_DEPTH {
            let parent = self.get_parent_element(child);
            if self.stop_condition(parent) || parent == child {
                return child;
            }
            child = parent;
        }
        child
    }
}

/// Closure-backed hierarchy lookup for flexible use and unit testing.
pub struct FnHierarchyLookup<FGet, FStop> {
    get_parent_fn: FGet,
    stop_fn: FStop,
}

impl<FGet, FStop> FnHierarchyLookup<FGet, FStop> {
    pub const fn new(get_parent_fn: FGet, stop_fn: FStop) -> Self {
        Self {
            get_parent_fn,
            stop_fn,
        }
    }
}

impl<T, FGet, FStop> HierarchyLookup<T> for FnHierarchyLookup<FGet, FStop>
where
    T: Copy + PartialEq,
    FGet: Fn(T) -> T,
    FStop: Fn(T) -> bool,
{
    fn get_parent_element(&self, child: T) -> T {
        (self.get_parent_fn)(child)
    }

    fn stop_condition(&self, parent: T) -> bool {
        (self.stop_fn)(parent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_parent_self_is_root() {
        let lookup = FnHierarchyLookup::new(|_id: usize| 0, |parent| parent == 0);
        assert_eq!(lookup.find_parent(42), 42);
    }

    #[test]
    fn test_find_parent_multi_level_chain() {
        // 4 -> 3 -> 2 -> 1 -> 0 (stop)
        let parents = [0usize, 0, 1, 2, 3];
        let lookup = FnHierarchyLookup::new(
            |node: usize| parents.get(node).copied().unwrap_or(0),
            |parent| parent == 0,
        );
        assert_eq!(lookup.find_parent(4), 1);
        assert_eq!(lookup.find_parent(2), 1);
        assert_eq!(lookup.find_parent(1), 1);
    }

    #[test]
    fn test_find_parent_terminates_on_cycle() {
        // 1 -> 2 -> 1 (cycle)
        let lookup = FnHierarchyLookup::new(
            |node: usize| if node == 1 { 2 } else { 1 },
            |parent| parent == 0,
        );
        let result = lookup.find_parent(1);
        assert!(result == 1 || result == 2);
    }
}
