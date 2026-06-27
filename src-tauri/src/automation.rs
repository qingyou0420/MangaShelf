use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationState {
    WaitingRefresh,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRunStatus {
    pub state: AutomationState,
    pub message: String,
    pub detected_badges: usize,
    pub stable_samples: usize,
}

impl AutomationRunStatus {
    pub fn waiting_refresh(detected_badges: usize, stable_samples: usize) -> Self {
        Self {
            state: AutomationState::WaitingRefresh,
            message: "等待漫画控刷新收藏更新...".to_string(),
            detected_badges,
            stable_samples,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_refresh_status_uses_fixed_message_and_badge_counts() {
        let status = AutomationRunStatus::waiting_refresh(2, 3);

        assert_eq!(status.state, AutomationState::WaitingRefresh);
        assert_eq!(status.message, "等待漫画控刷新收藏更新...");
        assert_eq!(status.detected_badges, 2);
        assert_eq!(status.stable_samples, 3);
    }
}
