import os
from datetime import datetime

import matplotlib.pyplot as plt
import numpy as np
import polars as pl
import tensorflow as tf
from keras.callbacks import EarlyStopping
from keras.layers import (
    BatchNormalization,
    Conv1D,
    Dense,
    Dropout,
    Flatten,
    InputLayer,
    MaxPooling1D,
    ReLU,
    Reshape,
    Softmax,
)
from keras.models import Sequential
from keras.optimizers.legacy import Adam
from tensorflow import keras


def plot_training_history(history, model_name) -> None:
    fig, (ax1, ax2) = plt.subplots(1, 2)
    fig.suptitle(f"Model {model_name}")
    fig.set_figwidth(15)

    ax1.plot(
        range(1, len(history.history["accuracy"]) + 1),
        history.history["accuracy"],
    )
    ax1.plot(
        range(1, len(history.history["val_accuracy"]) + 1),
        history.history["val_accuracy"],
    )
    ax1.set_title("Model accuracy")
    ax1.set(xlabel="epoch", ylabel="accuracy")
    ax1.legend(["training", "validation"], loc="best")

    ax2.plot(
        range(1, len(history.history["loss"]) + 1), history.history["loss"]
    )
    ax2.plot(
        range(1, len(history.history["val_loss"]) + 1),
        history.history["val_loss"],
    )
    ax2.set_title("Model loss")
    ax2.set(xlabel="epoch", ylabel="loss")
    ax2.legend(["training", "validation"], loc="best")
    plt.show()


# Build model
def build_model(num_classes: int, summary: bool = True):
    model = Sequential()

    # ADD YOUR LAYERS HERE
    model.add(Conv1D(3, kernel_size=5, padding="same", activation="relu"))
    model.add(MaxPooling1D(pool_size=2, strides=2, padding="same"))
    model.add(Dropout(0.25))
    model.add(Flatten())
    model.add(
        Dense(
            40,
            activation="relu",
            activity_regularizer=tf.keras.regularizers.l1(0.00001),
        )
    )
    model.add(
        Dense(
            20,
            activation="relu",
            activity_regularizer=tf.keras.regularizers.l1(0.00001),
        )
    )
    model.add(
        Dense(
            10,
            activation="relu",
            activity_regularizer=tf.keras.regularizers.l1(0.00001),
        )
    )
    model.add(Dropout(0.15))
    model.add(Dense(num_classes, activation="softmax"))

    # Compile model
    model.compile(
        optimizer="adam", loss="categorical_crossentropy", metrics=["accuracy"]
    )

    if summary:
        model.summary()

    return model


def train_model(model, x_train, y_train, x_test, y_test):
    early_stopping = EarlyStopping(
        monitor="val_loss",
        patience=50,
        min_delta=0.001,
        mode="min",
    )

    print(f"x_train: {x_train.shape}")
    print(f"y_train: {y_train.shape}")

    num_epochs = 10
    history = model.fit(
        x_train,
        y_train,
        batch_size=128,
        epochs=num_epochs,
        validation_data=(x_test, y_test),
        callbacks=[early_stopping],
    )
    # plot_training_history(history, 1)


def dummy_data(labels):
    number_of_samples = 1000
    control_frequency = 1 / 83
    window_size_s = 1.5
    window_stride_s = 0.2
    samples_per_window = int(window_size_s / control_frequency)
    number_of_windows = int(
        number_of_samples / control_frequency / window_stride_s
    )

    print(number_of_windows)
    print(samples_per_window)

    rng = np.random.default_rng()

    input_data = rng.random(size=(number_of_windows, samples_per_window, 4))
    data_labels = rng.integers(low=0, high=3, size=number_of_windows)

    return (input_data, data_labels)


def split_data(input_data, labels):
    train_test_split = 0.8
    split_index = int(len(input_data) * train_test_split)

    x_train = input_data[::split_index]
    x_test = input_data[split_index + 1 : :]

    y_train = labels[::split_index]
    y_test = labels[split_index + 1 : :]

    return (x_train, y_train, x_test, y_test)


if __name__ == "__main__":
    # df = load("data.parquet")
    labels = ["Unknown", "Upright", "Falling", "Fallen"]

    model = build_model(len(labels))

    (input_data, data_labels) = dummy_data(labels)
    (x_train, y_train, x_test, y_test) = split_data(input_data, data_labels)

    y_train = keras.utils.to_categorical(y_train, len(labels))
    y_test = keras.utils.to_categorical(y_test, len(labels))

    train_model(model, x_train, y_train, x_test, y_test)
    export_path = "saved_model.keras"
    model.save(export_path)

    converter = tf.lite.TFLiteConverter.from_keras_model(model)
    model_tflite = converter.convert()
    with open("../../../etc/neural_networks/base_model.tflite", "wb") as f:
        f.write(model_tflite)
