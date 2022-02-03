import json
import os
import click
import cv2
import h5py


@click.command(help='This tool extracts images and annotations from a HDF5 input file.\n\n\'b-alls-2019\': Creates b-alls-2019.json and a directory structure in the current working directory containing extracted labeled samples of b-alls-2019 dataset: raw/positives and raw/negatives\n\n\'dataset-v1\': TODO')
@click.option('--mode', type=click.Choice(['b-alls-2019', 'dataset-v1']), help='HDF5 interpretation mode', required=True)
@click.argument('input_hdf5_file', type=click.File(mode='rb', lazy=False))
def main(**config):
    with h5py.File(config['input_hdf5_file'], mode='r') as hdf5:
        if config['mode'] == 'b-alls-2019':
            annotations = {}
            for kind in ['positives', 'negatives']:
                kind_directory = os.path.join('raw', kind)
                os.makedirs(kind_directory, exist_ok=True)
                with click.progressbar(enumerate(zip(hdf5[kind]['data'], hdf5[kind]['labels'])), length=len(hdf5[kind]['data']), label=f'{config["input_hdf5_file"].name} ({kind})') as examples:
                    for i, (data, labels) in examples:
                        filename = f'{i:06d}.png'
                        cv2.imwrite(os.path.join(
                            kind_directory, filename), data)
                        annotations[os.path.join(kind_directory, filename)] = [{
                            'centerX': float(labels[1]),
                            'centerY': float(labels[2]),
                            'radius': float(labels[3]),
                        }] if labels[0] == 1 else []
            with open('b-alls-2019.json', 'w') as f:
                json.dump(annotations, f, sort_keys=True, indent=4)
                f.write('\n')
        elif config['mode'] == 'dataset-v1':
            annotations = {}
            for event in hdf5.keys():
                event_directory = os.path.join('raw', event)
                os.makedirs(event_directory, exist_ok=True)
                with click.progressbar(enumerate(zip(hdf5[event]['images'], hdf5[event]['labels'])), length=len(hdf5[event]['images']), label=f'{config["input_hdf5_file"].name} ({event})') as examples:
                    for i, (image, labels) in examples:
                        filename = f'{i:06d}.png'
                        image_shape = cv2.imdecode(image, cv2.IMREAD_UNCHANGED).shape
                        with open(os.path.join(event_directory, filename), 'wb') as f:
                            f.write(image.tobytes())
                        annotations[os.path.join(event_directory, filename)] = [{
                            'centerX': float(labels[1] * image_shape[1]),
                            'centerY': float(labels[2] * image_shape[0]),
                            'radius': float(labels[3] * image_shape[1]),
                        }] if labels[0] >= 0.5 else []
            with open('dataset-v1.json', 'w') as f:
                json.dump(annotations, f, sort_keys=True, indent=4)
                f.write('\n')
        else:
            raise RuntimeError(f'Mode {config["mode"]} not implemented.')
