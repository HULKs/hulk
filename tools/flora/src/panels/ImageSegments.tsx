import { useEffect, useRef, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./ImageSegments.css";

type ColorYCbCr = {
  cb: number;
  cr: number;
  y: number;
};

type Segment = {
  start: number;
  end: number;
  start_edge_type: String;
  end_edge_type: String;
  field_color: String;
  color: ColorYCbCr;
};

type ScanLine = {
  position: number;
  segments: Array<Segment>;
};

function clamp(n: number, low: number, high: number) {
  if (n < low) { return (low); }
  if (n > high) { return (high); }
  return n;
}

function yuv2rgb(color: ColorYCbCr) {
  const r = clamp(Math.floor(color.y + 1.4075 * (color.cb - 128)), 0, 255);
  const g = clamp(Math.floor(color.y - 0.3455 * (color.cr - 128) - (0.7169 * (color.cb - 128))), 0, 255);
  const b = clamp(Math.floor(color.y + 1.7790 * (color.cr - 128)), 0, 255);
  return ({ r: r, g: g, b: b });
}

export default function ImageSegments({
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
  const [imageSegmentsData, setImageSegmentsData] = useState<
    {
      scan_grid: {
        horizontal_scan_lines: Array<ScanLine>;
        vertical_scan_lines: Array<ScanLine>;
      };
    } | null | undefined
  >(undefined);
  const [filterSegments, setFilterSegments] = useState<boolean>(true);
  useEffect(() => {
    if (connection === null) {
      return;
    }
    const unsubscribeImageSegments = connection.subscribeOutput(
      cycler,
      OutputType.Main,
      filterSegments ? "filtered_segments" : "image_segments",
      (data) => {
        setImageSegmentsData(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return () => {
      unsubscribeImageSegments();
    };
  }, [connection, cycler, filterSegments]);

  let onCheckboxChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    console.log(e);
    setFilterSegments(!filterSegments);
  };

  let canvasRef = useRef<HTMLCanvasElement | null>(null);
  let canvasCtxRef = useRef<CanvasRenderingContext2D | null>(null);

  if (canvasRef.current) {
    canvasCtxRef.current = canvasRef.current.getContext("2d");
    let ctx = canvasCtxRef.current;
    ctx!.fillStyle = "#000000";
    ctx!.fillRect(0, 0, 640, 480);

    imageSegmentsData!.scan_grid.vertical_scan_lines.slice(0, 250).forEach((scan_line) => {
      scan_line.segments.forEach((segment) => {
        let rgb = yuv2rgb(segment.color);
        let color = `rgb(${rgb.r} ${rgb.g} ${rgb.b})`;
        ctx!.fillStyle = color;
        ctx!.fillRect(scan_line.position * 2, segment.start, 1, segment.end - segment.start);
      });
    });
  }

  return (
    <div className="imageSegments">
      <div className="header">
        <div className="panelType">ImageSegments:</div>
        <div className="cycler">{cycler}</div>
        <div className="filterToggle">
          <input id="filterSegments" type="checkbox" onChange={onCheckboxChange} checked={filterSegments} />
          <label htmlFor="filterSegments">Filter segments</label>
        </div>
        {selector}
        {connector}
      </div>
      {imageSegmentsData !== undefined ? (
        <>
          <canvas className="content" width="640" height="480" ref={canvasRef} />
        </>
      ) : (
        <div className="content noData">NAO has not sent any data yet</div>
      )}
    </div>
  );
}
