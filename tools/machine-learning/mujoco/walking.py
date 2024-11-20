#!/usr/bin/env python3

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

import matplotlib.pyplot as plt
import numpy as np


@dataclass
class Parameters:
    sole_pressure_threshold: float
    min_step_duration: float
    step_duration: float
    foot_lift_apex: float


class Side(Enum):
    LEFT = 0
    RIGHT = 1


@dataclass
class Feet:
    support_sole: Pose2
    swing_sole: Pose2


@dataclass
class Pose2:
    x: float = 0.0
    y: float = 0.0
    theta: float = 0.0

    def __add__(self, other: Pose2) -> Pose2:
        return Pose2(
            x=self.x + other.x,
            y=self.y + other.y,
            theta=self.theta + other.theta,
        )

    def __mul__(self, other: float) -> Pose2:
        return Pose2(
            x=self.x * other,
            y=self.y * other,
            theta=self.theta * other,
        )

    def __sub__(self, other: Pose2) -> Pose2:
        return Pose2(
            x=self.x - other.x,
            y=self.y - other.y,
            theta=self.theta - other.theta,
        )


@dataclass
class State:
    t: float
    support_side: Side
    start_feet: Feet
    end_feet: Feet


@dataclass
class Measurements:
    pressure_left: float
    pressure_right: float


@dataclass
class Control:
    forward: float
    left: float
    turn: float


def is_support_switched(
    x: State,
    i: Measurements,
    parameters: Parameters,
) -> bool:
    if x.t < parameters.min_step_duration:
        return False

    if x.support_side == Side.LEFT:
        return i.pressure_right > parameters.sole_pressure_threshold
    else:
        return i.pressure_left > parameters.sole_pressure_threshold


def end_feet_from_request(
    u: Control,
) -> Feet:
    support_sole = Pose2(
        x=-u.forward / 2.0,
        y=-u.left / 2.0,
        theta=-u.turn / 2.0,
    )
    swing_sole = Pose2(
        x=u.forward / 2.0,
        y=u.left / 2.0,
        theta=u.turn / 2.0,
    )
    return Feet(
        support_sole=support_sole,
        swing_sole=swing_sole,
    )


def normalized_time(t: float, parameters: Parameters) -> float:
    return np.clip(t / parameters.step_duration, 0.0, 1.0)


def swing_sole_lift_at(x: State, parameters: Parameters) -> float:
    t = normalized_time(x.t, parameters)
    t = parabolic_return(t)
    return parameters.foot_lift_apex * t


def lerp(t: float, start: Pose2, end: Pose2):
    return start + (end - start) * t


def swing_sole_position_at(x: State, parameters: Parameters) -> Pose2:
    t = normalized_time(x.t, parameters)
    t = parabolic_step(t)
    return lerp(t, x.start_feet.swing_sole, x.end_feet.swing_sole)


def support_sole_position_at(x: State, parameters: Parameters) -> Pose2:
    t = normalized_time(x.t, parameters)
    return lerp(t, x.start_feet.support_sole, x.end_feet.support_sole)


def compute_feet(x: State, parameters: Parameters) -> tuple[Feet, float]:
    swing_sole_lift = swing_sole_lift_at(x, parameters)
    swing_sole_position = swing_sole_position_at(x, parameters)
    support_sole_position = support_sole_position_at(x, parameters)
    return (
        Feet(
            support_sole=support_sole_position,
            swing_sole=swing_sole_position,
        ),
        swing_sole_lift,
    )


def step(
    x: State,
    i: Measurements,
    u: Control,
    dt: float,
    parameters: Parameters,
) -> tuple[State, Pose2, float, Pose2, float]:
    if is_support_switched(x, i, parameters):
        x.t = 0.0
        if x.support_side == Side.LEFT:
            x.support_side = Side.RIGHT
        else:
            x.support_side = Side.LEFT
        x.end_feet = end_feet_from_request(u)
    x.t += dt

    (feet, lift) = compute_feet(x, parameters)

    if x.support_side == Side.LEFT:
        return (
            x,
            feet.support_sole,
            0.0,
            feet.swing_sole,
            lift,
        )
    else:
        return (
            x,
            feet.swing_sole,
            lift,
            feet.support_sole,
            0.0,
        )


def parabolic_return(x: float, midpoint: float = 0.5) -> float:
    if x < midpoint:
        return -2.0 / (midpoint**3) * (x**3) + 3.0 / (midpoint**2) * (x**2)
    else:
        return (
            -1.0
            / ((midpoint - 1.0) ** 3)
            * (2.0 * (x**3) - 3.0 * (midpoint + 1.0) * (x**2) + 6.0 * midpoint * x - 3.0 * midpoint + 1.0)
        )


def parabolic_step(x: float) -> float:
    if x < 0.5:
        return 2.0 * x * x
    else:
        return 4.0 * x - 2.0 * x * x - 1.0


if __name__ == "__main__":
    parameters = Parameters(
        sole_pressure_threshold=0.5,
        min_step_duration=0.1,
        step_duration=0.25,
        foot_lift_apex=0.015,
    )
    state = State(
        t=1.0,
        support_side=Side.LEFT,
        start_feet=Feet(
            support_sole=Pose2(),
            swing_sole=Pose2(),
        ),
        end_feet=Feet(
            support_sole=Pose2(),
            swing_sole=Pose2(),
        ),
    )
    measurements = Measurements(
        pressure_left=0.0,
        pressure_right=1.0,
    )
    control = Control(
        forward=0.06,
        left=0.0,
        turn=0.0,
    )
    dt = 0.012
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")

    odometry = Pose2()
    last_left_sole = Pose2()
    last_right_sole = Pose2()

    for _ in range(100):
        state, left_sole, left_lift, right_sole, right_lift = step(
            state,
            measurements,
            control,
            dt,
            parameters,
        )

        moved = left_sole - last_left_sole if state.support_side == Side.LEFT else right_sole - last_right_sole
        odometry -= moved
        last_left_sole = left_sole
        last_right_sole = right_sole

        if left_lift < 0.001:
            measurements.pressure_left = 1.0
        else:
            measurements.pressure_left = 0.0

        if right_lift < 0.001:
            measurements.pressure_right = 1.0
        else:
            measurements.pressure_right = 0.0

        ax.plot(
            left_sole.x + odometry.x,
            left_sole.y + 0.02 + odometry.y,
            left_lift,
            "ro",
        )
        ax.plot(
            right_sole.x + odometry.x,
            right_sole.y - 0.02 + odometry.y,
            right_lift,
            "bo",
        )

    plt.show()
