import numpy as np
from sklearn.kernel_approximation import (
    Nystroem,
)
from sklearn.linear_model import SGDClassifier


class NystroemApprox:
    def __init__(
        self,
        kernel: str = "rbf",
        n_components: int = 100,
        n_jobs: int = 1,
        gamma: float | None = 0.01,
        class_weight: str | None = None,
        batch_size: int = 1000,
        sample_size: int = 100000,
        degree: int | None = None,
        alpha: float = 0.0001,
    ) -> None:
        self.kernel = kernel
        self.gamma = gamma
        self.n_components = n_components
        self.n_jobs = n_jobs
        self.class_weight = class_weight
        self.batch_size = batch_size
        self.sample_size = sample_size
        self.degree = degree

        self.nystroem = Nystroem(
            kernel=kernel,
            gamma=gamma,
            n_components=n_components,
            n_jobs=n_jobs,
            degree=degree,
        )

        # self.classifier = svm.LinearSVC(class_weight=self.class_weight)
        self.classifier = SGDClassifier(
            loss="hinge", alpha=alpha
        )  # 'hinge' = linear SVM

    def fit(self, X, y):
        subset_idx = np.random.choice(
            X.shape[0], size=min(self.sample_size, len(X)), replace=False
        )
        self.nystroem.fit(X[subset_idx])
        print("--------------> Finished fitting nystroem")
        # X = self._batched_transform(self.nystroem, X).astype(np.float32)
        X = self.nystroem.transform(X).astype(np.float32)
        print("--------------> Finished transforming data")

        # scaler = StandardScaler()
        # X = scaler.fit(X)
        # X = self._batched_transform(scaler, X).astype(np.float32)
        # print("--------------> Finished scaling data")

        # self.classifier.fit(X, y)
        for i in range(0, len(X), self.batch_size):
            X_batch = X[i : i + self.batch_size]
            y_batch = y[i : i + self.batch_size]

            if i == 0:
                self.classifier.partial_fit(X_batch, y_batch, classes=[0, 1])
            else:
                self.classifier.partial_fit(X_batch, y_batch)
        print("--------------> Finished fitting classifier")
        return self

    def _batched_transform(self, transformer, X):
        out = []
        for i in range(0, len(X), self.batch_size):
            out.append(
                transformer.transform(
                    X[i : i + self.batch_size].astype(np.float32)
                )
            )
        return np.vstack(out)

    def get_params(self, deep=True):
        return {
            "kernel": self.kernel,
            "gamma": self.gamma,
            "n_components": self.n_components,
            "n_jobs": self.n_jobs,
            "class_weight": self.class_weight,
            "batch_size": self.batch_size,
            "sample_size": self.sample_size,
            "degree": self.degree,
            "alpha": self.classifier.alpha,
        }

    def predict(self, X):
        X_transformed = self._batched_transform(self.nystroem, X)
        return self.classifier.predict(X_transformed)

    def set_params(self, **params):
        main_params = {}
        classifier_params = {}

        for param, value in params.items():
            if hasattr(self, param):
                main_params[param] = value
            else:
                classifier_params[param] = value

        for param, value in main_params.items():
            setattr(self, param, value)

        if classifier_params:
            self.classifier.set_params(**classifier_params)

        if any(
            p in main_params
            for p in ["gamma", "kernel", "n_components", "degree"]
        ):
            self.nystroem = Nystroem(
                kernel=self.kernel,
                gamma=self.gamma,
                n_components=self.n_components,
                n_jobs=self.n_jobs,
                degree=self.degree,
            )

        return self
