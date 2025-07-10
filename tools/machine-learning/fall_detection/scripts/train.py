import enum

import click
import numpy as np
import plotly.graph_objects as go
import plotly.io as pio
import polars as pl
import tensorflow as tf
import wandb
from data_loading import load
from dataset import FallenDataset
from keras.callbacks import EarlyStopping
from keras.layers import (
    LSTM,
    BatchNormalization,
    Conv1D,
    Conv2D,
    Dense,
    Dropout,
    Flatten,
    InputLayer,
)
from keras.models import Sequential
from sklearn.metrics import confusion_matrix
from tensorflow import keras
from wandb.integration.keras import WandbMetricsLogger


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

    wandb.log({"confusion_matrix": fig})


def train_model(model, x_train, y_train, max_epochs: int) -> None:
    early_stopping = EarlyStopping(
        monitor="val_loss",
        patience=50,
        min_delta=0.001,
        mode="min",
    )

    model.fit(
        x_train,
        y_train,
        batch_size=256,
        epochs=max_epochs,
        validation_split=0.2,
        callbacks=[early_stopping, WandbMetricsLogger()],
    )


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
            InputLayer(shape=(num_features, input_length, 1)),
            Conv2D(
                filters=wandb.config["number_of_filters"][0],
                kernel_size=[num_features, wandb.config["kernel_widths"][0]],
                strides=[num_features, 1],
                padding="valid",
                activation="relu",
            ),
            # BatchNormalization(),
            # Conv2D(
            #     filters=16,
            #     kernel_size=[8, 4],
            #     padding="same",
            #     activation="relu",
            # ),
            Flatten(),
            BatchNormalization(),
            Dropout(0.2),
            Dense(
                wandb.config["dense_layer_sizes"][0],
                activation="relu",
            ),
            BatchNormalization(),
            Dense(
                wandb.config["dense_layer_sizes"][1],
                activation="relu",
            ),
            Dropout(0.2),
            Dense(num_classes),
        ]
    )

    # Compile model
    model.compile(
        optimizer="adam",
        loss=keras.losses.CategoricalCrossentropy(
            from_logits=True, name="categorical_crossentropy"
        ),
        metrics=["accuracy"],
    )

    if summary:
        model.summary()

    return model


# Build model
def build_sequential_model(
    num_features: int,
    input_length: int,
    num_classes: int,
    summary: bool = False,
):
    model = Sequential(
        [
            # ADD YOUR LAYERS HERE
            InputLayer(shape=(num_features, input_length)),
            LSTM(
                wandb.config["lstm_sizes"][0],
                dropout=0.4,
                return_sequences=True,
            ),
            BatchNormalization(),
            LSTM(wandb.config["lstm_sizes"][1], dropout=0.4),
            BatchNormalization(),
            Dense(
                wandb.config["dense_layer_sizes"][0],
                activation="relu",
            ),
            BatchNormalization(),
            Dropout(0.2),
            Dense(num_classes),
        ]
    )

    # Compile model
    model.compile(
        optimizer="adam",
        loss=keras.losses.CategoricalCrossentropy(
            from_logits=True, name="categorical_crossentropy"
        ),
        metrics=["accuracy"],
    )

    if summary:
        model.summary()

    return model


class ModelType(enum.Enum):
    Linear = enum.auto()
    Sequential = enum.auto()


def train(model_type: ModelType, data_path: str) -> None:
    df = load(data_path)
    dataset = FallenDataset(
        df,
        group_keys=[pl.col("robot_identifier"), pl.col("match_identifier")],
        features=[
            pl.col(
                "Control.main_outputs.sensor_data.inertial_measurement_unit.linear_acceleration.x"
            ),
            pl.col(
                "Control.main_outputs.sensor_data.inertial_measurement_unit.linear_acceleration.y"
            ),
            pl.col(
                "Control.main_outputs.sensor_data.inertial_measurement_unit.linear_acceleration.z"
            ),
            pl.col(
                "Control.main_outputs.sensor_data.inertial_measurement_unit.roll_pitch.x"
            ),
            pl.col(
                "Control.main_outputs.sensor_data.inertial_measurement_unit.roll_pitch.y"
            ),
            pl.col("Control.main_outputs.has_ground_contact"),
        ],
    )

    dataset.to_windowed(
        window_size=wandb.config["window_size"],
        window_stride=wandb.config["window_stride"],
        label_shift=wandb.config["label_shift"],
    )

    input_data = dataset.get_input_tensor()
    data_labels = dataset.get_labels_tensor()

    match model_type:
        case ModelType.Linear:
            print("Training linear model")
            model = build_linear_model(
                input_length=input_data.shape[2],
                num_features=input_data.shape[1],
                num_classes=dataset.n_classes(),
            )
        case ModelType.Sequential:
            print("Training sequential model")
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


@click.command()
@click.option(
    "--model-type",
    required=True,
    type=click.Choice(ModelType, case_sensitive=False),
)
@click.option("--data-path", default="data.parquet")
def main(model_type: ModelType, data_path: str) -> None:
    config = {
        "model_type": model_type,
        "window_size": 0.7,
        "window_stride": 10 / 83,
        "label_shift": 30,
        "number_of_filters": [32],
        "kernel_widths": [32],
        "dense_layer_sizes": [32, 16],
        "lstm_sizes": [64, 32],
    }
    with wandb.init(project="tinyml-fall-state-prediction", config=config):
        code = wandb.Artifact(name="code", type="code")
        code.add_dir("./scripts")
        code.save()

        train(model_type, data_path)


if __name__ == "__main__":
    main()
