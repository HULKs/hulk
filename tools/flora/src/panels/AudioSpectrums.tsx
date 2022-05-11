import { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./AudioSpectrums.css";

export default function RawOutput({
  selector,
  connector,
  connection,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
}) {
  const [audioSpectrums, setAudioSpectrums] = useState<[number, number][][] | undefined>(undefined);
  const [detectionConfiguration, setDetectionConfiguration] = useState<
    | {
        detection_band: { start: number; end: number };
      }
    | undefined
  >(undefined);
  useEffect(() => {
    if (connection === null) {
      return;
    }
    const unsubscribeAudioSpectrums = connection.subscribeOutput(
      Cycler.Audio,
      OutputType.Additional,
      "audio_spectrums",
      (data) => {
        setAudioSpectrums(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    const unsubscribeDetectionConfiguration = connection.subscribeParameter(
      "audio.whistle_detection",
      (data) => {
        setDetectionConfiguration(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return () => {
      unsubscribeAudioSpectrums();
      unsubscribeDetectionConfiguration();
    };
  }, [connection]);
  let content = (
    <div className="content noData">
      NAO has not sent any data yet or the data is incomplete
    </div>
  );
  if (audioSpectrums !== undefined && detectionConfiguration !== undefined) {
    const minimumFrequency =
      detectionConfiguration.detection_band.start;
    const audioVisualisationScalingFactor = 0.65;
    const maximumFrequency = detectionConfiguration.detection_band.end;
    const height = 22000 / (16 / 9);
    const paths = audioSpectrums
      .map(
        (amplitudes) =>
          `M ${amplitudes[0][0]} ${audioVisualisationScalingFactor * amplitudes[0][1] * height}` +
          amplitudes
            .map(
              (amplitude) => `L ${amplitude[0]} ${audioVisualisationScalingFactor * amplitude[1] * height}`
            )
            .join(" ")
      )
      .map((path, index) => (
        <path
          key={index}
          d={path}
          fill="none"
          stroke="black"
          strokeWidth="10"
        />
      ));
    content = (
      <svg
        className="content"
        viewBox={`0 -${height} 22000 ${height}`}
      >
        <g transform="scale(1, -1)">
        {paths}
        <rect
          x={minimumFrequency}
          y={0}
          width={maximumFrequency - minimumFrequency}
          height={height}
          fill="none"
          stroke="black"
          strokeWidth="10"
        />
        </g>
      </svg>
    );
  }
  return (
    <div className="audioSpectrums">
      <div className="header">
        <div className="panelType">AudioSpectrums</div>
        {selector}
        {connector}
      </div>
      {content}
    </div>
  );
}
