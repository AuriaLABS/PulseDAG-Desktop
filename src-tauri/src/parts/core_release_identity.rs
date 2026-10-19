#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CoreReleaseFreezeState {
    Frozen,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CoreReleaseIdentity {
    pub(crate) release_tag: &'static str,
    pub(crate) source_commit: Option<&'static str>,
    pub(crate) source_tree: Option<&'static str>,
    pub(crate) freeze_state: CoreReleaseFreezeState,
}

impl CoreReleaseIdentity {
    pub(crate) fn is_frozen(self) -> bool {
        self.freeze_state == CoreReleaseFreezeState::Frozen
            && self.source_commit.is_some()
            && self.source_tree.is_some()
    }

    pub(crate) fn accepts_source(self, release_tag: &str, source_commit: &str) -> bool {
        self.is_frozen()
            && self.release_tag == release_tag
            && self.source_commit == Some(source_commit)
    }

    pub(crate) fn accepts_exact_source(
        self,
        release_tag: &str,
        source_commit: &str,
        source_tree: &str,
    ) -> bool {
        self.accepts_source(release_tag, source_commit) && self.source_tree == Some(source_tree)
    }
}

pub(crate) const V2_4_CORE_RELEASE_IDENTITY: CoreReleaseIdentity = CoreReleaseIdentity {
    release_tag: V2_4_FINAL_RELEASE_TAG,
    source_commit: Some(V2_4_FINAL_RELEASE_SOURCE_COMMIT),
    source_tree: Some(V2_4_FINAL_RELEASE_SOURCE_TREE),
    freeze_state: CoreReleaseFreezeState::Frozen,
};

pub(crate) const V3_0_TARGET_RELEASE_TAG: &str = "v3.0.0";

pub(crate) const V3_0_CORE_RELEASE_IDENTITY: CoreReleaseIdentity = CoreReleaseIdentity {
    release_tag: V3_0_TARGET_RELEASE_TAG,
    source_commit: None,
    source_tree: None,
    freeze_state: CoreReleaseFreezeState::Pending,
};

#[cfg(test)]
mod core_release_identity_tests {
    use super::*;

    #[test]
    fn historical_v2_4_release_identity_remains_frozen_and_exact() {
        assert!(V2_4_CORE_RELEASE_IDENTITY.is_frozen());
        assert!(V2_4_CORE_RELEASE_IDENTITY.accepts_exact_source(
            "v2.4.0",
            "876b48826a3875b729888edb88e2b0eea15bb717",
            "f41f65bc5c5da3a44903b84f0e0f7186df2b64a8",
        ));
        assert!(!V2_4_CORE_RELEASE_IDENTITY.accepts_source(
            "v2.4.0",
            "995b7b200afc90f705eece6c37a16b7a4fc294ec",
        ));
        assert!(!V2_4_CORE_RELEASE_IDENTITY.accepts_source(
            "v3.0.0",
            "876b48826a3875b729888edb88e2b0eea15bb717",
        ));
    }

    #[test]
    fn v3_target_is_fail_closed_until_core_freezes_exact_source_identity() {
        assert_eq!(V3_0_CORE_RELEASE_IDENTITY.release_tag, "v3.0.0");
        assert_eq!(V3_0_CORE_RELEASE_IDENTITY.freeze_state, CoreReleaseFreezeState::Pending);
        assert!(V3_0_CORE_RELEASE_IDENTITY.source_commit.is_none());
        assert!(V3_0_CORE_RELEASE_IDENTITY.source_tree.is_none());
        assert!(!V3_0_CORE_RELEASE_IDENTITY.is_frozen());

        for candidate in [
            "876b48826a3875b729888edb88e2b0eea15bb717",
            "0000000000000000000000000000000000000000",
        ] {
            assert!(!V3_0_CORE_RELEASE_IDENTITY.accepts_source("v3.0.0", candidate));
        }
    }
}
