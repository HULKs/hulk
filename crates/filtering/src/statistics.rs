pub fn mean(data: &[f32]) -> f32 {
    data.iter().sum::<f32>() / data.len() as f32
}

pub fn variance(data: &[f32], mean: f32) -> f32 {
    data.iter()
        .map(|value| {
            let difference = mean - value;
            difference * difference
        })
        .sum::<f32>()
        / data.len() as f32
}

pub fn standard_deviation(data: &[f32], mean: f32) -> f32 {
    variance(data, mean).sqrt()
}
