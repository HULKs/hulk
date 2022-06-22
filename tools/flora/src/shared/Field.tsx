import { FieldDimensions } from "../FieldDimensions";
import { Circle, Line, useMarks } from "../useMarks";
import "./Field.css";

export default function Field({
  fieldDimensions,
  children,
}: {
  fieldDimensions: FieldDimensions | null | undefined;
  children: React.ReactNode | React.ReactNode[];
}) {
  const marks = useMarks(fieldDimensions);
  if (
    fieldDimensions === undefined ||
    fieldDimensions === null ||
    marks === undefined ||
    marks === null
  ) {
    return <svg className="field" viewBox="0 0 1 1"></svg>;
  }
  return (
    <svg
      className="field"
      viewBox={`${
        -(fieldDimensions.length + fieldDimensions.border_strip_width) / 2
      } ${-(fieldDimensions.width + fieldDimensions.border_strip_width) / 2} ${
        fieldDimensions.length + fieldDimensions.border_strip_width
      } ${fieldDimensions.width + fieldDimensions.border_strip_width}`}
    >
      <g transform="scale(1, -1)">
        {marks.map((mark) => {
          switch (mark.type) {
            case "Line":
              return drawLine(mark, "white", fieldDimensions.line_width);
            case "Circle":
              return drawCircle(
                mark,
                "white",
                fieldDimensions.line_width,
                "none"
              );
          }
          return null;
        })}
        {children}
      </g>
    </svg>
  );
}

function drawLine(
  line: Line,
  strokeColor: string,
  strokeWidth: number
): JSX.Element {
  return (
    <line
      x1={line.point0[0]}
      y1={line.point0[1]}
      x2={line.point1[0]}
      y2={line.point1[1]}
      stroke={strokeColor}
      strokeWidth={strokeWidth}
      fill="none"
    />
  );
}

function drawCircle(
  circle: Circle,
  strokeColor: string,
  strokeWidth: number,
  fillColor: string
): JSX.Element {
  return (
    <circle
      cx={circle.center[0]}
      cy={circle.center[1]}
      r={circle.radius}
      stroke={strokeColor}
      strokeWidth={strokeWidth}
      fill={fillColor}
    />
  );
}
