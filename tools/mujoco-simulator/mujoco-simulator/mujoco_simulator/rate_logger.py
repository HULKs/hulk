import logging
import time
from datetime import timedelta


class SimulationRateLogger:
    def __init__(self, log_rate: timedelta) -> None:
        self.log_rate = log_rate
        self.last_log = None
        self.steps_since_last_log = 0

    def step(self) -> None:
        self.steps_since_last_log += 1
        now = time.time()
        if self.last_log is None:
            self.last_log = now

        if now - self.last_log >= self.log_rate.total_seconds():
            rate = self.steps_since_last_log / self.log_rate.total_seconds()
            logging.info(f"Simulation [steps/second]: {int(rate)}")
            self.steps_since_last_log = 0
            self.last_log = now
