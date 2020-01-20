import enum
import uuid
import typing as ty


class TabType(enum.Enum):
    MAP = 0
    LAYER = 1
    CONFIG = 2


def create_layer(layer_type: str) -> ty.Dict:
    """
    Helper function to create a new empty Layer. A layer has 'name', 'identifier', 'type', 'config', 'enabled' keys.
    The function creates a dict containing all this elements forming the layer_model.
    :param layer_type: String representing the type
    :return: A composed dict representing the model
    """
    return {
        "name": layer_type,
        "identifier": str(uuid.uuid4()),
        "type": layer_type,
        "config": None,
        "enabled": True
    }


def swap_layer(layer: ty.List, index_a: int, index_b: int) -> None:
    """
    Helper function to swap two layers in the layer list. The layers to be swapped are identified by their ids.
    :param layer: The list with all layers
    :param index_a: index of the first layer
    :param index_b: index of the second layer
    """
    layer[index_a], layer[index_b] = layer[index_b], layer[index_a]
