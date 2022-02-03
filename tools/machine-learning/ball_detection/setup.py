import setuptools

setuptools.setup(
    name='ball_detection',
    version='0.0.1',
    packages=[
        'adder',
        'annotation_rater',
        'annotation_sampler',
        'hdf5_extractor',
        'corrector',
        'detected_ball_extractor',
        'image',
        'random_negative_sampler',
        'remover',
        'tfrecord',
        'viewer',
    ],
    include_package_data=True,
    entry_points={
        'console_scripts': [
            'adder = adder:server',
            'annotation-rater = annotation_rater:main',
            'annotation-sampler = annotation_sampler:main',
            'hdf5-extractor = hdf5_extractor:main',
            'corrector = corrector:server',
            'detected-ball-extractor = detected_ball_extractor:main',
            'random-negative-sampler = random_negative_sampler:main',
            'remover = remover:server',
            'viewer = viewer:server',
        ],
    },
    install_requires=[
        'click>=7.1.2',
        'crc32c>=2.0.1',
        'Flask>=1.1.2',
        'h5py>=2.10.0',
        'numpy>=1.19.1',
        'opencv-python>=4.4.0.42',
        'protobuf>=3.13.0',
    ],
)
