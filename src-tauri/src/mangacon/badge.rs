use serde::{Deserialize, Serialize};

const DEFAULT_REQUIRED_EQUAL_SAMPLES: usize = 3;
const NORMALIZATION_GRID: i32 = 4;
const MIN_RED_BADGE_PIXELS: usize = 32;
const MIN_BADGE_DIAMETER: i32 = 7;
const MAX_BADGE_DIAMETER: i32 = 36;

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

pub fn detect_badge_points_from_rgba(width: usize, height: usize, rgba: &[u8]) -> BadgeSample {
    if width == 0 || height == 0 || rgba.len() != width * height * 4 {
        return BadgeSample { badges: Vec::new() };
    }

    let mut visited = vec![false; width * height];
    let mut badges = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            if visited[index] || !is_badge_red_pixel(rgba, index) {
                visited[index] = true;
                continue;
            }

            if let Some(component) = collect_red_component(width, height, rgba, &mut visited, x, y)
            {
                badges.push(component.center());
            }
        }
    }

    badges.sort();
    badges.dedup();
    BadgeSample { badges }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RedComponent {
    min_x: usize,
    max_x: usize,
    min_y: usize,
    max_y: usize,
    sum_x: usize,
    sum_y: usize,
    pixels: usize,
}

impl RedComponent {
    fn new(x: usize, y: usize) -> Self {
        Self {
            min_x: x,
            max_x: x,
            min_y: y,
            max_y: y,
            sum_x: 0,
            sum_y: 0,
            pixels: 0,
        }
    }

    fn add(&mut self, x: usize, y: usize) {
        self.min_x = self.min_x.min(x);
        self.max_x = self.max_x.max(x);
        self.min_y = self.min_y.min(y);
        self.max_y = self.max_y.max(y);
        self.sum_x += x;
        self.sum_y += y;
        self.pixels += 1;
    }

    fn is_badge_sized(&self) -> bool {
        let width = (self.max_x - self.min_x + 1) as i32;
        let height = (self.max_y - self.min_y + 1) as i32;

        self.pixels >= MIN_RED_BADGE_PIXELS
            && (MIN_BADGE_DIAMETER..=MAX_BADGE_DIAMETER).contains(&width)
            && (MIN_BADGE_DIAMETER..=MAX_BADGE_DIAMETER).contains(&height)
            && (width - height).abs() <= width.max(height) / 2
    }

    fn center(&self) -> BadgePoint {
        BadgePoint {
            x: (self.sum_x / self.pixels) as i32,
            y: (self.sum_y / self.pixels) as i32,
        }
    }
}

fn collect_red_component(
    width: usize,
    height: usize,
    rgba: &[u8],
    visited: &mut [bool],
    start_x: usize,
    start_y: usize,
) -> Option<RedComponent> {
    let mut stack = vec![(start_x, start_y)];
    let mut component = RedComponent::new(start_x, start_y);

    while let Some((x, y)) = stack.pop() {
        let index = y * width + x;
        if visited[index] {
            continue;
        }

        visited[index] = true;
        if !is_badge_red_pixel(rgba, index) {
            continue;
        }

        component.add(x, y);

        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < width {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < height {
            stack.push((x, y + 1));
        }
    }

    component.is_badge_sized().then_some(component)
}

fn is_badge_red_pixel(rgba: &[u8], pixel_index: usize) -> bool {
    let offset = pixel_index * 4;
    let red = rgba[offset];
    let green = rgba[offset + 1];
    let blue = rgba[offset + 2];
    let alpha = rgba[offset + 3];

    alpha > 120
        && red >= 180
        && green <= 110
        && blue <= 130
        && red.saturating_sub(green) >= 70
        && red.saturating_sub(blue) >= 55
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

    #[test]
    fn detects_red_badge_points_from_rgba_pixels() {
        let width = 96;
        let height = 72;
        let mut rgba = vec![255_u8; width * height * 4];

        draw_red_badge(&mut rgba, width, 24, 18, 7);
        draw_red_badge(&mut rgba, width, 70, 48, 6);
        set_pixel(&mut rgba, width, 5, 5, [235, 48, 66, 255]);

        let sample = detect_badge_points_from_rgba(width, height, &rgba);

        assert_eq!(sample, BadgeSample::from_points([(24, 18), (70, 48)]));
    }

    #[test]
    fn rejects_rgba_buffers_with_unexpected_length() {
        let sample = detect_badge_points_from_rgba(10, 10, &[255, 0, 0, 255]);

        assert_eq!(sample.badges, Vec::<BadgePoint>::new());
    }

    fn draw_red_badge(rgba: &mut [u8], width: usize, cx: usize, cy: usize, radius: usize) {
        let radius_squared = (radius * radius) as isize;
        for y in cy.saturating_sub(radius)..=cy + radius {
            for x in cx.saturating_sub(radius)..=cx + radius {
                let dx = x as isize - cx as isize;
                let dy = y as isize - cy as isize;
                if dx * dx + dy * dy <= radius_squared {
                    set_pixel(rgba, width, x, y, [235, 48, 66, 255]);
                }
            }
        }

        set_pixel(rgba, width, cx, cy, [255, 255, 255, 255]);
    }

    fn set_pixel(rgba: &mut [u8], width: usize, x: usize, y: usize, pixel: [u8; 4]) {
        let offset = (y * width + x) * 4;
        rgba[offset..offset + 4].copy_from_slice(&pixel);
    }
}
