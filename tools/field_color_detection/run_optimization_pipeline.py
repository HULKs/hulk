import argparse
import os
from datetime import datetime
from typing import get_args

import joblib
import numpy as np
import optuna
from sklearn import metrics
from sklearn.model_selection import GridSearchCV
from src.field_color_detection import (
    HEIGHT,
    WIDTH,
    Classes,
    Classifiers,
    FeatureIndices,
    Objective,
    f2_score,
    get_data_from_hdf5,
    load_sampling_masks,
)


def pretty_dict(dictionary: dict) -> str:
    line_breaks = str(dictionary).replace(",", "\n\t\t-")
    without_braces = line_breaks.replace("{", "").replace("}", "")
    return "\t\t- " + without_braces + "\n"

root = "/home/franziska-sophie/image-segmentation/dataset"

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("path_to_train_data", type=str)
    parser.add_argument("path_to_val_data", type=str)
    parser.add_argument("study_name", type=str)
    parser.add_argument(
        "--continue_from_trial_id", type=int, nargs="+", default=None
    )
    parser.add_argument("--path_to_test_data", type=str, default=None)
    args = parser.parse_args()

    train_data = get_data_from_hdf5(args.path_to_train_data)
    val_data = get_data_from_hdf5(args.path_to_val_data)

    test_data = (
        get_data_from_hdf5(args.path_to_test_data)
        if args.path_to_test_data is not None
        else None
    )

    train_mask, val_mask = load_sampling_masks(os.path.join(root, "masks.npz"))

    with open("log.txt", "a") as file:
        for classifier_name in get_args(Classifiers):
            study = optuna.create_study(
                directions=["maximize", "minimize"],
                storage="sqlite:///db.sqlite3",
                study_name=args.study_name + "_" + classifier_name,
                load_if_exists=True,
            )

            study.enqueue_trial(
                {
                    "color_Y": False,
                    "color_Cr": False,
                    "color_Cb": False,
                    "color_B": False,
                    "color_G": False,
                    "color_R": False,
                    "color_b": False,
                    "color_g": True,
                    "color_r": False,
                    "color_I": False,
                    "color_L*": False,
                    "color_a*": False,
                    "color_b*": False,
                    "color_H": False,
                    "color_S": False,
                    "color_V": False,
                    "selected_channel_index": 0,
                    "texture_method": "NeighborsDifference",
                    "Neighbors_radius": 28,
                    "Neighbors_orientations": 4,
                }
            )

            objective = Objective(
                train_data,
                val_data,
                classifier_name,
                train_mask,
                val_mask,
            )

            if args.continue_from_trial_id is None:
                study.optimize(
                    objective,
                    n_trials=1,
                    n_jobs=1,
                    gc_after_trial=True,
                    catch=(MemoryError,),
                )
                continue

            if test_data is None:
                raise Exception("No test data was provided. Program will exit.")

            if train_mask is not None and val_mask is not None:
                train_mask = np.concatenate([train_mask, val_mask], axis=0)

            now_string = datetime.now().strftime("%Y-%m-%d, %H:%M:%S")
            file.write(
                f"*** Hyperparameter tuning for {classifier_name} ({now_string}) ***\n\n"
            )
            for trial_id in args.continue_from_trial_id:
                best_trial = study.trials[trial_id]
                #             file.write(
                #                 f"- Trial {best_trial.number}\n\
                # \t- F-beta Score: {best_trial.value:.4f}\n\
                # \t- Took: {best_trial.duration}\n"
                #             )
                #             file.write(
                #                 f"\t- Best parameters:\n{pretty_dict(best_trial.params)}"
                #             )

                parameters = best_trial.params

                X_train, y_train = objective.train_data
                X_val, y_val = objective.val_data
                X_test, y_test = test_data

                X_test = X_test.astype(np.uint8)

                y_train = y_train.astype(np.uint8)
                y_val = y_val.astype(np.uint8)
                y_test = y_test.astype(np.uint8)

                indices = [
                    feature.value
                    for feature in FeatureIndices
                    if parameters[f"color_{feature.name}"]
                ]
                X_train, y_train_binary = objective.prepocess_data(
                    X_train, y_train, indices
                )
                X_val, y_val_binary = objective.prepocess_data(
                    X_val, y_val, indices
                )

                X_test, y_test_binary = objective.prepocess_data(
                    X_test, y_test, indices
                )

                selected_color_channel = parameters["selected_channel_index"]
                selected_texture_method = parameters["texture_method"]

                X_train = np.concatenate([X_train, X_val], axis=0)
                y_train = np.concatenate([y_train, y_val], axis=0)
                y_train_binary = np.concatenate(
                    [y_train_binary, y_val_binary], axis=0
                )

                texture_features_train, texture_features_test = (
                    objective.get_texture_features(
                        np.reshape(X_train, (-1, HEIGHT, WIDTH, len(indices))),
                        np.reshape(X_test, (-1, HEIGHT, WIDTH, len(indices))),
                        selected_texture_method,
                        selected_color_channel,
                        params=parameters,
                    )
                )

                X_train = np.concatenate(
                    (X_train, texture_features_train), axis=-1
                )
                X_test = np.concatenate(
                    (X_test, texture_features_test), axis=-1
                )
                X_train_binary = X_train[y_train != Classes.UNKNOWN.value]
                X_test_binary = X_test[y_test != Classes.UNKNOWN.value]

                X_train_binary = X_train_binary[train_mask == 1]
                y_train_binary = y_train_binary[train_mask == 1]

                classifier = objective.get_classifier()
                param_grid = objective.get_param_grid()
                search = GridSearchCV(
                    classifier,
                    param_grid,
                    cv=5,
                    scoring=metrics.make_scorer(f2_score),
                    n_jobs=1,
                    error_score=0,
                    verbose=2,
                )
                model = search.fit(X_train_binary, y_train_binary)
                file.write(f"- Cross Validation for Trial {trial_id}\n")
                file.write(f"\t- Best score: {model.best_score_:.4f}\n")
                params_as_string = pretty_dict(model.best_params_)
                file.write(f"\t- Best hyperparameter(s):\n{params_as_string}")

                best_model = model.best_estimator_
                prediction = best_model.predict(X_test_binary)
                # show_boxplot(best_model, X_train, y_train)
                # show_boxplot(best_model, X_test, y_val)
                final_score = f2_score(y_test_binary, prediction)
                file.write(f"- Final score on test data: {final_score:.4f}\n\n")

                # Note: Best estimater is refittet on the whole dataset
                # Load model with "joblib.load('best_model.joblib')""
                os.makedirs("best_models", exist_ok=True)
                joblib.dump(
                    best_model,
                    f"best_models/best_{classifier_name}_{trial_id}.joblib",
                )
