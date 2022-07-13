import fuzzysort from "fuzzysort";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  PanelType,
  SelectablePanel,
  SelectablePanels,
} from "../useSelectablePanels";
import "./Selector.css";

export default function Selector({
  selectablePanels,
  onSelect,
}: {
  selectablePanels: SelectablePanels;
  onSelect: (selected: SelectablePanel) => void;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const sortPaths = useMemo(
    () =>
      Object.entries(selectablePanels).map(([sortPath, selectablePanel]) => ({
        sortPath,
        selectablePanel,
      })),
    [selectablePanels]
  );
  const filtered = useMemo(() => {
    if (query.length === 0) {
      return sortPaths.map((item) => ({
        target: item.sortPath,
        indexes: [],
        obj: item,
      }));
    }
    const filtered = fuzzysort.go(query, sortPaths, { key: "sortPath" });
    if (selected >= filtered.length) {
      setSelected(Math.max(0, filtered.length - 1));
    }
    return filtered.slice(0, 20);
  }, [query, sortPaths, selected]);
  const queryElement = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    if (open) {
      if (queryElement.current !== null) {
        queryElement.current.focus();
      }
      setSelected(0);
    }
  }, [queryElement, open]);
  const suggestions = filtered.map((item, index) => {
    const highlighted = (() => {
      switch (item.obj.selectablePanel.panelType) {
        case PanelType.RawOutput:
          return (
            <>
              RawOutput.{item.obj.selectablePanel.cycler}.
              {item.obj.selectablePanel.outputType}.
              {item.obj.selectablePanel.path}
            </>
          );
        case PanelType.RawImage:
          return <>RawImage.{item.obj.selectablePanel.cycler}</>;
        case PanelType.Parameter:
          return <>Parameter.{item.obj.selectablePanel.path}</>;
        case PanelType.Horizon:
          return <>Horizon.{item.obj.selectablePanel.cycler}</>;
        case PanelType.ImageSegments:
          return <>ImageSegments.{item.obj.selectablePanel.cycler}</>;
        case PanelType.LineDetection:
          return <>LineDetection.{item.obj.selectablePanel.cycler}</>;
        case PanelType.ProjectedFieldLines:
          return <>ProjectedFieldLines.{item.obj.selectablePanel.cycler}</>;
        case PanelType.FieldBorder:
          return <>FieldBorder.{item.obj.selectablePanel.cycler}</>;
        case PanelType.BallCandidates:
          return <>BallCandidates.{item.obj.selectablePanel.cycler}</>;
        case PanelType.Localization:
          return <>Localization</>;
        case PanelType.BallFilter:
          return <>BallFilter</>;
        case PanelType.PathPlanning:
          return <>PathPlanning</>;
        case PanelType.Behavior:
          return <>Behavior</>;
        case PanelType.MotionDispatching:
          return <>MotionDispatching</>;
        case PanelType.AudioSpectrums:
          return <>AudioSpectrums</>;
        case PanelType.Odometry:
          return <>Odometry</>;
        case PanelType.ProjectedLimbs:
          return <>ProjectedLimbs.{item.obj.selectablePanel.cycler}</>;
        case PanelType.LineFitting:
          return <>LineFitting</>;
        case PanelType.RobotDetection:
          return <>RobotDetection.{item.obj.selectablePanel.cycler}</>;
        case PanelType.FieldColor:
          return <>FieldColor.{item.obj.selectablePanel.cycler}</>;
      }
    })();
    return (
      <div
        key={item.target}
        className={index === selected ? "suggestion selected" : "suggestion"}
        onClick={() => {
          onSelect(filtered[index].obj.selectablePanel);
          setOpen(false);
        }}
      >
        {highlighted}
      </div>
    );
  });
  return (
    <div className="selector">
      <button
        onClick={() => {
          setOpen(true);
        }}
      >
        Panel
      </button>
      <div
        className={open ? "modal" : "modal hidden"}
        onClick={(event) => {
          if (event.target === event.currentTarget) {
            setOpen(false);
          }
        }}
      >
        <div className="inner">
          <input
            ref={queryElement}
            type="text"
            onChange={(event) => {
              setQuery(event.target.value);
            }}
            onKeyDown={(event) => {
              switch (event.key) {
                case "ArrowUp": {
                  event.preventDefault();
                  setSelected(
                    filtered.length === 0
                      ? 0
                      : (selected + filtered.length - 1) % filtered.length
                  );
                  break;
                }
                case "ArrowDown": {
                  event.preventDefault();
                  setSelected(
                    filtered.length === 0 ? 0 : (selected + 1) % filtered.length
                  );
                  break;
                }
                case "Enter": {
                  event.preventDefault();
                  onSelect(filtered[selected].obj.selectablePanel);
                  setOpen(false);
                  break;
                }
                case "Escape": {
                  event.preventDefault();
                  if (open && queryElement.current !== null) {
                    queryElement.current.blur();
                  }
                  setOpen(false);
                  break;
                }
              }
            }}
          />
          <div className="suggestions">{suggestions}</div>
        </div>
      </div>
    </div>
  );
}
