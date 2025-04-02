import os
from typing import override

import wandb
from gymnasium import Env
from gymnasium.wrappers import RecordVideo


class SingleEpisodeVideoRecorder(RecordVideo):
    def __init__(self, env: Env, video_folder: str) -> None:
        super().__init__(env, video_folder, episode_trigger=lambda _: True)

    @override
    def stop_recording(self) -> None:
        if len(self.recorded_frames) < 10:
            # If the video is too short, don't save it
            self.recorded_frames = []
            self.recording = False
            self._video_name = None
            return

        path = os.path.join(self.video_folder, f"{self._video_name}.mp4")
        super().stop_recording()

        print("Logging to wandb...")
        wandb.log({"video": wandb.Video(path, format="mp4")}, step=self.step_id)
