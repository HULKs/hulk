import numpy as np

from transforms import (
    isometry_from_euler,
    isometry_from_translation,
    translation_from_isometry,
    rotation_from_isometry,
)


def test_rotation_from_euler() -> None:
    vector = np.array([1.0, 0.0, 0.0, 0.0])
    rotation = isometry_from_euler(0.0, 0.0, np.pi / 2)

    np.testing.assert_allclose(
        rotation @ vector, np.array([0.0, 1.0, 0.0, 0.0]), atol=1e-11
    )


def test_translation_from_vector() -> None:
    translation = isometry_from_translation(np.array([1.0, -1.0, 1.0]))

    vector = np.array([1.0, 2.0, 3.0, 0.0])
    np.testing.assert_allclose(
        translation @ vector, np.array([1.0, 2.0, 3.0, 0.0]), atol=1e-11
    )

    vector = np.array([1.0, 2.0, 3.0, 1.0])
    np.testing.assert_allclose(
        translation @ vector, np.array([2.0, 1.0, 4.0, 1.0]), atol=1e-11
    )


def test_isometry_from_parts() -> None:
    position = isometry_from_translation(np.array([1.0, -1.0, 1.0]))
    rotation = isometry_from_euler(0.0, 0.0, np.pi / 2)

    print(position)
    pose = position @ rotation

    np.testing.assert_allclose(
        pose @ np.array([1.0, 0.0, 0.0, 0.0]),
        np.array([0.0, 1.0, 0.0, 0.0]),
        atol=1e-11,
    )

    translation = isometry_from_translation(np.array([1.0, 2.0, 3.0]))
    rotation = isometry_from_euler(0.0, 0.0, np.pi / 2)
    transform = translation @ rotation

    result = transform @ pose

    result_position = translation_from_isometry(result)
    result_rotation = rotation_from_isometry(result)

    np.testing.assert_allclose(
        result_position,
        np.array([2.0, 3.0, 4.0]),
        atol=1e-11,
    )
    np.testing.assert_allclose(
        result_rotation,
        rotation_from_isometry(isometry_from_euler(0.0, 0.0, np.pi)),
        atol=1e-11,
    )
