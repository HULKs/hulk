import { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./MotionDispatching.css";

enum MotionType {
  DoNotInject = "DoNotInject",
  FallProtection = "FallProtection",
  Jump = "Jump",
  Kick = "Kick",
  Penalized = "Penalized",
  Sit = "Sit",
  Stand = "Stand",
  StandUp = "StandUp",
  Unstiff = "Unstiff",
  Walk = "Walk",
}

enum HeadMotionType {
  Center = "Center",
  LookAround = "LookAround",
  LookAt = "LookAt",
  Unstiff = "Unstiff",
  ZeroAngles = "ZeroAngles",
}

export default function MotionDispatching({
  selector,
  connector,
  connection,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
}) {
  const [motionData, setMotionData] = useState(undefined);
  const [motionSelectionData, setMotionSelectionData] = useState(undefined);
  const [motionType, setMotionType] = useState(MotionType.DoNotInject);
  const [standHeadMotionType, setStandHeadMotionType] = useState(
    HeadMotionType.Unstiff
  );
  useEffect(() => {
    if (connection === null) {
      return;
    }
    switch (motionType) {
      case MotionType.DoNotInject:
        connection.updateParameter(
          "control.behavior.injected_motion_command",
          null,
          () => {},
          (error) => {
            alert(`Error: ${error}`);
          }
        );
        break;
      case MotionType.FallProtection:
        connection.updateParameter(
          "control.behavior.injected_motion_command",
          { motion: { FallProtection: { direction: "Forward" } } },
          () => {},
          (error) => {
            alert(`Error: ${error}`);
          }
        );
        break;
      case MotionType.Jump:
        alert("Not implemented");
        break;
      case MotionType.Kick:
        alert("Not implemented");
        break;
      case MotionType.Penalized:
        connection.updateParameter(
          "control.behavior.injected_motion_command",
          { motion: "Penalized" },
          () => {},
          (error) => {
            alert(`Error: ${error}`);
          }
        );
        break;
      case MotionType.Sit:
        connection.updateParameter(
          "control.behavior.injected_motion_command",
          { motion: { Sit: { head: "Unstiff", direction: "Down" } } },
          () => {},
          (error) => {
            alert(`Error: ${error}`);
          }
        );
        break;
      case MotionType.Stand:
        const headMotion = ((headMotionType) => {
          switch (headMotionType) {
            case HeadMotionType.Center:
              return "Center";
            case HeadMotionType.LookAround:
              return "LookAround";
            case HeadMotionType.LookAt:
              return { LookAt: { target: [1, 1] } };
            case HeadMotionType.Unstiff:
              return "Unstiff";
            case HeadMotionType.ZeroAngles:
              return "ZeroAngles";
          }
        })(standHeadMotionType);
        connection.updateParameter(
          "control.behavior.injected_motion_command",
          { motion: { Stand: { head: headMotion } } },
          () => {},
          (error) => {
            alert(`Error: ${error}`);
          }
        );
        break;
      case MotionType.StandUp:
        connection.updateParameter(
          "control.behavior.injected_motion_command",
          { motion: { StandUp: { facing: "Up" } } },
          () => {},
          (error) => {
            alert(`Error: ${error}`);
          }
        );
        break;
      case MotionType.Unstiff:
        connection.updateParameter(
          "control.behavior.injected_motion_command",
          { motion: "Unstiff" },
          () => {},
          (error) => {
            alert(`Error: ${error}`);
          }
        );
        break;
      case MotionType.Walk:
        connection.updateParameter(
          "control.behavior.injected_motion_command",
          {
            motion: {
              Walk: {
                head: "Unstiff",
                in_walk_kick: "None",
                left_arm: "PullBack",
                right_arm: "PullBack",
                target_pose: {
                  rotation: [0, 0],
                  translation: [1, 0],
                },
              },
            },
          },
          () => {},
          (error) => {
            alert(`Error: ${error}`);
          }
        );
        break;
    }
  }, [connection, motionType, standHeadMotionType]);
  useEffect(() => {
    if (connection === null) {
      return;
    }
    const unsubscribeMotion = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "motion_command",
      (data) => {
        setMotionData(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    const unsubscribeMotionSelection = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "motion_selection",
      (data) => {
        setMotionSelectionData(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return () => {
      unsubscribeMotion();
      unsubscribeMotionSelection();
    };
  }, [connection]);
  const content =
    motionData === undefined ? (
      <div className="content noData">NAO has not sent any data yet</div>
    ) : (
      <pre className="content">
        <div className="motion">{JSON.stringify(motionData, null, 2)}</div>
        <div className="motionSelection">
          {JSON.stringify(motionSelectionData, null, 2)}
        </div>
        <div className="motionCommand">
          <div className="motionType">
            <input
              id="motionTypeDoNotInject"
              type="radio"
              checked={motionType === MotionType.DoNotInject}
              onChange={() => {
                setMotionType(MotionType.DoNotInject);
              }}
            />
            <label htmlFor="motionTypeDoNotInject">DoNotInject</label>
          </div>
          <div className="motionType">
            <input
              id="motionTypeFallProtection"
              type="radio"
              checked={motionType === MotionType.FallProtection}
              onChange={() => {
                setMotionType(MotionType.FallProtection);
              }}
            />
            <label htmlFor="motionTypeFallProtection">FallProtection</label>
          </div>
          <div className="motionType">
            <input
              id="motionTypeJump"
              type="radio"
              checked={motionType === MotionType.Jump}
              onChange={() => {
                setMotionType(MotionType.Jump);
              }}
            />
            <label htmlFor="motionTypeJump">Jump</label>
          </div>
          <div className="motionType">
            <input
              id="motionTypeKick"
              type="radio"
              checked={motionType === MotionType.Kick}
              onChange={() => {
                setMotionType(MotionType.Kick);
              }}
            />
            <label htmlFor="motionTypeKick">Kick</label>
          </div>
          <div className="motionType">
            <input
              id="motionTypePenalized"
              type="radio"
              checked={motionType === MotionType.Penalized}
              onChange={() => {
                setMotionType(MotionType.Penalized);
              }}
            />
            <label htmlFor="motionTypePenalized">Penalized</label>
          </div>
          <div className="motionType">
            <input
              id="motionTypeSit"
              type="radio"
              checked={motionType === MotionType.Sit}
              onChange={() => {
                setMotionType(MotionType.Sit);
              }}
            />
            <label htmlFor="motionTypeSit">Sit</label>
          </div>
          <div className="motionType">
            <input
              id="motionTypeStand"
              type="radio"
              checked={motionType === MotionType.Stand}
              onChange={() => {
                setMotionType(MotionType.Stand);
              }}
            />
            <label htmlFor="motionTypeStand">Stand</label>
            <select
              value={standHeadMotionType}
              onChange={(event) =>
                setStandHeadMotionType(event.target.value as HeadMotionType)
              }
            >
              <option value={HeadMotionType.Center}>Center</option>
              <option value={HeadMotionType.LookAround}>LookAround</option>
              <option value={HeadMotionType.LookAt}>LookAt</option>
              <option value={HeadMotionType.Unstiff}>Unstiff</option>
              <option value={HeadMotionType.ZeroAngles}>ZeroAngles</option>
            </select>
          </div>
          <div className="motionType">
            <input
              id="motionTypeStandUp"
              type="radio"
              checked={motionType === MotionType.StandUp}
              onChange={() => {
                setMotionType(MotionType.StandUp);
              }}
            />
            <label htmlFor="motionTypeStandUp">StandUp</label>
          </div>
          <div className="motionType">
            <input
              id="motionTypeUnstiff"
              type="radio"
              checked={motionType === MotionType.Unstiff}
              onChange={() => {
                setMotionType(MotionType.Unstiff);
              }}
            />
            <label htmlFor="motionTypeUnstiff">Unstiff</label>
          </div>
          <div className="motionType">
            <input
              id="motionTypeWalk"
              type="radio"
              checked={motionType === MotionType.Walk}
              onChange={() => {
                setMotionType(MotionType.Walk);
              }}
            />
            <label htmlFor="motionTypeWalk">Walk</label>
          </div>
        </div>
      </pre>
    );
  return (
    <div className="motionDispatching">
      <div className="header">
        <div className="panelType">MotionDispatching</div>
        {selector}
        {connector}
      </div>
      {content}
    </div>
  );
}
