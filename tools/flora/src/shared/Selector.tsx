import fuzzysort from "fuzzysort";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  Horizon,
  ImageSegments,
  LineDetection,
  ProjectedFieldLines,
  FieldBorder,
  BallCandidates,
  PanelType,
  RawImage,
  RawOutput,
  SelectablePanel,
  SelectablePanels,
  Parameter,
  Localization,
  AudioSpectrums,
  MotionDispatching,
} from "../useSelectablePanels";
import "./Selector.css";

function highlightRawOutput(parameters: RawOutput): JSX.Element {
  return (
    <>
      RawOutput.{parameters.cycler}.{parameters.outputType}.{parameters.path}
    </>
  );
}

function highlightRawImage(parameters: RawImage): JSX.Element {
  return <>RawImage.{parameters.cycler}</>;
}

function highlightParameter(parameters: Parameter): JSX.Element {
  return <>Parameter.{parameters.path}</>;
}

function highlightHorizon(parameters: Horizon): JSX.Element {
  return <>Horizon.{parameters.cycler}</>;
}

function highlightImageSegments(parameters: ImageSegments): JSX.Element {
  return <>ImageSegments.{parameters.cycler}</>;
}

function highlightLineDetection(parameters: LineDetection): JSX.Element {
  return <>LineDetection.{parameters.cycler}</>;
}

function highlightProjectedFieldLines(
  parameters: ProjectedFieldLines
): JSX.Element {
  return <>ProjectedFieldLines.{parameters.cycler}</>;
}

function highlightFieldBorder(parameters: FieldBorder): JSX.Element {
  return <>FieldBorder.{parameters.cycler}</>;
}

function highlightBallCandidates(parameters: BallCandidates): JSX.Element {
  return <>BallCandidates.{parameters.cycler}</>;
}

function highlightLocalization(parameters: Localization): JSX.Element {
  return <>Localization</>;
}

function highlightMotionDispatching(
  parameters: MotionDispatching
): JSX.Element {
  return <>MotionDispatching</>;
}

function highlightAudioSpectrums(parameters: AudioSpectrums): JSX.Element {
  return <>AudioSpectrums</>;
}

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
          return highlightRawOutput(item.obj.selectablePanel);
        case PanelType.RawImage:
          return highlightRawImage(item.obj.selectablePanel);
        case PanelType.Parameter:
          return highlightParameter(item.obj.selectablePanel);
        case PanelType.Horizon:
          return highlightHorizon(item.obj.selectablePanel);
        case PanelType.ImageSegments:
          return highlightImageSegments(item.obj.selectablePanel);
        case PanelType.LineDetection:
          return highlightLineDetection(item.obj.selectablePanel);
        case PanelType.ProjectedFieldLines:
          return highlightProjectedFieldLines(item.obj.selectablePanel);
        case PanelType.FieldBorder:
          return highlightFieldBorder(item.obj.selectablePanel);
        case PanelType.BallCandidates:
          return highlightBallCandidates(item.obj.selectablePanel);
        case PanelType.Localization:
          return highlightLocalization(item.obj.selectablePanel);
        case PanelType.MotionDispatching:
          return highlightMotionDispatching(item.obj.selectablePanel);
        case PanelType.AudioSpectrums:
          return highlightAudioSpectrums(item.obj.selectablePanel);
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
