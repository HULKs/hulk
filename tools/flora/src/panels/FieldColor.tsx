import Connection, { Cycler } from "../Connection/Connection";
import { useParameterSubscription } from "../useSubscription";
import "./FieldColor.css";

export default function FieldColor({
  selector,
  connector,
  connection,
  cycler,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
  cycler: Cycler;
}) {
  const redChromaticityThresholdPath = `${
    cycler === Cycler.VisionTop ? "vision_top" : "vision_bottom"
  }.field_color_detection.red_chromaticity_threshold`;
  const redChromaticityThreshold = useParameterSubscription<number>(
    connection,
    redChromaticityThresholdPath
  );
  const blueChromaticityThresholdPath = `${
    cycler === Cycler.VisionTop ? "vision_top" : "vision_bottom"
  }.field_color_detection.blue_chromaticity_threshold`;
  const blueChromaticityThreshold = useParameterSubscription<number>(
    connection,
    blueChromaticityThresholdPath
  );
  const lowerGreenChromaticityThresholdPath = `${
    cycler === Cycler.VisionTop ? "vision_top" : "vision_bottom"
  }.field_color_detection.lower_green_chromaticity_threshold`;
  const lowerGreenChromaticityThreshold = useParameterSubscription<number>(
    connection,
    lowerGreenChromaticityThresholdPath
  );
  const upperGreenChromaticityThresholdPath = `${
    cycler === Cycler.VisionTop ? "vision_top" : "vision_bottom"
  }.field_color_detection.upper_green_chromaticity_threshold`;
  const upperGreenChromaticityThreshold = useParameterSubscription<number>(
    connection,
    upperGreenChromaticityThresholdPath
  );
  const content =
    redChromaticityThreshold !== undefined &&
    blueChromaticityThreshold !== undefined &&
    lowerGreenChromaticityThreshold !== undefined &&
    upperGreenChromaticityThreshold ? (
      <div className="content">
        <div>
          redChromaticityThreshold:{" "}
          <input
            type="range"
            min="0"
            max="1"
            value={redChromaticityThreshold}
            onChange={(event) => {
              if (connection !== null) {
                connection.updateParameter(
                  redChromaticityThresholdPath,
                  parseFloat(event.target.value),
                  () => {},
                  (error) => {
                    alert(`Error: ${error}`);
                  }
                );
              }
            }}
            step="0.01"
          />{" "}
          {redChromaticityThreshold}
        </div>
        <div>
          blueChromaticityThreshold:{" "}
          <input
            type="range"
            min="0"
            max="1"
            value={blueChromaticityThreshold}
            onChange={(event) => {
              if (connection !== null) {
                connection.updateParameter(
                  blueChromaticityThresholdPath,
                  parseFloat(event.target.value),
                  () => {},
                  (error) => {
                    alert(`Error: ${error}`);
                  }
                );
              }
            }}
            step="0.01"
          />{" "}
          {blueChromaticityThreshold}
        </div>
        <div>
          lowerGreenChromaticityThreshold:{" "}
          <input
            type="range"
            min="0"
            max="1"
            value={lowerGreenChromaticityThreshold}
            onChange={(event) => {
              if (connection !== null) {
                connection.updateParameter(
                  lowerGreenChromaticityThresholdPath,
                  parseFloat(event.target.value),
                  () => {},
                  (error) => {
                    alert(`Error: ${error}`);
                  }
                );
              }
            }}
            step="0.01"
          />{" "}
          {lowerGreenChromaticityThreshold}
        </div>
        <div>
          upperGreenChromaticityThreshold:{" "}
          <input
            type="range"
            min="0"
            max="1"
            value={upperGreenChromaticityThreshold}
            onChange={(event) => {
              if (connection !== null) {
                connection.updateParameter(
                  upperGreenChromaticityThresholdPath,
                  parseFloat(event.target.value),
                  () => {},
                  (error) => {
                    alert(`Error: ${error}`);
                  }
                );
              }
            }}
            step="0.01"
          />{" "}
          {upperGreenChromaticityThreshold}
        </div>
      </div>
    ) : (
      <div className="content noData">NAO has not sent any data yet</div>
    );
  return (
    <div className="fieldColor">
      <div className="header">
        <div className="panelType">FieldColor:</div>
        <div className="cycler">{cycler}</div>
        {selector}
        {connector}
      </div>
      {content}
    </div>
  );
}
