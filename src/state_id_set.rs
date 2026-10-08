use std::rc::Rc;

use crate::dfa::StateId;

// `StateIdSet` is used as a `RangeMapBlaze` value, and range-set-blaze clones values whenever it
// splits a range, so cloning must be cheap. The `Rc` makes a clone a reference-count bump;
// `insert` copies the vector only when it is shared (copy-on-write via `Rc::make_mut`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct StateIdSet {
    active: Rc<Vec<bool>>,
}

impl StateIdSet {
    pub(crate) fn new() -> Self {
        Self {
            active: Rc::new(Vec::new()),
        }
    }

    pub(crate) fn from_state(state: StateId) -> Self {
        Self::new().with_inserted(state)
    }

    pub(crate) fn insert(&mut self, state: StateId) {
        let active = Rc::make_mut(&mut self.active);
        if state.id() >= active.len() {
            active.resize(state.id() + 1, false);
        }
        active[state.id()] = true;
    }

    pub(crate) fn with_inserted(&self, state: StateId) -> Self {
        let mut next = self.clone();
        next.insert(state);
        next
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = StateId> + '_ {
        self.active
            .iter()
            .enumerate()
            .filter_map(|(id, is_active)| {
                if *is_active {
                    Some(StateId::from_id(id))
                } else {
                    None
                }
            })
    }
}
