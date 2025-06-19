from dataclasses import dataclass
import polars as pl


@dataclass
class PseudoLabelParameters:
    pitch_threshold: float = 1.0

LABELS = {
    0: "Other",
    1: "Fallen",
}

class PseudoLabeller:
    def __init__(self, parameters: PseudoLabelParameters | None = None):
        self.parameters = parameters or PseudoLabelParameters()

    def generate_labels(self, data: pl.DataFrame) -> pl.Series:
        labels = data.select(
            self.map_to_schema(
                pl.col("Control.main_outputs.robot_orientation.pitch").abs()
                > self.parameters.pitch_threshold
            )
        )
        return labels.to_series()

    def map_to_schema(self, expression: pl.Expr) -> pl.Expr:
        return (
            pl.when(expression)
            .then(pl.lit(1))
            .otherwise(pl.lit(0))
        )
        
