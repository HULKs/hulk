import matplotlib.pyplot as plt
import numpy as np
import optuna
from numpy.typing import NDArray
from plotly.io import show
from sklearn import metrics
from sklearn.base import ClassifierMixin

from .settings import HEIGHT, WIDTH, Classes


def show_boxplot(
    model: ClassifierMixin, X: NDArray[np.uint8], y: NDArray[np.uint8]
) -> None:
    predictions = model.predict(X)
    predictions_reshaped = predictions.reshape((-1, HEIGHT, WIDTH))
    y_reshaped = y.reshape((-1, HEIGHT, WIDTH))
    f1_scores = []
    balanced_accuracies = []
    jaccard_scores = []
    fbeta_scores = []
    recall_scores = []
    precision_scores = []
    for i, predicted_image in enumerate(predictions_reshaped):
        mask = y_reshaped[i] != Classes.UNKNOWN.value
        y_i = y_reshaped[i][mask]
        prediction_i = predicted_image[mask]
        f1_scores.append(metrics.f1_score(y_i, prediction_i))
        balanced_accuracies.append(
            metrics.balanced_accuracy_score(y_i, prediction_i)
        )
        jaccard_scores.append(metrics.jaccard_score(y_i, prediction_i))
        recall_scores.append(metrics.recall_score(y_i, prediction_i))
        precision_scores.append(metrics.precision_score(y_i, prediction_i))
        fbeta_scores.append(metrics.fbeta_score(y_i, prediction_i, beta=2))

    labels = [
        "F1 Score",
        "Balanced\nAccuracy",
        "Jaccard\nScore",
        "F-beta Score",
        "Recall",
        "Precision",
    ]

    _, ax = plt.subplots()
    ax.boxplot(
        [
            f1_scores,
            balanced_accuracies,
            jaccard_scores,
            fbeta_scores,
            recall_scores,
            precision_scores,
        ],
        patch_artist=True,
        tick_labels=labels,
    )
    plt.show()


def show_pareto_front(study_name: str, study_location: str, title: str) -> None:
    study = optuna.load_study(study_name=study_name, storage=study_location)
    fig = optuna.visualization.plot_pareto_front(
        study,
        targets=lambda t: (t.values[0], t.values[1]),
        target_names=["F2 Score", "#Selected Color Channels"],
    )
    fig.update_layout(title=title)

    show(fig)


if __name__ == "__main__":
    show_pareto_front(
        "testVal_DecisionTree",
        "sqlite:////home/franziska-sophie/Downloads/sampled_images/db.sqlite3_remoteCompiler",
        "Decision Tree",
    )
    show_pareto_front(
        "testVal_LinearSVM",
        "sqlite:////home/franziska-sophie/Downloads/sampled_images/db.sqlite3_remoteCompiler",
        "Linear SVM",
    )
    show_pareto_front(
        "testVal_NystroemRBF",
        "sqlite:////home/franziska-sophie/Downloads/sampled_images/db.sqlite3_rechenknecht",
        "Nyström (BRF)",
    )
    show_pareto_front(
        "testVal_NystroemPolynomial",
        "sqlite:////home/franziska-sophie/Downloads/sampled_images/db.sqlite3_rechenknecht",
        "Nyström (Polynomial)",
    )
