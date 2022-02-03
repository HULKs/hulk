#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import argparse
import curses
import json
import random
from numpy import array
from numpy import concatenate
from pathlib import Path
import evolvercurse as ui
import nn_utils
import evolver_utils

ball_detection_path = Path(__file__).resolve().parent.parent
data_path = ball_detection_path / "data" / "evolver"

nn_evolver_params = None

available_settings = {
    "preclassifier": {
        "type": "preclassifier",
        "final_layer_neurons": 1,
        "loss": "binary_crossentropy",
        "input_size_x": 32,
        "input_size_y": 32,
        "input_channels": 1,
        "metric": "f2",
        "cost_factor": 0.0000005,
        "conv_activation_functions": ["elu", # good
                                    "hard_sigmoid",
                                    "linear", # good
                                    "relu",  # good
                                    "selu",
                                    "sigmoid",
                                    #"softmax", not supported for conv layers by CompiledNN
                                    #"softplus", not supported by CompiledNN
                                    #"softsign", #suspicious
                                    "tanh"], #approx
        "dense_activation_functions": ["elu", # good
                                    "hard_sigmoid",
                                    "linear",
                                    "relu",
                                    "selu", # good
                                    "sigmoid", # good
                                    #"softmax", #suspicious
                                    #"softplus", #not supported by CompiledNN
                                    #"softsign", #suspicious
                                    "tanh"], #approx
        "final_activation_functions": ["elu", # good
                                    "hard_sigmoid",
                                    "linear",
                                    "relu",
                                    "selu", # good
                                    "sigmoid", # good
                                    "softmax", #suspicious
                                    # "softplus", not supported by CompiledNN
                                    "softsign", #suspicious
                                    "tanh"], #approx
        "batch_size": 32,
        "batches_per_training_epoch": 1000,
        "batches_for_scoring_test": 500,
        "optimizers": ["Adadelta",
                    "Adam",
                    "Adamax",
                    "Nadam",
                    "RMSprop",
                    "SGD"]},

    "classifier": {
        "type": "classifier",
        "final_layer_neurons": 1,
        "loss": "binary_crossentropy",
        "input_size_x": 32,
        "input_size_y": 32,
        "input_channels": 1,
        "metric": "f1",
        "cost_factor": 0.00000005,
        "conv_activation_functions": ["elu", # good
                                    "hard_sigmoid",
                                    "linear", # good
                                    "relu",  # good
                                    "selu",
                                    "sigmoid",
                                    #"softmax", not supported for conv layers by CompiledNN
                                    #"softplus", not supported by CompiledNN
                                    #"softsign", #suspicious
                                    "tanh"], #approx
        "dense_activation_functions": ["elu", # good
                                    "hard_sigmoid",
                                    "linear",
                                    "relu",
                                    "selu", # good
                                    "sigmoid", # good
                                    #"softmax", #suspicious
                                    #"softplus", #not supported by CompiledNN
                                    #"softsign", #suspicious
                                    "tanh"], #approx
        "final_activation_functions": [#"elu", # good
                                    "hard_sigmoid",
                                    #"linear",
                                    #"relu",
                                    #"selu", # good
                                    "sigmoid", # good
                                    "softmax", #suspicious
                                    # "softplus", not supported by CompiledNN
                                    "softsign", #suspicious
                                    "tanh"], #approx
        "batch_size": 32,
        "batches_per_training_epoch": 1000,
        "batches_for_scoring_test": 500,
        "optimizers": ["Adadelta",
                    "Adam",
                    "Adamax",
                    "Nadam",
                    "RMSprop",
                    "SGD"]},

    "positioner": {
        "type": "positioner",
        "final_layer_neurons": 3,
        "loss": "mean_squared_error",
        "input_size_x": 32,
        "input_size_y": 32,
        "input_channels": 1,
        "metric": "accuracy",
        "cost_factor": 0.000000005,
        "conv_activation_functions": ["elu", # good
                                    "hard_sigmoid",
                                    "linear", # good
                                    "relu",  # good
                                    "selu",
                                    "sigmoid",
                                    #"softmax", not supported for conv layers by CompiledNN
                                    #"softplus", not supported by CompiledNN
                                    #"softsign", #suspicious
                                    "tanh"], #approx
        "dense_activation_functions": ["elu", # good
                                    "hard_sigmoid",
                                    "linear",
                                    "relu",
                                    "selu", # good
                                    "sigmoid", # good
                                    #"softmax", #suspicious
                                    #"softplus", #not supported by CompiledNN
                                    #"softsign", #suspicious
                                    "tanh"], #approx
        "final_activation_functions": ["elu", # good
                                    #"hard_sigmoid",
                                    "linear",
                                    "relu",
                                    "selu"], # good
                                    #"sigmoid", # good
                                    # "softmax", #suspicious
                                    # "softplus", not supported by CompiledNN
                                    # "softsign", #suspicious
                                    #"tanh"], #approx
        "batch_size": 32,
        "batches_per_training_epoch": 1000,
        "batches_for_scoring_test": 500,
        "optimizers": ["Adadelta",
                        "Adam",
                        "Adamax",
                        "Nadam",
                        "RMSprop",
                        "SGD"]}
}

pop_pad = None
status_pad = None
member_pad = None
spam_pad = None
progression_pad = None


def parse_arguments():
    arg_parser = argparse.ArgumentParser(description = "Neural Net Evolver",
                                         formatter_class = argparse.ArgumentDefaultsHelpFormatter)
    arg_parser.add_argument('-x', '--scratch',
                            dest = 'scratch_population',
                            default = False,
                            help = "start with a new random population, instead of loading the current saved population",
                            action = 'store_true')
    arg_parser.add_argument('-d', '--debug',
                            dest = 'prevent_curses_output',
                            default = False,
                            help = "don't use curses output",
                            action='store_true')
    arg_parser.add_argument('-m', '--mode',
                            dest = 'mode',
                            default = "classifier",
                            choices = ["preclassifier", "classifier", "positioner"],
                            help = "train preclassifier, classifier or positioner",
                            action='store')
    arg_parser.add_argument('-r', '--train-dataset',
                            dest = 'train_dataset',
                            default = str(data_path / "classifier_train.tfrecord"),
                            help = "TFRecord file for training neural nets",
                            action='store')
    arg_parser.add_argument('-t', '--test-dataset',
                            dest = 'test_dataset',
                            default = str(data_path / "classifier_test.tfrecord"),
                            help = "TFRecord file for testing neural nets",
                            action='store')
    arg_parser.add_argument('-c', '--cnn-verify-dataset',
                            dest = 'cnn_verify_dataset',
                            default = str(data_path / "cnn-check-dataset.tfrecord"),
                            help = "TFRecord file for testing CompiledNN compatibility",
                            action='store')
    arg_parser.add_argument('-p', '--predicter',
                            dest = 'predicter',
                            default = str(ball_detection_path / "predicter" / "build" / "predicter"),
                            help = "Path to predicter executable",
                            action='store')
    arg_parser.add_argument('-e', '--predicter-error-threshold',
                            dest = 'predicter_error_threshold',
                            default = 0.001,
                            type = float,
                            help = "Threshold for compiledNN check",
                            action='store')
    return arg_parser.parse_args()


def main():
    settings = available_settings[nn_evolver_params.mode]

    population_path = data_path / (settings["type"] + "_population/")
    trained_model_path = data_path / ("trained_" + settings["type"] + "_models/")

    pop = evolver_utils.Population(
                     nn_settings = settings,
                     trained_model_path = trained_model_path,
                     population_path = population_path,
                     member_pad = member_pad,
                     spam_pad = spam_pad,
                     progression_pad = progression_pad,
                     pop_pad = pop_pad,
                     status_pad = status_pad,
                     conv_activation_functions = settings["conv_activation_functions"],
                     dense_activation_functions = settings["dense_activation_functions"],
                     final_activation_functions = settings["final_activation_functions"],
                     optimizers = settings["optimizers"],
                     batch_size = settings["batch_size"],
                     tfr_train = nn_evolver_params.train_dataset,
                     tfr_test = nn_evolver_params.test_dataset,
                     tfr_verify = nn_evolver_params.cnn_verify_dataset,
                     predicter = nn_evolver_params.predicter,
                     predicter_error_threshold = nn_evolver_params.predicter_error_threshold
                     )
    if not nn_evolver_params.scratch_population:
        #load saved population
        for filename in population_path.iterdir():
            if filename.suffix == ".json":
                pop_pad.print(pop)
                try:
                    status_pad.print("loading " + str(filename.name))
                    f = open(filename, 'r')
                    data = json.load(f)
                    f.close()
                    new_individual = evolver_utils.Individual(name=filename.name[0:-5],
                                                              genes=data)
                    pop.add_member(new_individual, save_files=True)
                except Exception as e:
                    status_pad.print(str(e))
    running = True
    #start the evolutionary optimization
    while running:
        pop_pad.print(pop)
        pop.evolve()


def curses_main(stdscr):
    # initialize curses
    global pop_pad, status_pad, member_pad, spam_pad, progression_pad
    popmax = 25
    pop_pad = ui.PopulationPad(x = 4 + popmax,  to_line = 14 + popmax, stdscr = stdscr)
    status_pad = ui.StatusPad(stdscr = stdscr)
    member_pad = ui.MemberPad(stdscr = stdscr)
    spam_pad = ui.SpamPad(stdscr = stdscr)
    progression_pad = ui.ProgressionPad(from_line = 8 + popmax,  to_line = 38 + popmax, stdscr = stdscr)
    stdscr.clear()
    main()


if __name__== "__main__":
    nn_evolver_params = parse_arguments()

    if not nn_evolver_params.prevent_curses_output:
        curses.wrapper(curses_main)
    else:
        pop_pad = ui.PopulationPad(use_curses = False)
        status_pad = ui.StatusPad(use_curses = False)
        member_pad = ui.MemberPad(use_curses = False)
        spam_pad = ui.SpamPad(use_curses = False)
        progression_pad = ui.ProgressionPad(use_curses = False)
        main()
