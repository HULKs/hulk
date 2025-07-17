import enum
from pathlib import Path

import click
import numpy as np
import plotly as pt
import plotly.express as px
import plotly.graph_objects as go
import plotly.io as pio
import polars as pl
import polars.selectors as cs
import tensorflow as tf
import wandb
from data_loading import load
from dataset import FallenDataset
from dataset.dataset import (
    do_class_balancing,
    generate_selector_up_to_index,
    get_input_tensor,
    get_input_tensor_up_to_shift,
    get_labels_tensor,
    get_labels_tensor_up_to_shift,
)
from keras.callbacks import EarlyStopping
from keras.layers import (
    LSTM,
    BatchNormalization,
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


def evaluate_model(model: keras.Model, x_test, y_test) -> tuple[float, float]:
    (test_loss, accuracy) = model.evaluate(x_test, y_test)
    print(f"Test accuracy: {accuracy}, test loss: {test_loss}")

#    # Generate confusion matrix
#    cm = confusion_matrix(
#        np.argmax(y_test, axis=1), np.argmax(model.predict(x_test), axis=1)
#    )
#
#    # Normalize confusion matrix
#    cm = cm.astype("float") / cm.sum(axis=1)[:, np.newaxis]
#
#    # Define labels
#    labels = ["Stable", "SoonToBeUnstable"]
#
#    # Convert to percentage for display
#    cm_percent = cm * 100
#
#    # Create annotations for the heatmap
#    annotations = []
#    for i in range(len(labels)):
#        for j in range(len(labels)):
#            annotations.append(
#                {
#                    "x": j,
#                    "y": i,
#                    "text": f"{cm_percent[i][j]:.1f}",
#                    "showarrow": False,
#                    "font": {
#                        "color": "white" if cm_percent[i][j] > 50 else "black",
#                        "size": 14,
#                    },
#                }
#            )
#
#    # Create heatmap using Plotly
#    fig = go.Figure(
#        data=go.Heatmap(
#            z=cm_percent,
#            x=labels,
#            y=labels,
#            colorscale="Blues",
#            showscale=False,  # Equivalent to cbar=False
#            hoverongaps=False,
#            hovertemplate="True: %{y}<br>Predicted: %{x}<br>Value: %{z:.1f}%<extra></extra>",
#        )
#    )
#
#    # Add annotations
#    fig.update_layout(
#        annotations=annotations,
#        title="Confusion Matrix",
#        xaxis_title="<b>Predicted Class</b>",
#        yaxis_title="<b>True Class</b>",
#        width=400,
#        height=400,
#        xaxis={"side": "bottom"},
#        yaxis={
#            "autorange": "reversed"
#        },  # Reverse y-axis to match seaborn style
#    )
#
#    wandb.log({"confusion_matrix": fig})

    return (test_loss, accuracy)


def train_model(model, x_train, y_train, max_epochs: int) -> None:
    early_stopping = EarlyStopping(
        monitor="val_loss",
        patience=100,
        mode="min",
    )

    model.fit(
        x_train,
        y_train,
        batch_size=128,
        epochs=max_epochs,
        validation_split=0.2,
        callbacks=[early_stopping, WandbMetricsLogger()],
    )


# Build model
def build_linear_model(
    input_length: int,
    num_features: int,
    num_classes: int,
    run: wandb.run,
    summary: bool = False,
) -> None:
    model = Sequential(
        [
            # ADD YOUR LAYERS HERE
            InputLayer(shape=(input_length, num_features, 1)),
            Conv2D(
                filters=run.config["number_of_filters"][0],
                kernel_size=[run.config["kernel_widths"][0], num_features],
                strides=[input_length, 1],
                padding="valid",
                activation="relu",
            ),
            BatchNormalization(),
            # Conv2D(
            #     filters=16,
            #     kernel_size=[8, 4],
            #     padding="same",
            #     activation="relu",
            # ),
            Flatten(),
            # BatchNormalization(),
            Dropout(0.45),
            Dense(
                run.config["dense_layer_sizes"][0],
                activation="relu",
            ),
            BatchNormalization(),
            Dense(
                run.config["dense_layer_sizes"][1],
                activation="relu",
            ),
            Dropout(0.35),
            Dense(1),
        ]
    )

    # Compile model
    model.compile(
        optimizer="adam",
        loss=keras.losses.BinaryCrossentropy(
            from_logits=True, name="binary_crossentropy"
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
    run: wandb.run,
    summary: bool = False,
):
    model = Sequential(
        [
            # ADD YOUR LAYERS HERE
            InputLayer(shape=(input_length, num_features)),
            LSTM(
                run.config["lstm_sizes"][0],
                dropout=0.4,
                return_sequences=True,
            ),
            BatchNormalization(),
            LSTM(run.config["lstm_sizes"][1], dropout=0.4),
            BatchNormalization(),
            Dense(
                run.config["dense_layer_sizes"][0],
                activation="relu",
            ),
            BatchNormalization(),
            Dropout(0.2),
            Dense(1),
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


def train(
    model_type: ModelType,
    data_path: str,
    use_cache: bool,
    config: dict,
    number_of_classes: int,
) -> None:
    number_of_label_shifts = config["label_shift"]
    features = [
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
    ]
    train_windowed_filtered_dataframe_cache_path = Path(
        "./.cache/train_windowed_filtered_dataframe.parquet"
    )

    test_windowed_filtered_dataframe_cache_path = Path(
        "./.cache/test_windowed_filtered_dataframe.parquet"
    )

    samples_per_window = int(config["window_size"] * 83)

    if (
        train_windowed_filtered_dataframe_cache_path.exists()
        and test_windowed_filtered_dataframe_cache_path.exists()
        and use_cache
    ):
        train_windowed_filtered_dataframe = pl.read_parquet(
            train_windowed_filtered_dataframe_cache_path
        )
        test_windowed_filtered_dataframe = pl.read_parquet(
            test_windowed_filtered_dataframe_cache_path
        )
    else:
        df = load(data_path)
        dataset = FallenDataset(
            df,
            group_keys=[
                pl.col("robot_identifier"),
                pl.col("game_phase_identifier"),
                pl.col("match_identifier"),
            ],
            features=features,
        )

        dataset.to_windowed(
            window_size=config["window_size"],
            window_stride=config["window_stride"],
            label_shift=config["label_shift"],
        )
        train_windowed_filtered_dataframe = (
            dataset.train_windowed_filtered_dataframe
        )
        test_windowed_filtered_dataframe = (
            dataset.test_windowed_filtered_dataframe
        )

    input_datas = [
        get_input_tensor_up_to_shift(
            train_windowed_filtered_dataframe,
            features,
            samples_per_window,
            label_shift=shift,
        )
        for shift in range(1, config["label_shift"] - 1)
    ]
    data_labelss = [
        get_labels_tensor_up_to_shift(
            train_windowed_filtered_dataframe,
            shift,
        )
        for shift in range(1, config["label_shift"] - 1)
    ]

    label_shifts = []
    metrics = []
    for label_shift, (train_input_data, data_labels) in enumerate(
        zip(input_datas, data_labelss)
    ):
        if label_shift < 2:
            continue
        config["label_shift"] = label_shift
        with wandb.init(
            project="tinyml-fall-state-prediction",
            config=config,
        ) as run:
            balanced_test_dataframe = do_class_balancing(
                test_windowed_filtered_dataframe, run.config["label_shift"]
            )

            balanced_test_input_data = balanced_test_dataframe.select(
                generate_selector_up_to_index(features, samples_per_window)
            )

            test_input_data = get_input_tensor(
                input_data=balanced_test_input_data,
                samples_per_window=samples_per_window,
                features=features,
            )

            balanced_test_labels = balanced_test_dataframe.select(
                cs.starts_with("labels")
                & cs.ends_with("_" + str(run.config["label_shift"]))
            )

            test_labels = get_labels_tensor(balanced_test_labels)
            match model_type:
                case ModelType.Linear:
                    print("Training linear model")
                    model = build_linear_model(
                        num_features=train_input_data.shape[2],
                        input_length=train_input_data.shape[1],
                        num_classes=number_of_classes,
                        run=run,
                    )
                case ModelType.Sequential:
                    print("Training sequential model")
                    model = build_sequential_model(
                        num_features=train_input_data.shape[2],
                        input_length=train_input_data.shape[1],
                        num_classes=number_of_classes,
                        run=run,
                    )

            categorical_train_labels = keras.utils.to_categorical(
                data_labels, number_of_classes
            )
            categorical_test_labels = keras.utils.to_categorical(
                test_labels, number_of_classes
            )

            print(f"Input shape: {train_input_data.shape}")
            print(f"Label shape: {categorical_train_labels.shape}")

            train_model(
                model,
                train_input_data,
                data_labels,
                max_epochs=300,
            )

            print(f"Test input shape: {test_input_data.shape}")
            print(f"Label shape: {categorical_test_labels.shape}")

            metric = evaluate_model(
                model, test_input_data, test_labels
            )
            label_shifts.append(label_shift)
            metrics.append(metric)
            
            
            converter = tf.lite.TFLiteConverter.from_keras_model(model)
            converter._experimental_lower_tensor_list_ops = False
            converter.optimizations = [tf.lite.Optimize.DEFAULT]
            converter.target_spec.supported_ops = [
                tf.lite.OpsSet.TFLITE_BUILTINS,  # enable TensorFlow Lite ops.
                tf.lite.OpsSet.SELECT_TF_OPS,  # enable TensorFlow ops.
            ]
            converter.allow_custom_ops = True
            model_tflite = converter.convert()
            with open(
                f"../../../etc/neural_networks/fall_detection_shift_{label_shift}.tflite",
                "wb",
            ) as f:
                f.write(model_tflite)

    pio.renderers.default = "browser"
    unzipped_metrics = list(zip(*metrics))
    test_losses = unzipped_metrics[0]
    test_accuracies = unzipped_metrics[1]
    fig1 = px.scatter(
        x=label_shifts,
        y=test_losses,
        labels={"x": "label shift", "y": "test loss"},
    )
    fig2 = px.scatter(
        x=label_shifts,
        y=test_accuracies,
        labels={"x": "label shift", "y": "test accuracy"},
    )

    pt.offline.plot(fig1, filename="./test_loss.html")
    pt.offline.plot(fig2, filename="./test_accuracies.html")

    fig1.show()
    fig2.show()

@click.command()
@click.option(
    "--model-type",
    required=True,
    type=click.Choice(ModelType, case_sensitive=False),
)
@click.option("--data-path", default="data.parquet")
@click.option("--use-cache", is_flag=True, default=False)
def main(model_type: ModelType, data_path: str, use_cache: bool) -> None:
    config = {
        "model_type": model_type,
        "window_size": 70 / 83,
        "window_stride": 1 / 83,
        "label_shift": 30,
        "number_of_filters": [12],
        "kernel_widths": [10],
        "dense_layer_sizes": [24, 16],
        "lstm_sizes": [32, 16],
    }
    assert config["window_size"] * 83 > config["kernel_widths"][0]
    code = wandb.Artifact(name="code", type="code")
    code.add_dir("./scripts")
    code.save()

    train(model_type, data_path, use_cache, config, number_of_classes=2)


if __name__ == "__main__":
    main()
