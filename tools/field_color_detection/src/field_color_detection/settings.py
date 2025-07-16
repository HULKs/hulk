import multiprocessing
from enum import Enum
from typing import Literal

ColorValues = Enum(
    "ColorValues",
    [
        ("FIELD_COLOR", (0, 255, 0)),
        ("NOT_FIELD_COLOR", (255, 0, 0)),
        ("UNKNOWN", (0, 0, 0)),
    ],
)

GreyValues = Enum("GreyValues", {"BLACK": 0, "GREY": 127, "WHITE": 255})

Classes = Enum("Classes", {"NOT_FIELD": 0, "FIELD": 1, "UNKNOWN": 2})

FeatureIndices = Enum(
    "FeatureIndices",
    {
        "Y": 0,
        "Cr": 1,
        "Cb": 2,
        "B": 3,
        "G": 4,
        "R": 5,
        "b": 6,
        "g": 7,
        "r": 8,
        "I": 9,
        "L*": 10,
        "a*": 11,
        "b*": 12,
        "H": 13,
        "S": 14,
        "V": 15,
    },
)

Classifiers = Literal["DecisionTree"]
# [
#     "LinearSVM",
#     "DecisionTree",
#     "MLPClassifier",
#     "RBFSampler",
#     "NystroemRBF",
#     "NystroemPolynomial",
#     "Nystroem",
#     "PolynomialCountSketch",
#     "ZerothOrderOptimizer",
# ]
TextureMethods = Literal[
    "Neighbors", "HoG", "LBP", "GaborFilters", "NeighborsDifference"
]  # ["Neighbors", "HoG", "LBP", "GaborFilters"]

SHOW_PREDICTED_IMGS = True

HEIGHT = 480
WIDTH = 640

MAX_N_JOBS = max(1, multiprocessing.cpu_count() - 2)
