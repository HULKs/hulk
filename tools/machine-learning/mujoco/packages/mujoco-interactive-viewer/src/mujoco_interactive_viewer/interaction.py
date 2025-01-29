from dataclasses import dataclass

import glfw

DOUBLE_CLICK_INTERVAL = 0.3


@dataclass
class InteractionState:
    left_mouse_button_pressed: bool = False
    right_mouse_button_pressed: bool = False
    left_double_click_pressed: bool = False
    right_double_click_pressed: bool = False
    last_left_click_time: float | None = None
    last_right_click_time: float | None = None
    last_mouse_x: float = 0.0
    last_mouse_y: float = 0.0

    def detect_click(self, button: int, action: int) -> None:
        self.left_mouse_button_pressed = (
            button == glfw.MOUSE_BUTTON_LEFT and action == glfw.PRESS
        )
        self.right_mouse_button_pressed = (
            button == glfw.MOUSE_BUTTON_RIGHT and action == glfw.PRESS
        )

    def detect_double_click(self) -> None:
        self.left_double_click_pressed = False
        self.right_double_click_pressed = False
        time_now = glfw.get_time()

        if self.left_mouse_button_pressed:
            if self.last_left_click_time is None:
                self.last_left_click_time = glfw.get_time()

            time_diff = time_now - self.last_left_click_time
            if time_diff > 0.01 and time_diff < DOUBLE_CLICK_INTERVAL:
                self.left_double_click_pressed = True
            self.last_left_click_time = time_now

        if self.right_mouse_button_pressed:
            if self.last_right_click_time is None:
                self.last_right_click_time = glfw.get_time()

            time_diff = time_now - self.last_right_click_time
            if time_diff > 0.01 and time_diff < 0.2:
                self.right_double_click_pressed = True
            self.last_right_click_time = time_now
