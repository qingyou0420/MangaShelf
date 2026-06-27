use serde::{Deserialize, Serialize};

const DEFAULT_REQUIRED_EQUAL_SAMPLES: usize = 3;
const NORMALIZATION_GRID: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgePoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeSample {
    pub badges: Vec<BadgePoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeDetection {
    pub is_stable: bool,
    pub detected_badges: usize,
    pub stable_samples: usize,
    pub normalized_badges: Vec<BadgePoint>,
}

impl BadgeSample {
    pub fn from_points(points: impl IntoIterator<Item = (i32, i32)>) -> Self {
        Self {
            badges: points
                .into_iter()
                .map(|(x, y)| BadgePoint { x, y })
                .collect(),
        }
    }
}

pub fn detect_stable_badges(
    samples: &[BadgeSample],
    required_equal_samples: usize,
) -> BadgeDetection {
    let required_equal_samples = if required_equal_samples == 0 {
        DEFAULT_REQUIRED_EQUAL_SAMPLES
    } else {
        required_equal_samples
    };

    let Some(last_sample) = samples.last() else {
        return BadgeDetection {
            is_stable: false,
            detected_badges: 0,
            stable_samples: 0,
            normalized_badges: Vec::new(),
        };
    };

    let normalized_badges = normalize_sample(last_sample);
    let stable_samples = samples
        .iter()
        .rev()
        .take_while(|sample| normalize_sample(sample) == normalized_badges)
        .count();

    BadgeDetection {
        is_stable: stable_samples >= required_equal_samples,
        detected_badges: normalized_badges.len(),
        stable_samples,
        normalized_badges,
    }
}

fn normalize_sample(sample: &BadgeSample) -> Vec<BadgePoint> {
    let mut badges = sample
        .badges
        .iter()
        .map(|badge| BadgePoint {
            x: normalize_axis(badge.x),
            y: normalize_axis(badge.y),
        })
        .collect::<Vec<_>>();
    badges.sort();
    badges.dedup();
    badges
}

fn normalize_axis(value: i32) -> i32 {
    ((value + (NORMALIZATION_GRID / 2)).div_euclid(NORMALIZATION_GRID)) * NORMALIZATION_GRID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_detection_is_stable_after_required_equal_samples() {
        let samples = vec![
            BadgeSample::from_points([(100, 200)]),
            BadgeSample::from_points([(101, 199)]),
            BadgeSample::from_points([(100, 201)]),
        ];

        let detection = detect_stable_badges(&samples, 3);

        assert!(detection.is_stable);
        assert_eq!(detection.detected_badges, 1);
        assert_eq!(detection.stable_samples, 3);
    }

    #[test]
    fn badge_detection_stays_unstable_for_changing_samples() {
        let samples = vec![
            BadgeSample::from_points([(100, 200)]),
            BadgeSample::from_points([(150, 240)]),
            BadgeSample::from_points([(100, 200)]),
        ];

        let detection = detect_stable_badges(&samples, 3);

        assert!(!detection.is_stable);
        assert_eq!(detection.stable_samples, 1);
    }
}
