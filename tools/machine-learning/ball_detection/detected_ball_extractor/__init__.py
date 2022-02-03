import json
import os
import click


@click.command()
@click.argument('replay_directory_or_file', type=click.Path(exists=True))
@click.argument('output_annotations_file', type=click.File(mode='w'))
def main(**config):
    annotations = {}
    for root, _, files in os.walk(config['replay_directory_or_file']):
        for file in files:
            if file == 'replay.json':
                with open(os.path.join(root, file)) as f:
                    parsed_replay = json.load(f)

                print(os.path.join(root, file), '...')

                for frame in parsed_replay['frames']:
                    image_path = frame['topImage'] if 'topImage' in frame else frame['bottomImage']
                    rooted_image_path = os.path.join(root, image_path)

                    annotations[rooted_image_path] = []
                    for cluster in frame['ballDetectionData']['clusters']:
                        annotations[rooted_image_path].append({
                            'centerX': cluster['mergedCircle'][0][0],
                            'centerY': cluster['mergedCircle'][0][1],
                            'radius': cluster['mergedCircle'][1],
                        })

    json.dump(annotations, config['output_annotations_file'], sort_keys=True, indent=4)
    config['output_annotations_file'].write('\n')
