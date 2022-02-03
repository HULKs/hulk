import click
import json
import tensorflow as tf
import tensorflow_addons as tfa


@click.command()
@click.argument('individual_file', type=click.File('r'))
@click.argument('model_file', type=click.Path())
def main(individual_file, model_file):
    individual_data = json.load(individual_file)
    create_model(
        individual_data['genome'],
        individual_data['configuration'],
        model_file,
    )


def create_model(genome, configuration, model_file):
    metrics = []
    for metric in configuration['metrics']:
        if metric[0] == 'f' and metric[-5:] == 'score':
            metrics.append(
                tfa.metrics.FBetaScore(
                    name=metric,
                    num_classes=2,
                    average='micro',
                    threshold=0.5, #TODO: fscore threshold
                    beta=float(metric[1:-5]),
                )
            )
        else:
            metrics.append(metric)

    model = build_model(genome, configuration)
    model.compile(
        optimizer=genome['optimizer'],
        loss=configuration['loss'],
        metrics=metrics,
    )

    train_dataset = get_dataset(
        tf.data.TFRecordDataset(configuration['train_dataset']),
        configuration['batch_size'],
        configuration['type'],
        configuration['input_size_x'] *
        configuration['input_size_y'],
        configuration['augment_params'],
    )

    model.optimizer.lr.assign(genome['initial_learning_rate'])

    model.fit(
        train_dataset,
        epochs=genome['training_epochs'])

    tf.keras.models.save_model(
        model,
        str(model_file),
        save_format='h5')

    tf.keras.backend.clear_session()


def build_convolution_layers(genome):
    '''Build sequential layer list of convolution layers'''

    layers = []
    for layer in genome['convolution_layers']:
        if layer['type'] == 'SeparableConv2D':
            layers.append(tf.keras.layers.SeparableConv2D(
                filters=layer['filters'],
                kernel_size=layer['kernel_size'],
                strides=layer['stride'],
                padding='same',
                use_bias=False,
            ))
        elif layer['type'] == 'Conv2D':
            layers.append(tf.keras.layers.Conv2D(
                filters=layer['filters'],
                kernel_size=layer['kernel_size'],
                strides=layer['stride'],
                padding='same',
                use_bias=False,
            ))
        else:
            raise NotImplementedError

        if layer['batch_normalization']:
            layers.append(tf.keras.layers.BatchNormalization())

        layers.append(tf.keras.layers.Activation(
            activation=layer['activation_function'],
        ))

        if layer['pooling_type'] is not None:
            if layer['pooling_type'] == 'maximum':
                layers.append(tf.keras.layers.MaxPooling2D(
                    pool_size=layer['pooling_size'],
                ))
            elif layer['pooling_type'] == 'average':
                layers.append(tf.keras.layers.AveragePooling2D(
                    pool_size=layer['pooling_size'],
                ))
            else:
                raise NotImplementedError

        if layer['drop_out_rate'] > 0.01:
            layers.append(tf.keras.layers.Dropout(
                rate=layer['drop_out_rate'],
            ))

    return layers


def build_dense_layers(genome):
    '''Build sequential layer list of dense layers'''

    layers = []
    for layer in genome['dense_layers']:
        layers.append(tf.keras.layers.Dense(
            layer['units'],
            activation=layer['activation_function'],
        ))

        if layer['batch_normalization']:
            layers.append(tf.keras.layers.BatchNormalization())

        if layer['drop_out_rate'] > 0.01:
            layers.append(tf.keras.layers.Dropout(
                rate=layer['drop_out_rate'],
            ))

    return layers


def build_layers(genome, configuration):
    '''Build sequential layer list'''

    # input layer
    layers = [
        tf.keras.Input(
            shape=(configuration['input_size_x'],
                   configuration['input_size_y'],
                   configuration['input_channels']),
        )
    ]

    # convolution layers
    layers += build_convolution_layers(genome)

    # flatten between convolutions and denses
    layers.append(tf.keras.layers.Flatten())

    # dense layers
    layers += build_dense_layers(genome)

    # final layer
    layers.append(tf.keras.layers.Dense(
        configuration['final_layer_neurons'],
        activation=genome['final_layer_activation_function'],
    ))

    if genome['final_layer_batch_normalization']:
        layers.append(tf.keras.layers.BatchNormalization())

    return layers


def build_model(genome, configuration):
    '''Build sequential model'''

    return tf.keras.Sequential(
        layers=build_layers(genome, configuration),
    )


def shape_and_augment_sample(data, shape, augment_params):
    image = tf.reshape(tf.cast(data, tf.float32), shape)
    image = tf.image.random_brightness(
        image,
        augment_params['random_brightness_delta'],
        seed=augment_params['random_brightness_seed'])
    image = tf.clip_by_value(image, 0.0, 255.0, name=None)
    return(image)


def parse_tfrecord_class(data_size, augment_params, example):
    parsed = tf.io.parse_single_example(example, features={
        'data': tf.io.FixedLenFeature([data_size], tf.int64),
        'dataShape': tf.io.FixedLenFeature([3], tf.int64),
        'isPositive': tf.io.FixedLenFeature([1], tf.int64),
        'circle': tf.io.FixedLenFeature([3], tf.float32)
    })
    return(shape_and_augment_sample(parsed['data'], parsed['dataShape'], augment_params),
           tf.cast(parsed['isPositive'], tf.float32))


def parse_tfrecord_circle(data_size, augment_params, example):
    parsed = tf.io.parse_single_example(example, features={
        'data': tf.io.FixedLenFeature([data_size], tf.int64),
        'dataShape': tf.io.FixedLenFeature([3], tf.int64),
        'isPositive': tf.io.FixedLenFeature([1], tf.int64),
        'circle': tf.io.FixedLenFeature([3], tf.float32)
    })
    return(shape_and_augment_sample(parsed['data'], parsed['dataShape'], augment_params),
           tf.math.multiply(parsed['circle'], tf.constant([1.0/32.0, 1.0/32.0, 1.0/16.0])))


def parse_tfrecord_verify(data_size, example):
    parsed = tf.io.parse_single_example(example, features={
        'data': tf.io.FixedLenFeature([data_size], tf.int64),
        'dataShape': tf.io.FixedLenFeature([3], tf.int64),
        'isPositive': tf.io.FixedLenFeature([1], tf.int64),
        'circle': tf.io.FixedLenFeature([3], tf.float32)
    })
    return tf.reshape(tf.cast(parsed['data'], tf.float32), parsed['dataShape'])


def get_dataset(tfr_ds, batch_size, nnType, data_size, augment_params):
    if (nnType == 'positioner'):
        tfr_ds = tfr_ds.map(lambda x: parse_tfrecord_circle(
            data_size, augment_params, x))
        tfr_ds = tfr_ds.batch(batch_size)
        tfr_ds = tfr_ds.prefetch(batch_size)
        return tfr_ds
    elif (nnType == 'verify'):
        tfr_ds = tfr_ds.map(lambda x: parse_tfrecord_verify(data_size, x))
        return tfr_ds
    else:
        tfr_ds = tfr_ds.map(lambda x: parse_tfrecord_class(
            data_size, augment_params, x))
        tfr_ds = tfr_ds.batch(batch_size)
        tfr_ds = tfr_ds.prefetch(batch_size)
        return tfr_ds

if __name__ == '__main__':
    main()
