use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncAction {
    #[serde(rename = "skip")]
    Skip,
    #[serde(rename = "copy_a_to_b")]
    CopyAtoB,
    #[serde(rename = "copy_b_to_a")]
    CopyBtoA,
    #[serde(rename = "delete_a")]
    DeleteA,
    #[serde(rename = "delete_b")]
    DeleteB,
    #[serde(rename = "delete_both")]
    DeleteBoth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncTask {
    pub relative_path: String,
    pub action: SyncAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SyncResult {
    pub completed: usize,
    pub total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_action_serde_roundtrip() {
        let cases = [
            (SyncAction::Skip, "\"skip\""),
            (SyncAction::CopyAtoB, "\"copy_a_to_b\""),
            (SyncAction::CopyBtoA, "\"copy_b_to_a\""),
            (SyncAction::DeleteA, "\"delete_a\""),
            (SyncAction::DeleteB, "\"delete_b\""),
            (SyncAction::DeleteBoth, "\"delete_both\""),
        ];

        for (action, expected_json) in cases {
            // Test serialization
            let serialized = serde_json::to_string(&action).unwrap();
            assert_eq!(serialized, expected_json);

            // Test deserialization
            let deserialized: SyncAction = serde_json::from_str(expected_json).unwrap();
            assert_eq!(deserialized, action);
        }
    }

    #[test]
    fn test_sync_task_serde() {
        let json_input = r#"{"relative_path":"test/file.txt","action":"copy_a_to_b"}"#;
        let task: SyncTask = serde_json::from_str(json_input).unwrap();
        assert_eq!(task.relative_path, "test/file.txt");
        assert_eq!(task.action, SyncAction::CopyAtoB);
    }
}
