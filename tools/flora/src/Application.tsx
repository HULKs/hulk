import { useState } from "react";
import RawImage from "./panels/RawImage";
import RawOutput from "./panels/RawOutput";
import Connector from "./shared/Connector";
import Selector from "./shared/Selector";
import useConnection from "./useConnection";
import useSelectablePanels, {
  PanelType,
  SelectablePanel,
} from "./useSelectablePanels";
import "./Application.css";
import Horizon from "./panels/Horizon";
import ImageSegments from "./panels/ImageSegments";
import LineDetection from "./panels/LineDetection";
import ProjectedFieldLines from "./panels/ProjectedFieldLines";
import FieldBorder from "./panels/FieldBorder";
import BallCandidates from "./panels/BallCandidates";
import Localization from "./panels/Localization";
import BallFilter from "./panels/BallFilter";
import PathPlanning from "./panels/PathPlanning";
import Behavior from "./panels/Behavior";
import { Cycler } from "./Connection/Connection";
import Parameter from "./panels/Parameter";
import AudioSpectrums from "./panels/AudioSpectrums";
import MotionDispatching from "./panels/MotionDispatching";
import Odometry from "./panels/Odometry";
import ProjectedLimbs from "./panels/ProjectedLimbs";
import LineFitting from "./panels/LineFitting";
import RobotDetection from "./panels/RobotDetection";

export default function Application() {
  const [
    connect,
    setConnect,
    ,
    setWebSocketUrl,
    outputHierarchy,
    parameterHierarchy,
    connectionState,
    connection,
  ] = useConnection();
  const selectablePanels = useSelectablePanels(
    outputHierarchy,
    parameterHierarchy
  );
  const [selectedPanel, setSelectedPanel] = useState<SelectablePanel>({
    panelType: PanelType.RawImage,
    cycler: Cycler.VisionTop,
  });
  const selector = (
    <Selector selectablePanels={selectablePanels} onSelect={setSelectedPanel} />
  );
  const connector = (
    <Connector
      connectionState={connectionState}
      connect={connect}
      setConnect={setConnect}
      setWebSocketUrl={setWebSocketUrl}
    />
  );
  if (selectedPanel.panelType === PanelType.RawOutput) {
    return (
      <RawOutput
        selector={selector}
        connector={connector}
        connection={connection}
        cycler={selectedPanel.cycler}
        outputType={selectedPanel.outputType}
        path={selectedPanel.path}
        type={selectedPanel.type}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.RawImage) {
    return (
      <RawImage
        selector={selector}
        connector={connector}
        connection={connection}
        cycler={selectedPanel.cycler}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.Parameter) {
    return (
      <Parameter
        selector={selector}
        connector={connector}
        connection={connection}
        path={selectedPanel.path}
        type={selectedPanel.type}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.Horizon) {
    return (
      <Horizon
        selector={selector}
        connector={connector}
        connection={connection}
        cycler={selectedPanel.cycler}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.ImageSegments) {
    return (
      <ImageSegments
        selector={selector}
        connector={connector}
        connection={connection}
        cycler={selectedPanel.cycler}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.LineDetection) {
    return (
      <LineDetection
        selector={selector}
        connector={connector}
        connection={connection}
        cycler={selectedPanel.cycler}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.ProjectedFieldLines) {
    return (
      <ProjectedFieldLines
        selector={selector}
        connector={connector}
        connection={connection}
        cycler={selectedPanel.cycler}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.FieldBorder) {
    return (
      <FieldBorder
        selector={selector}
        connector={connector}
        connection={connection}
        cycler={selectedPanel.cycler}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.BallCandidates) {
    return (
      <BallCandidates
        selector={selector}
        connector={connector}
        connection={connection}
        cycler={selectedPanel.cycler}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.Localization) {
    return (
      <Localization
        selector={selector}
        connector={connector}
        connection={connection}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.BallFilter) {
    return (
      <BallFilter
        selector={selector}
        connector={connector}
        connection={connection}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.PathPlanning) {
    return (
      <PathPlanning
        selector={selector}
        connector={connector}
        connection={connection}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.Behavior) {
    return (
      <Behavior
        selector={selector}
        connector={connector}
        connection={connection}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.AudioSpectrums) {
    return (
      <AudioSpectrums
        selector={selector}
        connector={connector}
        connection={connection}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.MotionDispatching) {
    return (
      <MotionDispatching
        selector={selector}
        connector={connector}
        connection={connection}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.Odometry) {
    return (
      <Odometry
        selector={selector}
        connector={connector}
        connection={connection}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.ProjectedLimbs) {
    return (
      <ProjectedLimbs
        selector={selector}
        connector={connector}
        connection={connection}
        cycler={selectedPanel.cycler}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.RobotDetection) {
    return (
      <RobotDetection
        selector={selector}
        connector={connector}
        connection={connection}
        cycler={selectedPanel.cycler}
      />
    );
  }
  if (selectedPanel.panelType === PanelType.LineFitting) {
    return (
      <LineFitting
        selector={selector}
        connector={connector}
        connection={connection}
      />
    );
  }

  throw new Error("Reached unreachable code, this is a bug");
}
