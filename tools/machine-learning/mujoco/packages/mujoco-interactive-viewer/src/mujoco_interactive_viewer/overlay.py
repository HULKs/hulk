from dataclasses import dataclass


@dataclass
class Overlay:
    text1: str
    text2: str

    def __init__(self) -> None:
        self.text1 = ""
        self.text2 = ""
