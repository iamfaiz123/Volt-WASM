/// A unique identifier for a spawned task.
///
/// Contains an index into the arena and a generation counter to prevent
/// the ABA problem (stale wakers waking a reused slot).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskId {
    pub(crate) index: usize,
    pub(crate) generation: u64,
}

/// A slot in the generational arena.
struct Slot<T> {
    /// The value, if occupied.
    value: Option<T>,
    /// The generation of this slot. Incremented when the slot is reused.
    generation: u64,
}

/// A generational arena allocator.
///
/// Stores values in a contiguous array for cache locality and fast access.
/// Uses generation counters to provide safe identifiers that detect ABA
/// (reuse of a slot).
pub struct GenerationalArena<T> {
    slots: Vec<Slot<T>>,
    free_list: Vec<usize>,
    count: usize,
}

impl<T> GenerationalArena<T> {
    /// Create a new, empty arena.
    pub fn new() -> Self {
        GenerationalArena {
            slots: Vec::new(),
            free_list: Vec::new(),
            count: 0,
        }
    }

    /// Insert a value into the arena, returning its unique `TaskId`.
    pub fn insert(&mut self, value: T) -> TaskId {
        self.count += 1;
        if let Some(index) = self.free_list.pop() {
            let slot = &mut self.slots[index];
            slot.value = Some(value);
            // Generation was already incremented when the slot was removed
            TaskId {
                index,
                generation: slot.generation,
            }
        } else {
            let index = self.slots.len();
            self.slots.push(Slot {
                value: Some(value),
                generation: 0,
            });
            TaskId {
                index,
                generation: 0,
            }
        }
    }

    /// Retrieve a mutable reference to the value associated with `id`.
    /// Returns `None` if the slot is empty or the generation doesn't match.
    pub fn get_mut(&mut self, id: TaskId) -> Option<&mut T> {
        if let Some(slot) = self.slots.get_mut(id.index) {
            if slot.generation == id.generation {
                return slot.value.as_mut();
            }
        }
        None
    }

    /// Remove the value associated with `id`.
    /// Returns `true` if a value was removed, `false` if it was empty or mismatched.
    pub fn remove(&mut self, id: TaskId) -> bool {
        if let Some(slot) = self.slots.get_mut(id.index) {
            if slot.generation == id.generation && slot.value.is_some() {
                slot.value = None;
                slot.generation = slot.generation.wrapping_add(1);
                self.free_list.push(id.index);
                self.count -= 1;
                return true;
            }
        }
        false
    }

    /// Check if the arena contains a valid value for `id`.
    pub fn contains(&self, id: TaskId) -> bool {
        if let Some(slot) = self.slots.get(id.index) {
            return slot.generation == id.generation && slot.value.is_some();
        }
        false
    }

    /// Return the number of active elements in the arena.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Return `true` if the arena is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl<T> Default for GenerationalArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut arena = GenerationalArena::new();
        let id = arena.insert(42);
        assert_eq!(arena.get_mut(id), Some(&mut 42));
    }

    #[test]
    fn test_remove() {
        let mut arena = GenerationalArena::new();
        let id = arena.insert(42);
        assert!(arena.remove(id));
        assert_eq!(arena.get_mut(id), None);
        assert!(!arena.contains(id));
    }

    #[test]
    fn test_aba_prevention() {
        let mut arena = GenerationalArena::new();
        let id1 = arena.insert("first");
        arena.remove(id1);

        let id2 = arena.insert("second");
        // They should reuse the same slot but have different generations
        assert_eq!(id1.index, id2.index);
        assert_ne!(id1.generation, id2.generation);

        // Attempting to access via old ID should fail
        assert_eq!(arena.get_mut(id1), None);
        assert_eq!(arena.get_mut(id2), Some(&mut "second"));
    }
}
