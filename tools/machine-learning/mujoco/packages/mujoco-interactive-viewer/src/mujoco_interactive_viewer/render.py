from dataclasses import dataclass


@dataclass
class RenderState:
    is_paused: bool = False
    render_every_frame: bool = False
    time_per_render: float = 1.0 / 60.0
    run_speed: float = 1.0
    loop_count: float = 0.0
    steps_to_advance: int = 0

    def toggle_render_every_frame(self) -> None:
        self.render_every_frame = not self.render_every_frame

    def toggle_pause(self) -> None:
        self.is_paused = not self.is_paused

    def advance_by_one_step(self) -> None:
        self.steps_to_advance = 1
        self.is_paused = True

    def run_slower(self) -> None:
        self.run_speed /= 2.0
        if self.run_speed < 2**-4:
            self.run_speed = 2**-4

    def run_faster(self) -> None:
        self.run_speed *= 2.0
        if self.run_speed > 2**4:
            self.run_speed = 2**4
