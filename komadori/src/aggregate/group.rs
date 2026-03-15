/// A group that either exists or not.
///
/// This enum is created by [`GroupMap::group()`](super::GroupMap::group).
/// See its documentation for more.
pub enum Group<
    Occupied: OccupiedGroup,
    Vacant: VacantGroup<Key = Occupied::Key, Value = Occupied::Value>,
> {
    /// A group that exists.
    Occupied(Occupied),

    /// A group that does not yet exist.
    Vacant(Vacant),
}

/// A handle to an existing group.
///
/// This represents a group that is already present in a [`GroupMap`](super::GroupMap),
/// so the group's value can be accessed.
pub trait OccupiedGroup {
    /// The key of the group.
    type Key;

    /// The value of the group.
    type Value;

    /// Returns the key of the group.
    fn key(&self) -> &Self::Key;

    /// Returns a shared reference to the value of the group.
    fn value(&self) -> &Self::Value;

    /// Returns a mutable reference to the value of the group.
    fn value_mut(&mut self) -> &mut Self::Value;
}

/// A handle to a group that does not yet exist.
///
/// This represents a group that is not yet in a [`GroupMap`](super::GroupMap),
/// but can be inserted later.
pub trait VacantGroup {
    /// The key of the group.
    type Key;

    /// The value to be inserted for the group.
    type Value;

    /// Returns the key of the group.
    fn key(&self) -> &Self::Key;

    /// Makes the group exist with the given value.
    fn insert(self, value: Self::Value);
}
