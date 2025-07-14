from sklearn.linear_model import SGDClassifier


class LinearSVM:
    def __init__(self, batch_size: int = 1024):
        self.batch_size = batch_size
        self.classifier = SGDClassifier(loss="hinge")

    def fit(self, X, y):
        for i in range(0, len(X), self.batch_size):
            X_batch = X[i : i + self.batch_size]
            y_batch = y[i : i + self.batch_size]

            if i == 0:
                self.classifier.partial_fit(X_batch, y_batch, classes=[0, 1])
            else:
                self.classifier.partial_fit(X_batch, y_batch)
        return self

    def predict(self, X):
        return self.classifier.predict(X)
