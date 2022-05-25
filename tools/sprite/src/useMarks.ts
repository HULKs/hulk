import { FieldDimensions } from "./useFieldDimensions";

export type Line = {
  type: "Line";
  point0: [number, number];
  point1: [number, number];
};

export type Circle = {
  type: "Circle";
  center: [number, number];
  radius: number;
};

export type Mark = Line | Circle;

export function useMarks(fieldDimensions: FieldDimensions): Mark[] {
  return [
    {
      type: "Line",
      point0: [-fieldDimensions.length / 2.0, fieldDimensions.width / 2.0],
      point1: [fieldDimensions.length / 2.0, fieldDimensions.width / 2.0],
    },
    {
      type: "Line",
      point0: [-fieldDimensions.length / 2.0, -fieldDimensions.width / 2.0],
      point1: [fieldDimensions.length / 2.0, -fieldDimensions.width / 2.0],
    },
    {
      type: "Line",
      point0: [-fieldDimensions.length / 2.0, -fieldDimensions.width / 2.0],
      point1: [-fieldDimensions.length / 2.0, fieldDimensions.width / 2.0],
    },
    {
      type: "Line",
      point0: [fieldDimensions.length / 2.0, -fieldDimensions.width / 2.0],
      point1: [fieldDimensions.length / 2.0, fieldDimensions.width / 2.0],
    },
    {
      type: "Line",
      point0: [
        -fieldDimensions.length / 2.0,
        fieldDimensions.penalty_area_width / 2.0,
      ],
      point1: [
        -fieldDimensions.length / 2.0 + fieldDimensions.penalty_area_length,
        fieldDimensions.penalty_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        -fieldDimensions.length / 2.0,
        -fieldDimensions.penalty_area_width / 2.0,
      ],
      point1: [
        -fieldDimensions.length / 2.0 + fieldDimensions.penalty_area_length,
        -fieldDimensions.penalty_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        -fieldDimensions.length / 2.0 + fieldDimensions.penalty_area_length,
        -fieldDimensions.penalty_area_width / 2.0,
      ],
      point1: [
        -fieldDimensions.length / 2.0 + fieldDimensions.penalty_area_length,
        fieldDimensions.penalty_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        -fieldDimensions.length / 2.0,
        fieldDimensions.goal_box_area_width / 2.0,
      ],
      point1: [
        -fieldDimensions.length / 2.0 + fieldDimensions.goal_box_area_length,
        fieldDimensions.goal_box_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        -fieldDimensions.length / 2.0,
        -fieldDimensions.goal_box_area_width / 2.0,
      ],
      point1: [
        -fieldDimensions.length / 2.0 + fieldDimensions.goal_box_area_length,
        -fieldDimensions.goal_box_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        -fieldDimensions.length / 2.0 + fieldDimensions.goal_box_area_length,
        -fieldDimensions.goal_box_area_width / 2.0,
      ],
      point1: [
        -fieldDimensions.length / 2.0 + fieldDimensions.goal_box_area_length,
        fieldDimensions.goal_box_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        fieldDimensions.length / 2.0 - fieldDimensions.penalty_area_length,
        fieldDimensions.penalty_area_width / 2.0,
      ],
      point1: [
        fieldDimensions.length / 2.0,
        fieldDimensions.penalty_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        fieldDimensions.length / 2.0 - fieldDimensions.penalty_area_length,
        -fieldDimensions.penalty_area_width / 2.0,
      ],
      point1: [
        fieldDimensions.length / 2.0,
        -fieldDimensions.penalty_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        fieldDimensions.length / 2.0 - fieldDimensions.penalty_area_length,
        -fieldDimensions.penalty_area_width / 2.0,
      ],
      point1: [
        fieldDimensions.length / 2.0 - fieldDimensions.penalty_area_length,
        fieldDimensions.penalty_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        fieldDimensions.length / 2.0 - fieldDimensions.goal_box_area_length,
        fieldDimensions.goal_box_area_width / 2.0,
      ],
      point1: [
        fieldDimensions.length / 2.0,
        fieldDimensions.goal_box_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        fieldDimensions.length / 2.0 - fieldDimensions.goal_box_area_length,
        -fieldDimensions.goal_box_area_width / 2.0,
      ],
      point1: [
        fieldDimensions.length / 2.0,
        -fieldDimensions.goal_box_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        fieldDimensions.length / 2.0 - fieldDimensions.goal_box_area_length,
        -fieldDimensions.goal_box_area_width / 2.0,
      ],
      point1: [
        fieldDimensions.length / 2.0 - fieldDimensions.goal_box_area_length,
        fieldDimensions.goal_box_area_width / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [0.0, -fieldDimensions.width / 2.0],
      point1: [0.0, fieldDimensions.width / 2.0],
    },
    {
      type: "Circle",
      center: [0.0, 0.0],
      radius: fieldDimensions.center_circle_diameter / 2.0,
    },
    {
      type: "Line",
      point0: [
        -fieldDimensions.length / 2.0 +
          fieldDimensions.penalty_marker_distance -
          fieldDimensions.penalty_marker_size / 2.0,
        0.0,
      ],
      point1: [
        -fieldDimensions.length / 2.0 +
          fieldDimensions.penalty_marker_distance +
          fieldDimensions.penalty_marker_size / 2.0,
        0.0,
      ],
    },
    {
      type: "Line",
      point0: [
        -fieldDimensions.length / 2.0 + fieldDimensions.penalty_marker_distance,
        -fieldDimensions.penalty_marker_size / 2.0,
      ],
      point1: [
        -fieldDimensions.length / 2.0 + fieldDimensions.penalty_marker_distance,
        fieldDimensions.penalty_marker_size / 2.0,
      ],
    },
    {
      type: "Line",
      point0: [
        fieldDimensions.length / 2.0 -
          fieldDimensions.penalty_marker_distance -
          fieldDimensions.penalty_marker_size / 2.0,
        0.0,
      ],
      point1: [
        fieldDimensions.length / 2.0 -
          fieldDimensions.penalty_marker_distance +
          fieldDimensions.penalty_marker_size / 2.0,
        0.0,
      ],
    },
    {
      type: "Line",
      point0: [
        fieldDimensions.length / 2.0 - fieldDimensions.penalty_marker_distance,
        -fieldDimensions.penalty_marker_size / 2.0,
      ],
      point1: [
        fieldDimensions.length / 2.0 - fieldDimensions.penalty_marker_distance,
        fieldDimensions.penalty_marker_size / 2.0,
      ],
    },
  ];
}
