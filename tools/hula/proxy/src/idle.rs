use std::{
    f32::consts::PI,
    io::{BufWriter, Write},
    os::unix::net::UnixStream,
    time::UNIX_EPOCH,
};

use color_eyre::eyre::{Result, WrapErr};
use hula_types::{Battery, LolaControlFrame};
use rmp_serde::encode::write_named;

pub fn knight_rider_eyes() -> ([f32; 24], [f32; 24]) {
    let since_epoch = UNIX_EPOCH.elapsed().expect("time ran backwards");
    let interval_from_0_to_1 = since_epoch.subsec_millis() as f32 / 1000.0;
    let position = ((2.0 * PI * interval_from_0_to_1).sin() + 1.0) / 2.0;
    let maximal_distance_from_center = 1.0 / 4.0;

    //     1
    //  0     2
    // 7       3
    //  6     4
    //     5
    let led_positions_left = [
        0.715_482_2,
        0.833_333_3,
        0.951_184_45,
        1.0,
        0.951_184_45,
        0.833_333_3,
        0.715_482_2,
        0.666_666_7,
    ];

    //     0
    //  1     7
    // 2       6
    //  3     5
    //     4
    let led_positions_right = [
        0.166_666_67,
        0.048_815_537,
        0.0,
        0.048_815_537,
        0.166_666_67,
        0.284_517_8,
        0.333_333_34,
        0.284_517_8,
    ];

    let mut intensities_left = [0.0; 24];
    let mut intensities_right = [0.0; 24];

    for (intensity, led_position) in intensities_left.iter_mut().zip(led_positions_left.iter()) {
        let distance = (led_position - position).abs();
        *intensity =
            ((maximal_distance_from_center - distance) / maximal_distance_from_center).max(0.0);
    }
    for (intensity, led_position) in intensities_right.iter_mut().zip(led_positions_right.iter()) {
        let distance = (led_position - position).abs();
        *intensity =
            ((maximal_distance_from_center - distance) / maximal_distance_from_center).max(0.0);
    }

    (intensities_left, intensities_right)
}

pub fn charging_skull(battery: &Battery) -> [f32; 12] {
    //   front
    //    0 11
    //  1     10
    // 2       9
    // 3       8
    //  4     7
    //    5 6
    //   back
    // 6 is beginning, clock-wise
    let led_positions = [
        0.433_628_32,
        0.349_557_52,
        0.274_336_28,
        0.168_141_59,
        0.088_495_575,
        0.044_247_787,
        0.955_752_2,
        0.911_504_45,
        0.831_858_4,
        0.725_663_7,
        0.650_442_5,
        0.566_371_7,
    ];
    let interval_from_0_to_1 = UNIX_EPOCH
        .elapsed()
        .expect("time ran backwards")
        .subsec_millis() as f32
        / 1000.0;
    let mut skull = [0.0; 12];
    for (led, led_position) in led_positions.into_iter().enumerate() {
        skull[led] = if battery.charge > led_position {
            if battery.current > 0.0 {
                let offsetted_seconds = interval_from_0_to_1 - led_position;
                let fraction = 1.0 - (offsetted_seconds - offsetted_seconds.floor());
                (fraction * 0.8) + 0.2
            } else {
                1.0
            }
        } else {
            0.0
        };
    }
    skull
}

pub fn send_idle(writer: &mut BufWriter<UnixStream>, battery: Option<Battery>) -> Result<()> {
    let mut control_frame = LolaControlFrame::default();
    (control_frame.left_eye, control_frame.right_eye) = knight_rider_eyes();
    if let Some(battery) = &battery {
        control_frame.skull = charging_skull(battery);
    }
    write_named(writer, &control_frame).wrap_err("failed to serialize control message")?;
    writer
        .flush()
        .wrap_err("failed to flush control data to LoLA")?;
    Ok(())
}
