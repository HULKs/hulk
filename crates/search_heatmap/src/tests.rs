use super::*;

#[test]
fn high_mean_priority_is_scaled_down_without_changing_relative_values() {
    let mut heatmap = Heatmap::new(FieldDimensions::SPL_2025);
    heatmap.map.fill(1.0);
    heatmap.map[(0, 0)] = 0.5;

    heatmap.limit_mean_priority(0.5);

    let mean = heatmap.map.sum() / heatmap.map.len() as f32;
    assert!((mean - 0.5).abs() < 1e-6);
    assert!((heatmap.map[(1, 1)] / heatmap.map[(0, 0)] - 2.0).abs() < 1e-6);
}

#[test]
fn low_mean_priority_is_preserved() {
    let mut heatmap = Heatmap::new(FieldDimensions::SPL_2025);
    heatmap.map.fill(0.2);

    heatmap.limit_mean_priority(0.5);

    assert!(heatmap.map.iter().all(|value| *value == 0.2));
}
