import { Isometry2 } from "../Isometry2";

export default function Transform({
  isometry,
  children,
}: {
  isometry: Isometry2;
  children: React.ReactNode | React.ReactNode[];
}) {
  let angle =
    (180 / Math.PI) * Math.atan2(isometry.rotation[1], isometry.rotation[0]);
  return (
    <g
      transform={`translate(${isometry.translation[0]}, ${isometry.translation[1]}) rotate(${angle})`}
    >
      {children}
    </g>
  );
}
