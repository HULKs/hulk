import typing
import uuid
from enum import Enum


def create_curve(name: str = "Curve", enabled: bool = True, key: str = "",
                 key_lambda: str = "output = input", color: str = "#000000"):
    return {
        "name": name,
        "identifier": str(uuid.uuid4()),
        "enabled": enabled,
        "key": key,
        "key_lambda": key_lambda,
        "color": color
    }


class TabType(Enum):
    plot = 0
    config = 1


def select_curve(index: int, model: typing.Dict):
    if index < len(model["curves"]):
        model["selected_curve"] = index
    else:
        model["selected_curve"] = None

