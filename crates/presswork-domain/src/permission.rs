pub use presswork_api::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ReadProjects,
    ManageProjects,
    InvitePeople,
    WriteTasks,
    DeleteTask { own: bool },
}

#[must_use]
pub fn allowed(role: Role, action: Action) -> bool {
    match (role, action) {
        (Role::Administrator, _) | (Role::Member, Action::ReadProjects | Action::WriteTasks) => {
            true
        }
        (Role::Member, Action::DeleteTask { own }) => own,
        (Role::Member, Action::ManageProjects | Action::InvitePeople) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn administrators_can_do_all_operations() {
        for action in [
            Action::ReadProjects,
            Action::ManageProjects,
            Action::InvitePeople,
            Action::WriteTasks,
            Action::DeleteTask { own: false },
            Action::DeleteTask { own: true },
        ] {
            assert!(allowed(Role::Administrator, action), "{action:?}");
        }
    }

    #[test]
    fn members_do_the_daily_work() {
        assert!(allowed(Role::Member, Action::ReadProjects));
        assert!(allowed(Role::Member, Action::WriteTasks));
        assert!(allowed(Role::Member, Action::DeleteTask { own: true }));
    }

    #[test]
    fn members_cannot_change_the_structure_or_delete_the_tasks_of_others() {
        assert!(!allowed(Role::Member, Action::ManageProjects));
        assert!(!allowed(Role::Member, Action::InvitePeople));
        assert!(!allowed(Role::Member, Action::DeleteTask { own: false }));
    }
}
