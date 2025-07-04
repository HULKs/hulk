import enum

import click
import numpy as np
import plotly.graph_objects as go
import plotly.io as pio
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
from plotly.subplots import make_subplots
from sklearn.metrics import confusion_matrix
from tensorflow import keras


def plot_training_history(history, model_name) -> None:
    # Create subplots with 1 row and 2 columns
    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=("Model accuracy", "Model loss"),
        horizontal_spacing=0.1,
    )

    # Add accuracy traces
    fig.add_trace(
        go.Scatter(
            x=list(range(1, len(history.history["accuracy"]) + 1)),
            y=history.history["accuracy"],
            mode="lines+markers",
            name="training",
            line={"color": "blue"},
            legendgroup="accuracy",
        ),
        row=1,
        col=1,
    )

    fig.add_trace(
        go.Scatter(
            x=list(range(1, len(history.history["val_accuracy"]) + 1)),
            y=history.history["val_accuracy"],
            mode="lines+markers",
            name="validation",
            line={"color": "orange"},
            legendgroup="accuracy",
        ),
        row=1,
        col=1,
    )

    # Add loss traces
    fig.add_trace(
        go.Scatter(
            x=list(range(1, len(history.history["loss"]) + 1)),
            y=history.history["loss"],
            mode="lines+markers",
            name="training",
            line={"color": "blue"},
            legendgroup="loss",
            showlegend=False,  # Hide duplicate legend entries
        ),
        row=1,
        col=2,
    )

    fig.add_trace(
        go.Scatter(
            x=list(range(1, len(history.history["val_loss"]) + 1)),
            y=history.history["val_loss"],
            mode="lines+markers",
            name="validation",
            line={"color": "orange"},
            legendgroup="loss",
            showlegend=False,  # Hide duplicate legend entries
        ),
        row=1,
        col=2,
    )

    # Update layout
    fig.update_layout(
        title=f"Model {model_name}",
        width=900,  # Equivalent to figwidth=15 in matplotlib
        height=400,
        legend={
            "orientation": "h",
            "yanchor": "bottom",
            "y": 1.02,
            "xanchor": "right",
            "x": 1,
        },
    )

    # Update x and y axis labels
    fig.update_xaxes(title_text="epoch", row=1, col=1)
    fig.update_yaxes(title_text="accuracy", row=1, col=1)
    fig.update_xaxes(title_text="epoch", row=1, col=2)
    fig.update_yaxes(title_text="loss", row=1, col=2)

    # Show the plot
    fig.show()


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
    print(f"Test accuracy: {accuracy}, test loss: {test_loss}")

    # Generate confusion matrix
    cm = confusion_matrix(
        np.argmax(y_test, axis=1), np.argmax(model.predict(x_test), axis=1)
    )

    # Normalize confusion matrix
    cm = cm.astype("float") / cm.sum(axis=1)[:, np.newaxis]

    # Define labels
    labels = ["Upright", "Falling", "Fallen"]

    # Convert to percentage for display
    cm_percent = cm * 100

    # Create annotations for the heatmap
    annotations = []
    for i in range(len(labels)):
        for j in range(len(labels)):
            annotations.append(
                {
                    "x": j,
                    "y": i,
                    "text": f"{cm_percent[i][j]:.1f}",
                    "showarrow": False,
                    "font": {
                        "color": "white" if cm_percent[i][j] > 50 else "black",
                        "size": 14,
                    },
                }
            )

    # Create heatmap using Plotly
    fig = go.Figure(
        data=go.Heatmap(
            z=cm_percent,
            x=labels,
            y=labels,
            colorscale="Blues",
            showscale=False,  # Equivalent to cbar=False
            hoverongaps=False,
            hovertemplate="True: %{y}<br>Predicted: %{x}<br>Value: %{z:.1f}%<extra></extra>",
        )
    )

    # Add annotations
    fig.update_layout(
        annotations=annotations,
        title="Confusion Matrix",
        xaxis_title="<b>Predicted Class</b>",
        yaxis_title="<b>True Class</b>",
        width=400,
        height=400,
        xaxis={"side": "bottom"},
        yaxis={
            "autorange": "reversed"
        },  # Reverse y-axis to match seaborn style
    )

    # Show the plot
    fig.show()


def train_model(model, x_train, y_train, max_epochs: int):
    early_stopping = EarlyStopping(
        monitor="val_loss",
        patience=50,
        min_delta=0.001,
        mode="min",
    )

    history = model.fit(
        x_train,
        y_train,
        batch_size=128,
        epochs=max_epochs,
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

    print(f"Input shape: {x_train.shape}")
    print(f"Label shape: {y_train.shape}")

    train_model(model, x_train, y_train, max_epochs=300)

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

    print(f"Input shape: {x_train.shape}")
    print(f"Label shape: {y_train.shape}")

    train_model(model, x_train, y_train, max_epochs=300)

    evaluate_model(model, x_test, y_test)

    converter = tf.lite.TFLiteConverter.from_keras_model(model)
    converter._experimental_lower_tensor_list_ops = False
    converter.optimizations = [tf.lite.Optimize.DEFAULT]
    converter.target_spec.supported_ops = [
        tf.lite.OpsSet.TFLITE_BUILTINS,  # enable TensorFlow Lite ops.
        tf.lite.OpsSet.SELECT_TF_OPS,  # enable TensorFlow ops.
    ]
    converter.allow_custom_ops = True
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
    pio.renderers.default = "browser"

    match model_type:
        case ModelType.Linear:
            print("Training linear model")
            train_linear()
        case ModelType.Sequential:
            print("Training sequential model")
            train_sequential()


if __name__ == "__main__":
    main()
