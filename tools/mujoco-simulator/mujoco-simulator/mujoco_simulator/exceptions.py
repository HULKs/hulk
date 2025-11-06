from mujoco_rust_server import TaskName


class UnknownTaskException(ValueError):
    def __init__(self, task_name: TaskName) -> None:
        self.task_name = task_name
