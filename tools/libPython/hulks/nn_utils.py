import subprocess
import os
import time
import datetime
import json
import tensorflow as tf
import math


hulks_path = os.path.dirname(os.path.realpath(__file__)) + "/"


def f1(y_true, y_pred):
    return f_beta(1, y_true, y_pred)


def f2(y_true, y_pred):
    return f_beta(2, y_true, y_pred)


def f_beta(beta, y_true, y_pred):
    K = tf.keras.backend
    true_positives = K.sum(K.round(K.clip(y_true * y_pred, 0, 1)))
    possible_positives = K.sum(K.round(K.clip(y_true, 0, 1)))
    predicted_positives = K.sum(K.round(K.clip(y_pred, 0, 1)))
    precision = true_positives / (predicted_positives + K.epsilon())
    recall = true_positives / (possible_positives + K.epsilon())
    beta_sq = beta*beta
    return (1 + beta_sq) * ((precision*recall)/((beta_sq * precision) +recall + K.epsilon()))


def compiledNN_distance(model, model_path, samples):
    tf2_keras_results = model.predict(samples)
    samples = samples.reshape((10, 32*32))
    samples = "\n".join([str([float(v) for v in s]) for s in samples])
    proc = subprocess.Popen([hulks_path + "Predicter", model_path], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    output, errors = proc.communicate(input=str.encode(samples))
    compiledNN_output = output.decode().split("\n")
    compiledNN_output = [s.strip("[]").split(",")  for s in compiledNN_output]
    distance = 0.0
    for i in range(len(tf2_keras_results)):
        for j in range(len(tf2_keras_results[i])):
            try:
                distance += abs(float(tf2_keras_results[i][j]) - float(compiledNN_output[i][j]))
            except Exception as e:
                print(e)
                return 100.0
    return distance


def conv_layer_list(genes, input_shape):
    result = []
    is_first_clayer = True
    for (t, n, k, a, m, bn, dr, s) in genes["conv_layers"]:
        if is_first_clayer:
            if t == "Conv2D":
                result.append(tf.keras.layers.Conv2D(n, kernel_size = (k, k), padding = 'same', strides = s, use_bias = False, input_shape = input_shape))
            elif t == "SeparableConv2D":
                result.append(tf.keras.layers.SeparableConv2D(n, (k, k), padding = 'same', strides = s,  use_bias = False, input_shape = input_shape))
        else:
            if t == "Conv2D":
                result.append(tf.keras.layers.Conv2D(n, kernel_size = (k, k), padding = 'same', strides = s, use_bias = False))
            elif t == "SeparableConv2D":
                result.append(tf.keras.layers.SeparableConv2D(n, (k, k), padding = 'same', strides = s, use_bias = False))
        if is_first_clayer:
            is_first_clayer = False

        if bn:
            result.append(tf.keras.layers.BatchNormalization())
        result.append(tf.keras.layers.Activation(a))
        if m != 0:
            if m > 0:
                result.append(tf.keras.layers.MaxPooling2D(pool_size=(m, m)))
            else:
                result.append(tf.keras.layers.AveragePooling2D(pool_size=(-m, -m)))
        if dr > 0.01:
            result.append(tf.keras.layers.Dropout(dr))
    result.append(tf.keras.layers.Flatten())
    return result


def dense_layer_list(genes, inputs, final_layer_neurons):
    result = []
    is_first_clayer = True
    for (n,a, bn, dr) in genes["dense_layers"]:
        if is_first_clayer:
            result.append(tf.keras.layers.Dense(n, input_shape = [inputs]))
            is_first_clayer = False
        else:
            result.append(tf.keras.layers.Dense(n))
        if bn:
            result.append(tf.keras.layers.BatchNormalization())
        result.append(tf.keras.layers.Activation(a))
        if dr > 0.01:
            result.append(tf.keras.layers.Dropout(dr))
    (a, bn) = genes["final_layer"]
    result.append(tf.keras.layers.Dense(final_layer_neurons))
    if bn:
        result.append(tf.keras.layers.BatchNormalization())
    result.append(tf.keras.layers.Activation(a))
    return result


def add_to_log(line_type, dict_line, log_file):
    with open(hulks_path + log_file, "a") as evolver_log_file:
        timestamp = datetime.datetime.fromtimestamp(time.time()).strftime('%d/%m/%y-%H:%M:%S')
        evolver_log_file.write("{\"t\": \"" + timestamp + 
                               "\", \"type\": \"" + line_type + "\", " + 
                               json.dumps(dict_line)[1:] + "\n")


def cost(genes, final_layer_neurons, cost_factor):
    df = 32
    result = 0
    m = 1
    for i in range(len(genes["conv_layers"])):
        (t, n, dk, a, maxp, b, dr, s) = genes["conv_layers"][i]
        stride = 1
        if maxp == 0:
            stride = 2
        if t == "Conv2D":
            result += (m*n*dk*dk*df*df) / (s*s)
            m = n
        else:
            result += ((m*dk*dk*df*df) + (m*n*df*df)) / (s*s)
            m = n
        resize_denominator = s
        if maxp != 0:
            result += m * maxp * maxp * df * df
            resize_denominator *= abs(maxp)
        df /= resize_denominator
    previous_layer_neurons = m*df*df
    for (n, a, b, dr) in genes["dense_layers"]:
        result += previous_layer_neurons * n
        previous_layer_neurons = n
    result += previous_layer_neurons * final_layer_neurons
    return (result * cost_factor) #0.00000001
