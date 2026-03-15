use crate::dfa::StateId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct StateIdSet {
    active: Vec<bool>,
}

impl StateIdSet {
    pub(crate) fn new() -> Self {
        Self { active: Vec::new() }
    }

    pub(crate) fn from_state(state: StateId) -> Self {
        Self::new().with_inserted(state)
    }

    pub(crate) fn insert(&mut self, state: StateId) {
        if state.id() >= self.active.len() {
            self.active.resize(state.id() + 1, false);
        }
        self.active[state.id()] = true;
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
