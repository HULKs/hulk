import { useEffect, useState } from "react";

export enum State {
  Pause = "Pause",
  BackwardRealtime = "BackwardRealtime",
  ForwardRealtime = "ForwardRealtime",
  BackwardFast = "BackwardFast",
  ForwardFast = "ForwardFast",
}

export function useAnimation(
  intervalMillisecondsRealtime: number,
  intervalMillisecondsFast: number,
  stepForward: () => void,
  stepBackward: () => void
): [State, React.Dispatch<React.SetStateAction<State>>] {
  const [state, setState] = useState(State.Pause);
  useEffect(() => {
    switch (state) {
      case State.Pause: {
        return () => {};
      }
      case State.BackwardRealtime: {
        const intervalId = setInterval(
          stepBackward,
          intervalMillisecondsRealtime
        );
        return () => {
          clearInterval(intervalId);
        };
      }
      case State.ForwardRealtime: {
        const intervalId = setInterval(
          stepForward,
          intervalMillisecondsRealtime
        );
        return () => {
          clearInterval(intervalId);
        };
      }
      case State.BackwardFast: {
        const intervalId = setInterval(stepBackward, intervalMillisecondsFast);
        return () => {
          clearInterval(intervalId);
        };
      }
      case State.ForwardFast: {
        const intervalId = setInterval(stepForward, intervalMillisecondsFast);
        return () => {
          clearInterval(intervalId);
        };
      }
    }
  }, [
    state,
    intervalMillisecondsRealtime,
    intervalMillisecondsFast,
    stepForward,
    stepBackward,
  ]);
  return [state, setState];
}
