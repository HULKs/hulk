import Connection, { Cycler, OutputType } from "../Connection/Connection";
import { useOutputSubscription } from "../useSubscription";
import "./LineFitting.css";

export default function RawOutput({
  selector,
  connector,
  connection,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
}) {
  const fitErrors = useOutputSubscription<number[][][][] | null>(
    connection,
    Cycler.Control,
    OutputType.Additional,
    "localization.fit_errors"
  );
  let content = (
    <div className="content noData">
      NAO has not sent any data yet or the data is incomplete
    </div>
  );
  if (fitErrors !== undefined && fitErrors !== null) {
    content = (
      <div className="content">
        {fitErrors.map((fitErrorsPerMeasurement) => (
          <div className="measurement">
            {fitErrorsPerMeasurement.map((fitErrorsPerHypothesis) =>
              renderFitErrors(fitErrorsPerHypothesis)
            )}
          </div>
        ))}
      </div>
    );
  }
  return (
    <div className="lineFitting">
      <div className="header">
        <div className="panelType">LineFitting</div>
        {selector}
        {connector}
      </div>
      {content}
    </div>
  );
}

function renderFitErrors(fitErrors: number[][]) {
  const flattenedErrors = fitErrors.reduce(
    (fitErrors, fitErrorsPerIteration) => [
      ...fitErrors,
      ...fitErrorsPerIteration,
    ],
    []
  );
  const maximumError = Math.max(...flattenedErrors.map(Math.log10));
  const minimumError = Math.min(...flattenedErrors.map(Math.log10));
  const startError = flattenedErrors[0];
  const endError = flattenedErrors[flattenedErrors.length - 1];
  const amountOfErrors = flattenedErrors.length;
  return (
    <svg className="hypothesis" viewBox="-1 -1 23 11">
      <path
        d={flattenedErrors
          .map((error, index) => {
            const x = (index / amountOfErrors) * 21;
            const y =
              (1 -
                (Math.log10(error) - minimumError) /
                  (maximumError - minimumError)) *
              9;
            return `${index === 0 ? "M" : "L"} ${x} ${y}`;
          })
          .join(" ")}
        stroke="black"
        strokeWidth={0.05}
        fill="none"
      />
      <text x={0} y={0 - 0.1} fontSize={0.5}>
        {startError}
      </text>
      <text x={21} y={9 - 0.1} fontSize={0.5} textAnchor="end">
        {endError}
      </text>
      <text x={10.5} y={4.5} fontSize={0.5} textAnchor="middle">
        {flattenedErrors.length} iterations
      </text>
    </svg>
  );
}
