import enum

import click
import matplotlib.pyplot as plt
import numpy as np
import polars as pl
import tensorflow as tf
from data_loading import load
from dataset import FallenDataset
from keras.callbacks import EarlyStopping
from keras.layers import (
    LSTM,
    Conv1D,
    Dense,
    Dropout,
    Flatten,
    InputLayer,
)
from keras.models import Sequential
from sklearn.metrics import confusion_matrix
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


def split_data(input_data, labels):
    train_test_split = 0.8
    split_index = int(len(input_data) * train_test_split)

    x_train = input_data[:split_index]
    x_test = input_data[split_index + 1 :]

    y_train = labels[:split_index]
    y_test = labels[split_index + 1 :]

    return (x_train, y_train, x_test, y_test)


def evaluate_model(model, x_test, y_test) -> None:
    (test_loss, accuracy) = model.evaluate(x_test, y_test)
    print("Test accuracy: {}, test loss: {}", accuracy, test_loss)
    cm = confusion_matrix(
        np.argmax(y_test, axis=1), np.argmax(model.predict(x_test), axis=1)
    )
    cm = cm.astype("float") / cm.sum(axis=1)[:, np.newaxis]

    labels = ["Upright", "Falling", "Fallen"]
    import pandas as pd

    cm = pd.DataFrame(cm, index=labels, columns=labels)

    plt.figure(figsize=(3, 3))
    import seaborn as sns

    ax = sns.heatmap(
        cm * 100,
        annot=True,
        fmt=".1f",
        cmap="Blues",
        cbar=False,
    )
    ax.set_ylabel("True Class", fontdict={"fontweight": "bold"})
    ax.set_xlabel("Predicted Class", fontdict={"fontweight": "bold"})

    plt.show()


def train_model(model, x_train, y_train):
    early_stopping = EarlyStopping(
        monitor="val_loss",
        patience=50,
        min_delta=0.001,
        mode="min",
    )

    num_epochs = 300
    history = model.fit(
        x_train,
        y_train,
        batch_size=128,
        epochs=num_epochs,
        validation_split=0.2,
        callbacks=[early_stopping],
    )
    plot_training_history(history, 1)


# Build model
def build_linear_model(
    num_features: int,
    input_length: int,
    num_classes: int,
    summary: bool = False,
) -> None:
    model = Sequential(
        [
            # ADD YOUR LAYERS HERE
            InputLayer(shape=(num_features, input_length)),
            Conv1D(
                filters=32, kernel_size=32, padding="same", activation="relu"
            ),
            Flatten(),
            Dense(
                32,
                activation="relu",
            ),
            Dropout(0.2),
            Dense(num_classes, activation="softmax"),
        ]
    )

    # Compile model
    model.compile(
        optimizer="adam", loss="categorical_crossentropy", metrics=["accuracy"]
    )

    if summary:
        model.summary()

    return model


def train_linear() -> None:
    df = load("data.parquet")
    dataset = FallenDataset(
        df,
        group_keys=[pl.col("robot_identifier"), pl.col("match_identifier")],
        features=[
            pl.col("Control.main_outputs.robot_orientation.pitch"),
            pl.col("Control.main_outputs.robot_orientation.roll"),
            pl.col("Control.main_outputs.robot_orientation.yaw"),
            pl.col("Control.main_outputs.has_ground_contact"),
        ],
    )
    dataset.to_windowed(window_size=0.7, window_stride=10 / 83)

    input_data = dataset.get_input_tensor()
    data_labels = dataset.get_labels_tensor()

    model = build_linear_model(
        input_length=input_data.shape[2],
        num_features=input_data.shape[1],
        num_classes=dataset.n_classes(),
    )

    (x_train, y_train, x_test, y_test) = split_data(input_data, data_labels)

    y_train = keras.utils.to_categorical(y_train, dataset.n_classes())
    y_test = keras.utils.to_categorical(y_test, dataset.n_classes())

    print(x_train.shape)
    print(y_train.shape)

    train_model(model, x_train, y_train)

    evaluate_model(model, x_test, y_test)

    converter = tf.lite.TFLiteConverter.from_keras_model(model)
    model_tflite = converter.convert()
    with open("../../../etc/neural_networks/fall_detection.tflite", "wb") as f:
        f.write(model_tflite)


# Build model
def build_sequential_model(
    num_features: int,
    input_length: int,
    num_classes: int,
    summary: bool = False,
) -> None:
    model = Sequential(
        [
            # ADD YOUR LAYERS HERE
            InputLayer(shape=(num_features, input_length)),
            LSTM(
                128,
                return_sequences=True,
            ),
            LSTM(64, dropout=0.2),
            Dense(
                32,
                activation="relu",
            ),
            Dense(num_classes, activation="softmax"),
        ]
    )

    # Compile model
    model.compile(
        optimizer="adam", loss="categorical_crossentropy", metrics=["accuracy"]
    )

    if summary:
        model.summary()

    return model


def train_sequential() -> None:
    df = load("data.parquet")
    dataset = FallenDataset(
        df,
        group_keys=[pl.col("robot_identifier"), pl.col("match_identifier")],
        features=[
            pl.col("Control.main_outputs.robot_orientation.pitch"),
            pl.col("Control.main_outputs.robot_orientation.roll"),
            pl.col("Control.main_outputs.robot_orientation.yaw"),
            pl.col("Control.main_outputs.has_ground_contact"),
        ],
    )

    dataset.to_windowed(window_size=1.0, window_stride=5 / 83)

    input_data = dataset.get_input_tensor()
    data_labels = dataset.get_labels_tensor()

    model = build_sequential_model(
        input_length=input_data.shape[2],
        num_features=input_data.shape[1],
        num_classes=dataset.n_classes(),
    )

    (x_train, y_train, x_test, y_test) = split_data(input_data, data_labels)

    y_train = keras.utils.to_categorical(y_train, dataset.n_classes())
    y_test = keras.utils.to_categorical(y_test, dataset.n_classes())

    print(x_train.shape)
    print(y_train.shape)

    train_model(model, x_train, y_train)

    evaluate_model(model, x_test, y_test)

    converter = tf.lite.TFLiteConverter.from_keras_model(model)
    converter._experimental_lower_tensor_list_ops = False
    converter.optimizations = [tf.lite.Optimize.DEFAULT]
    converter.target_spec.supported_ops = [
        tf.lite.OpsSet.TFLITE_BUILTINS,  # enable TensorFlow Lite ops.
        tf.lite.OpsSet.SELECT_TF_OPS,  # enable TensorFlow ops.
    ]
    model_tflite = converter.convert()
    with open("../../../etc/neural_networks/fall_detection.tflite", "wb") as f:
        f.write(model_tflite)


class ModelType(enum.Enum):
    Linear = enum.auto()
    Sequential = enum.auto()


@click.command()
@click.option(
    "--model-type",
    required=True,
    type=click.Choice(ModelType, case_sensitive=False),
)
def main(model_type: ModelType) -> None:
    match model_type:
        case ModelType.Linear:
            print("Training linear model")
            train_linear()
        case ModelType.Sequential:
            print("Training sequential model")
            train_sequential()


if __name__ == "__main__":
    main()
